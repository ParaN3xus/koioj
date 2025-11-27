use crate::AppState;
use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use super::{ContestInfo, SubmissionResult};

#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContestRankingItem {
    pub user_id: i32,
    pub username: String,
    pub solved_count: i32,
    pub total_penalty: i64,
    pub problem_results: Vec<ProblemResult>,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProblemResult {
    pub problem_id: i32,
    pub accepted: bool,
    pub attempts: i32,
    pub accepted_time: Option<DateTime<Utc>>,
}

/// Redis key generators
fn ranking_key(contest_id: i32) -> String {
    format!("contest:{}:ranking", contest_id)
}

fn user_key(contest_id: i32, user_id: i32) -> String {
    format!("contest:{}:user:{}", contest_id, user_id)
}

fn version_key(contest_id: i32) -> String {
    format!("contest:{}:ranking:version", contest_id)
}

/// Calculate score for sorted set
fn calculate_score(solved_count: i32, total_penalty: i64) -> i64 {
    solved_count as i64 * 9999999 - total_penalty
}

/// Get ranking from Redis cache
pub async fn get_contest_ranking_cached(
    state: &Arc<AppState>,
    contest: &ContestInfo,
) -> Result<Vec<ContestRankingItem>> {
    let mut redis_conn = state.redis.clone();

    // Check if cache exists
    let exists: bool = redis_conn
        .exists(&ranking_key(contest.id))
        .await
        .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

    if !exists {
        tracing::info!("Cache miss for contest {}, rebuilding", contest.id);
        return rebuild_ranking_cache(state, contest).await;
    }

    // Get sorted user ids
    let user_ids: Vec<String> = redis_conn
        .zrange(&ranking_key(contest.id), 0, -1)
        .await
        .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

    if user_ids.is_empty() {
        return Ok(vec![]);
    }

    // Get problem list
    let problem_ids = get_contest_problems(&state.pool, contest.id).await?;

    // Batch get user data
    let mut rankings = Vec::new();
    for user_id_str in user_ids {
        let user_id: i32 = user_id_str
            .parse()
            .map_err(|e| Error::msg(format!("invalid user_id in redis: {}", e)))?;

        let user_data: std::collections::HashMap<String, String> = redis_conn
            .hgetall(&user_key(contest.id, user_id))
            .await
            .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

        if user_data.is_empty() {
            tracing::warn!(
                "User data missing for user {} in contest {}",
                user_id,
                contest.id
            );
            continue;
        }

        let username = user_data.get("username").cloned().unwrap_or_default();
        let solved_count: i32 = user_data
            .get("solved_count")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let total_penalty: i64 = user_data
            .get("total_penalty")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let mut problem_results = Vec::new();
        for problem_id in &problem_ids {
            let accepted = user_data
                .get(&format!("problem:{}:accepted", problem_id))
                .and_then(|s| s.parse().ok())
                .unwrap_or(false);
            let attempts = user_data
                .get(&format!("problem:{}:attempts", problem_id))
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let accepted_time = user_data
                .get(&format!("problem:{}:accepted_time", problem_id))
                .and_then(|s| s.parse::<i64>().ok())
                .and_then(|ts| DateTime::from_timestamp(ts, 0));

            problem_results.push(ProblemResult {
                problem_id: *problem_id,
                accepted,
                attempts,
                accepted_time,
            });
        }

        rankings.push(ContestRankingItem {
            user_id: user_id,
            username,
            solved_count,
            total_penalty,
            problem_results,
        });
    }

    // Refresh TTL
    let ttl = calculate_ttl(contest);
    let _: () = redis_conn
        .expire(&ranking_key(contest.id), ttl)
        .await
        .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

    Ok(rankings)
}

