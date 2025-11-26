use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    middleware,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use theoj_common::bail;
use utoipa::ToSchema;

use crate::{
    AppState, Result, State,
    auth::{Claims, jwt_auth_accept_guest_middleware, jwt_auth_middleware},
    error::Error,
    models::TrainingPlanContent,
    perm::{Action, Resource, check_permission},
};

pub fn top_routes() -> Router<Arc<AppState>> {
    Router::new()
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
        .route("/{plan_id}", get(get_training_plan))
        .route("/", get(list_training_plans))
        .merge(
            Router::new()
                .route("/", post(create_training_plan))
                .route("/{plan_id}", put(put_training_plan))
                .route("/{plan_id}", delete(delete_training_plan))
                .route("/{plan_id}/participants", put(set_participants))
                .route("/{plan_id}/contests", put(set_contests))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    jwt_auth_middleware,
                )),
        )
        .merge(Router::new().layer(middleware::from_fn_with_state(
            state.clone(),
            jwt_auth_accept_guest_middleware,
        )))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateTrainingPlanRequest {
    name: String,
    description: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateTrainingPlanResponse {
    plan_id: i32,
}

#[utoipa::path(
    post,
    path = "/api/training-plans",
    request_body = CreateTrainingPlanRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = CreateTrainingPlanResponse),
    ),
    tag = "training_plan"
)]
async fn create_training_plan(
    state: State,
    claims: Extension<Claims>,
    Json(p): Json<CreateTrainingPlanRequest>,
) -> Result<Json<CreateTrainingPlanResponse>> {
    check_permission(
        &state.pool,
        &claims,
        Action::CreateTrainingPlan,
        Resource::Global,
    )
    .await?;

    if p.name.trim().is_empty() {
        bail!(@BAD_REQUEST "name cannot be empty");
    }

    let plan_id: i32 = sqlx::query_scalar!(
        r#"
        INSERT INTO training_plans (creator_id, name)
        VALUES ($1, $2)
        RETURNING id
        "#,
        claims.sub,
        p.name.trim()
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return Error::msg("training plan name already exists")
                    .status_code(StatusCode::BAD_REQUEST);
            }
        }
        Error::msg(format!("database error: {}", e))
    })?;

    let content = TrainingPlanContent {
        description: p.description,
    };
    state
        .write_training_plan_content(plan_id, &content)
        .await
        .map_err(|e| Error::msg(format!("failed to write training plan content: {:?}", e)))?;

    Ok(Json(CreateTrainingPlanResponse { plan_id }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetTrainingPlanResponse {
    id: i32,
    creator_id: i32,
    name: String,
    description: String,
    created_at: String,
    updated_at: String,
    participants: Vec<ParticipantInfo>,
    contests: Vec<ContestInfo>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ParticipantInfo {
    user_id: i32,
    username: String,
    joined_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContestInfo {
    contest_id: i32,
    name: String,
    begin_time: String,
    end_time: String,
    added_at: String,
}

#[utoipa::path(
    get,
    path = "/api/training-plans/{plan_id}",
    params(
        ("plan_id" = i32, Path)
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = GetTrainingPlanResponse),
    ),
    tag = "training_plan"
)]
async fn get_training_plan(
    state: State,
    Path(plan_id): Path<i32>,
) -> Result<Json<GetTrainingPlanResponse>> {
    let plan = sqlx::query!(
        r#"
        SELECT id, creator_id, name, created_at, updated_at
        FROM training_plans
        WHERE id = $1
        "#,
        plan_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("training plan not found").status_code(StatusCode::NOT_FOUND))?;

    let content = state
        .read_training_plan_content(plan_id)
        .await
        .map_err(|e| Error::msg(format!("failed to read training plan content: {:?}", e)))?;

    let participants = sqlx::query!(
        r#"
        SELECT 
            tpp.user_id,
            u.username,
            tpp.joined_at
        FROM training_plan_participants tpp
        JOIN users u ON tpp.user_id = u.id
        WHERE tpp.plan_id = $1
        ORDER BY tpp.joined_at
        "#,
        plan_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .into_iter()
    .map(|row| ParticipantInfo {
        user_id: row.user_id,
        username: row.username,
        joined_at: row.joined_at,
    })
    .collect();

    let contests = sqlx::query!(
        r#"
        SELECT 
            tpc.contest_id,
            c.name,
            c.begin_time,
            c.end_time,
            tpc.created_at as added_at
        FROM training_plan_contests tpc
        JOIN contests c ON tpc.contest_id = c.id
        WHERE tpc.plan_id = $1
        ORDER BY c.begin_time
        "#,
        plan_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .into_iter()
    .map(|row| ContestInfo {
        contest_id: row.contest_id,
        name: row.name,
        begin_time: row.begin_time.to_rfc3339(),
        end_time: row.end_time.to_rfc3339(),
        added_at: row.added_at.to_rfc3339(),
    })
    .collect();

    Ok(Json(GetTrainingPlanResponse {
        id: plan.id,
        creator_id: plan.creator_id,
        name: plan.name,
        description: content.description,
        created_at: plan.created_at.to_rfc3339(),
        updated_at: plan.updated_at.to_rfc3339(),
        participants,
        contests,
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TrainingPlanListItem {
    id: i32,
    creator_id: i32,
    name: String,
    participant_count: i64,
    contest_count: i64,
}
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListTrainingPlansQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    end_after: Option<DateTime<Utc>>,
}
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListTrainingPlansResponse {
    plans: Vec<TrainingPlanListItem>,
    total: i64,
}
#[utoipa::path(
    get,
    path = "/api/training-plans",
    security(("bearer_auth" = [])),
    params(
        ("page" = Option<i64>, Query),
        ("pageSize" = Option<i64>, Query),
        ("endAfter" = Option<DateTime<Utc>>, Query),
    ),
    responses(
        (status = 200, body = ListTrainingPlansResponse),
    ),
    tag = "training_plan"
)]
async fn list_training_plans(
    state: State,
    Query(q): Query<ListTrainingPlansQuery>,
) -> Result<Json<ListTrainingPlansResponse>> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let (count_query, list_query, bind_end_after) = if let Some(end_after) = q.end_after {
        (
            "SELECT COUNT(*) FROM training_plans tp 
             WHERE tp.id != 0 AND EXISTS (
                 SELECT 1 FROM training_plan_contests tpc2
                 JOIN contests c ON tpc2.contest_id = c.id
                 WHERE tpc2.plan_id = tp.id AND c.end_time > $1
             )",
            r#"
            SELECT 
                tp.id, tp.creator_id, tp.name,
                COUNT(DISTINCT tpp.user_id) as participant_count,
                COUNT(DISTINCT tpc.contest_id) as contest_count
            FROM training_plans tp
            LEFT JOIN training_plan_participants tpp ON tp.id = tpp.plan_id
            LEFT JOIN training_plan_contests tpc ON tp.id = tpc.plan_id
            WHERE tp.id != 0 AND EXISTS (
                SELECT 1 FROM training_plan_contests tpc2
                JOIN contests c ON tpc2.contest_id = c.id
                WHERE tpc2.plan_id = tp.id AND c.end_time > $1
            )
            GROUP BY tp.id, tp.creator_id, tp.name
            ORDER BY tp.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            Some(end_after),
        )
    } else {
        (
            "SELECT COUNT(*) FROM training_plans tp WHERE tp.id != 0",
            r#"
            SELECT 
                tp.id, tp.creator_id, tp.name, 
                COUNT(DISTINCT tpp.user_id) as participant_count,
                COUNT(DISTINCT tpc.contest_id) as contest_count
            FROM training_plans tp
            LEFT JOIN training_plan_participants tpp ON tp.id = tpp.plan_id
            LEFT JOIN training_plan_contests tpc ON tp.id = tpc.plan_id
            WHERE tp.id != 0
            GROUP BY tp.id, tp.creator_id, tp.name
            ORDER BY tp.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            None,
        )
    };

    // Count query
    let total: i64 = if let Some(end_after) = bind_end_after {
        sqlx::query_scalar(count_query)
            .bind(end_after)
            .fetch_one(&state.pool)
            .await?
    } else {
        sqlx::query_scalar(count_query)
            .fetch_one(&state.pool)
            .await?
    };

    // List query
    let rows = if let Some(end_after) = bind_end_after {
        sqlx::query(list_query)
            .bind(end_after)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?
    } else {
        sqlx::query(list_query)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?
    };

    let plans = rows
        .into_iter()
        .map(|row| TrainingPlanListItem {
            id: row.get("id"),
            creator_id: row.get("creator_id"),
            name: row.get("name"),
            participant_count: row.get("participant_count"),
            contest_count: row.get("contest_count"),
        })
        .collect();

    Ok(Json(ListTrainingPlansResponse { plans, total }))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PutTrainingPlanRequest {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PutTrainingPlanResponse {
    success: bool,
}

#[utoipa::path(
    put,
    path = "/api/training-plans/{plan_id}",
    params(
        ("plan_id" = i32, Path)
    ),
    request_body = PutTrainingPlanRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = PutTrainingPlanResponse),
    ),
    tag = "training_plan"
)]
async fn put_training_plan(
    state: State,
    claims: Extension<Claims>,
    Path(plan_id): Path<i32>,
    Json(req): Json<PutTrainingPlanRequest>,
) -> Result<Json<PutTrainingPlanResponse>> {
    check_permission(
        &state.pool,
        &claims,
        Action::PutTrainingPlan,
        Resource::TrainingPlan(plan_id),
    )
    .await?;

    let _plan = sqlx::query!(
        r#"
        SELECT id
        FROM training_plans
        WHERE id = $1
        "#,
        plan_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("training plan not found").status_code(StatusCode::NOT_FOUND))?;

    if let Some(name) = req.name {
        sqlx::query!(
            r#"
            UPDATE training_plans
            SET name = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            name,
            plan_id
        )
        .execute(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;
    }

    if let Some(description) = req.description {
        let content = TrainingPlanContent { description };
        state
            .write_training_plan_content(plan_id, &content)
            .await
            .map_err(|e| Error::msg(format!("failed to write training plan content: {:?}", e)))?;

        sqlx::query!(
            r#"
            UPDATE training_plans
            SET updated_at = NOW()
            WHERE id = $1
            "#,
            plan_id
        )
        .execute(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;
    }

    Ok(Json(PutTrainingPlanResponse { success: true }))
}

#[utoipa::path(
    delete,
    path = "/api/training-plans/{plan_id}",
    params(
        ("plan_id" = i32, Path)
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = ()),
        (status = 400, description = "Cannot delete training plan with started contests"),
        (status = 404, description = "Training plan not found"),
    ),
    tag = "training_plan"
)]
async fn delete_training_plan(
    state: State,
    claims: Extension<Claims>,
    Path(plan_id): Path<i32>,
) -> Result<()> {
    check_permission(
        &state.pool,
        &claims,
        Action::DeleteTrainingPlan,
        Resource::TrainingPlan(plan_id),
    )
    .await?;

    let _plan = sqlx::query!(
        r#"
        SELECT id
        FROM training_plans
        WHERE id = $1
        "#,
        plan_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("training plan not found").status_code(StatusCode::NOT_FOUND))?;

    // Check if all contests in the training plan haven't started yet
    let started_contest_count = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM training_plan_contests tpc
        JOIN contests c ON tpc.contest_id = c.id
        WHERE tpc.plan_id = $1 AND c.begin_time <= NOW()
        "#,
        plan_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .count
    .unwrap_or(0);

    if started_contest_count > 0 {
        return Err(Error::msg(
            "cannot delete training plan with contests that have already started",
        )
        .status_code(StatusCode::BAD_REQUEST));
    }

    sqlx::query!(
        r#"
        DELETE FROM training_plans
        WHERE id = $1
        "#,
        plan_id
    )
    .execute(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    // state.delete_training_plan_content(plan_id).await?;

    Ok(())
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetParticipantsRequest {
    user_ids: Vec<i32>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetParticipantsResponse {
    added: i32,
    removed: i32,
}

#[utoipa::path(
    put,
    path = "/api/training-plans/{plan_id}/participants",
    params(
        ("plan_id" = i32, Path)
    ),
    request_body = SetParticipantsRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = SetParticipantsResponse),
    ),
    tag = "training_plan"
)]
async fn set_participants(
    state: State,
    claims: Extension<Claims>,
    Path(plan_id): Path<i32>,
    Json(req): Json<SetParticipantsRequest>,
) -> Result<Json<SetParticipantsResponse>> {
    check_permission(
        &state.pool,
        &claims,
        Action::PutTrainingPlan,
        Resource::TrainingPlan(plan_id),
    )
    .await?;

    sqlx::query!(
        r#"
        SELECT id
        FROM training_plans
        WHERE id = $1
        "#,
        plan_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("training plan not found").status_code(StatusCode::NOT_FOUND))?;

    let contest_ids = sqlx::query!(
        r#"
        SELECT contest_id
        FROM training_plan_contests
        WHERE plan_id = $1
        "#,
        plan_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .into_iter()
    .map(|r| r.contest_id)
    .collect::<Vec<_>>();

    let current_participants = sqlx::query!(
        r#"
        SELECT user_id
        FROM training_plan_participants
        WHERE plan_id = $1
        "#,
        plan_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .into_iter()
    .map(|r| r.user_id)
    .collect::<std::collections::HashSet<_>>();

    let new_participants = req
        .user_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>();

    // diff
    let to_add: Vec<i32> = new_participants
        .difference(&current_participants)
        .copied()
        .collect();
    let to_remove: Vec<i32> = current_participants
        .difference(&new_participants)
        .copied()
        .collect();

    let mut added = 0;
    let mut removed = 0;

    if !to_add.is_empty() {
        let existing_users = sqlx::query!(
            r#"
            SELECT id
            FROM users
            WHERE id = ANY($1)
            "#,
            &to_add
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?
        .into_iter()
        .map(|r| r.id)
        .collect::<std::collections::HashSet<_>>();

        let invalid_users: Vec<i32> = to_add
            .iter()
            .filter(|id| !existing_users.contains(id))
            .copied()
            .collect();
        if !invalid_users.is_empty() {
            return Err(Error::msg(format!("users not found: {:?}", invalid_users))
                .status_code(StatusCode::NOT_FOUND));
        }

        // add
        for user_id in &to_add {
            // training_plan_participants
            sqlx::query!(
                r#"
                INSERT INTO training_plan_participants (plan_id, user_id)
                VALUES ($1, $2)
                "#,
                plan_id,
                user_id
            )
            .execute(&state.pool)
            .await
            .map_err(|e| Error::msg(format!("database error: {}", e)))?;

            // contest_participants
            for contest_id in &contest_ids {
                sqlx::query!(
                    r#"
                    INSERT INTO contest_participants (contest_id, user_id, training_plan_id)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (contest_id, user_id, training_plan_id) DO NOTHING
                    "#,
                    contest_id,
                    user_id,
                    plan_id
                )
                .execute(&state.pool)
                .await
                .map_err(|e| Error::msg(format!("database error: {}", e)))?;
            }

            added += 1;
        }
    }

    // rm
    if !to_remove.is_empty() {
        // training_plan_participants
        sqlx::query!(
            r#"
            DELETE FROM training_plan_participants
            WHERE plan_id = $1 AND user_id = ANY($2)
            "#,
            plan_id,
            &to_remove
        )
        .execute(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;

        // contest_participants
        for contest_id in &contest_ids {
            sqlx::query!(
                r#"
                DELETE FROM contest_participants
                WHERE contest_id = $1 AND user_id = ANY($2) AND training_plan_id = $3
                "#,
                contest_id,
                &to_remove,
                plan_id
            )
            .execute(&state.pool)
            .await
            .map_err(|e| Error::msg(format!("database error: {}", e)))?;
        }

        removed = to_remove.len() as i32;
    }

    Ok(Json(SetParticipantsResponse { added, removed }))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetContestsRequest {
    contest_ids: Vec<i32>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetContestsResponse {
    added: i32,
    removed: i32,
}

#[utoipa::path(
    put,
    path = "/api/training-plans/{plan_id}/contests",
    params(
        ("plan_id" = i32, Path)
    ),
    request_body = SetContestsRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = SetContestsResponse),
    ),
    tag = "training_plan"
)]
async fn set_contests(
    state: State,
    claims: Extension<Claims>,
    Path(plan_id): Path<i32>,
    Json(req): Json<SetContestsRequest>,
) -> Result<Json<SetContestsResponse>> {
    check_permission(
        &state.pool,
        &claims,
        Action::PutTrainingPlan,
        Resource::TrainingPlan(plan_id),
    )
    .await?;

    sqlx::query!(
        r#"
        SELECT id
        FROM training_plans
        WHERE id = $1
        "#,
        plan_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("training plan not found").status_code(StatusCode::NOT_FOUND))?;

    let current_contests = sqlx::query!(
        r#"
        SELECT contest_id
        FROM training_plan_contests
        WHERE plan_id = $1
        "#,
        plan_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .into_iter()
    .map(|r| r.contest_id)
    .collect::<std::collections::HashSet<_>>();

    let new_contests = req
        .contest_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>();

    // calc diff
    let to_add: Vec<i32> = new_contests
        .difference(&current_contests)
        .copied()
        .collect();
    let to_remove: Vec<i32> = current_contests
        .difference(&new_contests)
        .copied()
        .collect();

    // check
    let mut contests_to_check = to_add.clone();
    contests_to_check.extend(to_remove.iter().copied());
    if !contests_to_check.is_empty() {
        let started_contests = sqlx::query!(
            r#"
        SELECT id
        FROM contests
        WHERE id = ANY($1) AND begin_time <= $2
        "#,
            &contests_to_check,
            Utc::now()
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;
        if !started_contests.is_empty() {
            let started_ids: Vec<i32> = started_contests.iter().map(|r| r.id).collect();
            return Err(Error::msg(format!(
                "cannot modify contests that have already started: {:?}",
                started_ids
            ))
            .status_code(StatusCode::BAD_REQUEST));
        }
    }

    let mut added = 0;
    let mut removed = 0;

    if !to_add.is_empty() {
        let existing_contests = sqlx::query!(
            r#"
            SELECT id
            FROM contests
            WHERE id = ANY($1)
            "#,
            &to_add
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?
        .into_iter()
        .map(|r| r.id)
        .collect::<std::collections::HashSet<_>>();

        let invalid_contests: Vec<i32> = to_add
            .iter()
            .filter(|id| !existing_contests.contains(id))
            .copied()
            .collect();
        if !invalid_contests.is_empty() {
            return Err(
                Error::msg(format!("contests not found: {:?}", invalid_contests))
                    .status_code(StatusCode::NOT_FOUND),
            );
        }

        let participants = sqlx::query!(
            r#"
            SELECT user_id
            FROM training_plan_participants
            WHERE plan_id = $1
            "#,
            plan_id
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?
        .into_iter()
        .map(|r| r.user_id)
        .collect::<Vec<_>>();

        // add
        for contest_id in &to_add {
            // training_plan_contests
            sqlx::query!(
                r#"
                INSERT INTO training_plan_contests (plan_id, contest_id)
                VALUES ($1, $2)
                "#,
                plan_id,
                contest_id
            )
            .execute(&state.pool)
            .await
            .map_err(|e| Error::msg(format!("database error: {}", e)))?;

            // contest_participants
            for user_id in &participants {
                sqlx::query!(
                    r#"
                    INSERT INTO contest_participants (contest_id, user_id, training_plan_id)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (contest_id, user_id, training_plan_id) DO NOTHING
                    "#,
                    contest_id,
                    user_id,
                    plan_id
                )
                .execute(&state.pool)
                .await
                .map_err(|e| Error::msg(format!("database error: {}", e)))?;
            }

            added += 1;
        }
    }

    // rm
    if !to_remove.is_empty() {
        // contest_participants
        sqlx::query!(
            r#"
            DELETE FROM contest_participants
            WHERE contest_id = ANY($1) AND training_plan_id = $2
            "#,
            &to_remove,
            plan_id
        )
        .execute(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;

        // training_plan_contests
        sqlx::query!(
            r#"
            DELETE FROM training_plan_contests
            WHERE plan_id = $1 AND contest_id = ANY($2)
            "#,
            plan_id,
            &to_remove
        )
        .execute(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;

        removed = to_remove.len() as i32;
    }

    Ok(Json(SetContestsResponse { added, removed }))
}