/// Rebuild ranking cache from database
pub async fn rebuild_ranking_cache(
    state: &Arc<AppState>,
    contest: &ContestInfo,
) -> Result<Vec<ContestRankingItem>> {
    let rankings = calculate_contest_ranking_from_db(&state.pool, contest).await?;

    let mut redis_conn = state.redis.clone();

    // Clear old data
    let _: () = redis_conn
        .del(&ranking_key(contest.id))
        .await
        .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

    // Write to Redis
    for item in &rankings {
        let user_id: i32 = item.user_id;

        // Add to sorted set
        let score = calculate_score(item.solved_count, item.total_penalty);
        let _: () = redis_conn
            .zadd(&ranking_key(contest.id), &item.user_id, score)
            .await
            .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

        // Store user data
        let mut fields = vec![
            ("username".to_string(), item.username.clone()),
            ("solved_count".to_string(), item.solved_count.to_string()),
            ("total_penalty".to_string(), item.total_penalty.to_string()),
        ];

        for pr in &item.problem_results {
            fields.push((
                format!("problem:{}:accepted", pr.problem_id),
                pr.accepted.to_string(),
            ));
            fields.push((
                format!("problem:{}:attempts", pr.problem_id),
                pr.attempts.to_string(),
            ));
            if let Some(time) = pr.accepted_time {
                fields.push((
                    format!("problem:{}:accepted_time", pr.problem_id),
                    time.timestamp().to_string(),
                ));
            }
        }

        let _: () = redis_conn
            .hset_multiple(&user_key(contest.id, user_id), &fields)
            .await
            .map_err(|e| Error::msg(format!("redis error: {}", e)))?;
    }

    // Set TTL
    let ttl = calculate_ttl(contest);
    let _: () = redis_conn
        .expire(&ranking_key(contest.id), ttl)
        .await
        .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

    // Set version
    let _: () = redis_conn
        .set_ex(&version_key(contest.id), Utc::now().timestamp(), ttl as u64)
        .await
        .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

    Ok(rankings)
}

/// Update ranking cache when a submission is judged
pub async fn update_ranking_on_submission(
    state: &Arc<AppState>,
    contest_id: i32,
    user_id: i32,
    problem_id: i32,
    result: SubmissionResult,
    created_at: DateTime<Utc>,
) -> Result<()> {
    let mut redis_conn = state.redis.clone();

    // Check if cache exists
    let exists: bool = redis_conn
        .exists(&ranking_key(contest_id))
        .await
        .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

    if !exists {
        tracing::debug!(
            "Cache doesn't exist for contest {}, skipping update",
            contest_id
        );
        return Ok(());
    }

    // Get contest info
    let contest = sqlx::query_as!(
        ContestInfo,
        "SELECT id, begin_time, end_time FROM contests WHERE id = $1",
        contest_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    let user_key = user_key(contest_id, user_id);
    let problem_key_prefix = format!("problem:{}:", problem_id);

    // Get current problem state
    let accepted: bool = redis_conn
        .hget(&user_key, format!("{}accepted", problem_key_prefix))
        .await
        .unwrap_or(false);

    if accepted {
        // Already solved, no need to update
        return Ok(());
    }

    // Increment attempts
    let _: () = redis_conn
        .hincr(&user_key, format!("{}attempts", problem_key_prefix), 1)
        .await
        .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

    // If accepted, update ranking
    if result == SubmissionResult::Accepted {
        let attempts: i32 = redis_conn
            .hget(&user_key, format!("{}attempts", problem_key_prefix))
            .await
            .unwrap_or(1);

        // Mark as accepted
        let _: () = redis_conn
            .hset(&user_key, format!("{}accepted", problem_key_prefix), "true")
            .await
            .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

        // Store accepted time
        let _: () = redis_conn
            .hset(
                &user_key,
                format!("{}accepted_time", problem_key_prefix),
                created_at.timestamp(),
            )
            .await
            .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

        // Calculate penalty
        let solve_time = (created_at - contest.begin_time).num_seconds();
        let penalty = solve_time + (attempts - 1) as i64 * 20 * 60;

        // Update solved count and total penalty
        let _: () = redis_conn
            .hincr(&user_key, "solved_count", 1)
            .await
            .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

        let _: () = redis_conn
            .hincr(&user_key, "total_penalty", penalty)
            .await
            .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

        // Get updated values
        let solved_count: i32 = redis_conn
            .hget(&user_key, "solved_count")
            .await
            .unwrap_or(0);

        let total_penalty: i64 = redis_conn
            .hget(&user_key, "total_penalty")
            .await
            .unwrap_or(0);

        // Update sorted set score
        let score = calculate_score(solved_count, total_penalty);
        let _: () = redis_conn
            .zadd(&ranking_key(contest_id), user_id.to_string(), score)
            .await
            .map_err(|e| Error::msg(format!("redis error: {}", e)))?;

        tracing::info!(
            "Updated ranking for user {} in contest {}: solved={}, penalty={}",
            user_id,
            contest_id,
            solved_count,
            total_penalty
        );
    }

    Ok(())
}

/// Calculate TTL based on contest state
fn calculate_ttl(contest: &ContestInfo) -> i64 {
    let now = Utc::now();
    if now < contest.end_time {
        // Contest ongoing: expire 1 hour after end
        (contest.end_time - now).num_seconds() + 3600
    } else {
        // Contest ended: keep for 7 days
        7 * 24 * 3600
    }
}

/// Get contest problems
async fn get_contest_problems(pool: &sqlx::PgPool, contest_id: i32) -> Result<Vec<i32>> {
    sqlx::query_scalar!(
        "SELECT problem_id FROM contest_problems WHERE contest_id = $1 ORDER BY problem_id",
        contest_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))
}

/// Calculate ranking from database (original logic)
async fn calculate_contest_ranking_from_db(
    pool: &sqlx::PgPool,
    contest: &ContestInfo,
) -> Result<Vec<ContestRankingItem>> {
    let problem_ids = get_contest_problems(pool, contest.id).await?;

    let submissions = sqlx::query!(
        r#"
        SELECT s.user_id, s.problem_id, s.result as "result: SubmissionResult", s.created_at,
               u.username
        FROM submissions s
        JOIN users u ON s.user_id = u.id
        WHERE s.problem_id = ANY($1) AND s.contest_id = $2
        ORDER BY s.user_id, s.problem_id, s.created_at
        "#,
        &problem_ids,
        &contest.id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    let mut user_map: std::collections::HashMap<i32, ContestRankingItem> =
        std::collections::HashMap::new();

    for sub in submissions {
        let entry = user_map
            .entry(sub.user_id)
            .or_insert_with(|| ContestRankingItem {
                user_id: sub.user_id,
                username: sub.username.clone(),
                solved_count: 0,
                total_penalty: 0,
                problem_results: problem_ids
                    .iter()
                    .map(|&pid| ProblemResult {
                        problem_id: pid,
                        accepted: false,
                        attempts: 0,
                        accepted_time: None,
                    })
                    .collect(),
            });

        let problem_result = entry
            .problem_results
            .iter_mut()
            .find(|pr| pr.problem_id == sub.problem_id)
            .unwrap();

        if problem_result.accepted {
            continue; // Already solved
        }

        problem_result.attempts += 1;

        if sub.result == SubmissionResult::Accepted {
            problem_result.accepted = true;
            let solve_time = (sub.created_at - contest.begin_time).num_seconds();
            problem_result.accepted_time = Some(sub.created_at);

            // Penalty: solve time + 20 minutes per wrong attempt
            let penalty = solve_time + (problem_result.attempts - 1) as i64 * 20 * 60;
            entry.total_penalty += penalty;
            entry.solved_count += 1;
        }
    }

    let mut rankings: Vec<ContestRankingItem> = user_map.into_values().collect();

    // Sort by solved_count (desc), then by total_penalty (asc)
    rankings.sort_by(|a, b| {
        b.solved_count
            .cmp(&a.solved_count)
            .then_with(|| a.total_penalty.cmp(&b.total_penalty))
    });

    Ok(rankings)
}
