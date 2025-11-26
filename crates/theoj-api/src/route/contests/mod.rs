pub(crate) mod ranking_cache;

pub use ranking_cache::ContestRankingItem;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    middleware,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use theoj_common::{bail, judge::SubmissionResult};
use utoipa::{IntoParams, ToSchema};

use crate::{
    AppState, Result, State,
    auth::{
        Claims, hash_password, jwt_auth_accept_guest_middleware, jwt_auth_middleware,
        verify_password,
    },
    error::Error,
    models::ContestContent,
    perm::{Action, Resource, UserRole, check_permission, role_of_claims},
};

pub fn top_routes() -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
        .merge(
            Router::new()
                .route("/", get(list_contests))
                .route("/{contest_id}", get(get_contest))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    jwt_auth_accept_guest_middleware,
                )),
        )
        .merge(
            Router::new()
                .route("/", post(create_contest))
                .route("/{contest_id}", put(put_contest))
                .route("/{contest_id}", delete(delete_contest))
                .route("/{contest_id}/join", post(join_contest))
                .route("/{contest_id}/ranking", get(get_contest_ranking))
                .route("/overall-ranking", get(get_overall_ranking))
                .layer(middleware::from_fn_with_state(state, jwt_auth_middleware)),
        )
}

#[derive(Serialize, Deserialize, ToSchema, Debug, sqlx::Type, PartialEq)]
#[sqlx(type_name = "contest_status_enum")]
#[sqlx(rename_all = "lowercase")]
pub enum ContestStatus {
    Active,
    Hidden,
}

#[derive(Serialize, Deserialize, ToSchema, Debug, sqlx::Type, PartialEq)]
#[sqlx(type_name = "contest_type_enum")]
#[sqlx(rename_all = "lowercase")]
pub enum ContestType {
    Public,
    Private,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateContestRequest {
    name: String,
    description: String,
    begin_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    password: Option<String>,
    #[serde(rename = "type")]
    contest_type: ContestType,
    problem_ids: Vec<i32>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateContestResponse {
    contest_id: String,
}

#[utoipa::path(
    post,
    path = "/api/contests",
    request_body = CreateContestRequest,
    responses(
        (status = 200, body = CreateContestResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "contest"
)]
async fn create_contest(
    state: State,
    claims: Extension<Claims>,
    Json(p): Json<CreateContestRequest>,
) -> Result<Json<CreateContestResponse>> {
    check_permission(
        &state.pool,
        &claims,
        Action::CreateContest,
        Resource::Global,
    )
    .await?;

    if p.name.is_empty() || p.description.is_empty() {
        bail!(@BAD_REQUEST "name and description are required");
    }

    if p.problem_ids.is_empty() {
        bail!(@BAD_REQUEST "at least one problem is required");
    }

    if p.problem_ids.len() > 10 {
        bail!(@BAD_REQUEST "contest can have at most 10 problems");
    }

    if p.begin_time >= p.end_time {
        bail!(@BAD_REQUEST "begin time must be before end time");
    }

    let hashed_password = p.password.map(|p| hash_password(p)).transpose()?;

    let contest_id: i32 = sqlx::query_scalar!(
        r#"
        INSERT INTO contests (creator_id, name, begin_time, end_time, password, type, status)
        VALUES ($1, $2, $3, $4, $5, $6, 'active')
        RETURNING id
        "#,
        claims.sub,
        p.name,
        p.begin_time,
        p.end_time,
        hashed_password,
        p.contest_type as ContestType
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return Error::msg("contest name already exists")
                    .status_code(StatusCode::BAD_REQUEST);
            }
        }
        Error::msg(format!("database error: {}", e))
    })?;

    let content = ContestContent {
        description: p.description,
    };

    state
        .write_contest_content(contest_id, &content)
        .await
        .map_err(|e| Error::msg(format!("failed to write contest content: {:?}", e)))?;

    for (index, problem_id) in p.problem_ids.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO contest_problems (contest_id, problem_id, number)
            VALUES ($1, $2, $3)
            "#,
            contest_id,
            problem_id,
            index as i32
        )
        .execute(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("failed to add problem to contest: {}", e)))?;
    }

    Ok(Json(CreateContestResponse {
        contest_id: contest_id.to_string(),
    }))
}

async fn check_contest_password(
    pool: &sqlx::PgPool,
    contest_id: i32,
    password: Option<String>,
) -> Result<()> {
    let contest = sqlx::query!(
        r#"
        SELECT password
        FROM contests
        WHERE id = $1
        "#,
        contest_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("contest not found").status_code(StatusCode::NOT_FOUND))?;

    match (contest.password, password) {
        (None, _) => Ok(()), // No password required
        (Some(_), None) => bail!(@FORBIDDEN "contest password required"),
        (Some(hash), Some(pwd)) => verify_password(pwd, hash)
            .map_err(|_| Error::msg("contest password wrong").status_code(StatusCode::FORBIDDEN)),
    }
}

#[derive(Serialize, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListContestsQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    end_after: Option<DateTime<Utc>>,
}
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContestListItem {
    contest_id: String,
    name: String,
    begin_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    #[serde(rename = "type")]
    contest_type: ContestType,
    has_password: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListContestsResponse {
    contests: Vec<ContestListItem>,
    total: i64,
}

#[utoipa::path(
    get,
    path = "/api/contests",
    params(ListContestsQuery),
    responses(
        (status = 200, body = ListContestsResponse),
    ),
    tag = "contest"
)]
async fn list_contests(
    state: State,
    claims: Extension<Claims>,
    Query(q): Query<ListContestsQuery>,
) -> Result<Json<ListContestsResponse>> {
    let user_role = role_of_claims(&state.pool, &claims).await?;
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let end_after = q
        .end_after
        .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
    let (count_query, select_query) = match user_role {
        UserRole::Teacher | UserRole::Admin => (
            "SELECT COUNT(*) FROM contests WHERE end_time > $1",
            r#"
            SELECT id, name, begin_time, end_time, type, (password IS NOT NULL) as has_password
            FROM contests
            WHERE end_time > $1
            ORDER BY begin_time DESC
            LIMIT $2 OFFSET $3
            "#,
        ),
        _ => (
            "SELECT COUNT(*) FROM contests WHERE status = 'active' AND end_time > $1",
            r#"
            SELECT id, name, begin_time, end_time, type, (password IS NOT NULL) as has_password
            FROM contests
            WHERE status = 'active' AND end_time > $1
            ORDER BY begin_time DESC
            LIMIT $2 OFFSET $3
            "#,
        ),
    };
    let total: i64 = sqlx::query_scalar(count_query)
        .bind(end_after)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;
    let contests = sqlx::query(select_query)
        .bind(end_after)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?
        .into_iter()
        .map(|row| ContestListItem {
            contest_id: row.get::<i32, _>("id").to_string(),
            name: row.get::<String, _>("name"),
            begin_time: row.get::<DateTime<Utc>, _>("begin_time"),
            end_time: row.get::<DateTime<Utc>, _>("end_time"),
            contest_type: row.get::<ContestType, _>("type"),
            has_password: row.get::<bool, _>("has_password"),
        })
        .collect();
    Ok(Json(ListContestsResponse { contests, total }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetContestResponse {
    contest_id: String,
    name: String,
    description: String,
    begin_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    #[serde(rename = "type")]
    contest_type: ContestType,
    status: ContestStatus,
    problem_ids: Vec<i32>,
    has_password: bool,
}

#[derive(Serialize, Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetContestQuery {
    password: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/contests/{contest_id}",
    params(
        ("contest_id" = String, Path, description = "Contest ID"),
        GetContestQuery
    ),
    responses(
        (status = 200, body = GetContestResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "contest"
)]
async fn get_contest(
    state: State,
    claims: Extension<Claims>,
    Path(contest_id): Path<String>,
    Query(query): Query<GetContestQuery>,
) -> Result<Json<GetContestResponse>> {
    let contest_id: i32 = contest_id
        .parse()
        .map_err(|_| Error::msg("invalid contest id").status_code(StatusCode::BAD_REQUEST))?;

    let contest = sqlx::query!(
        r#"
        SELECT id, name, begin_time, end_time, password, type as "type_: ContestType", status as "status_: ContestStatus", created_at
        FROM contests
        WHERE id = $1
        "#,
        contest_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("contest not found").status_code(StatusCode::NOT_FOUND))?;

    let user_role = role_of_claims(&state.pool, &claims).await?;

    // check hidden
    if contest.status_ == ContestStatus::Hidden {
        match user_role {
            UserRole::Teacher | UserRole::Admin => {}
            _ => bail!(@NOT_FOUND "contest not found"),
        }
    }

    check_contest_password(&state.pool, contest_id, query.password).await?;

    // Get contest content
    let content = state
        .read_contest_content(contest_id)
        .await
        .unwrap_or(ContestContent {
            description: String::new(),
        });

    let mut is_allowed = true;
    let now = chrono::Utc::now();
    if contest.begin_time > now {
        is_allowed = match user_role {
            UserRole::Admin => true,
            _ => {
                let owner_id = Resource::Contest(contest_id).owner_id(&state.pool).await?;
                owner_id == claims.sub
            }
        };
    };

    // Get problem list
    let problem_ids = match is_allowed {
        true => sqlx::query_scalar!(
            "SELECT problem_id FROM contest_problems WHERE contest_id = $1 ORDER BY problem_id",
            contest_id
        )
        .fetch_all(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?,
        false => vec![],
    };

    Ok(Json(GetContestResponse {
        contest_id: contest.id.to_string(),
        name: contest.name,
        description: content.description,
        begin_time: contest.begin_time,
        end_time: contest.end_time,
        has_password: contest.password.is_some(),
        contest_type: contest.type_,
        status: contest.status_,
        problem_ids,
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateContestRequest {
    name: Option<String>,
    description: Option<String>,
    begin_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    password: Option<String>,
    #[serde(rename = "type")]
    contest_type: Option<ContestType>,
    status: Option<ContestStatus>,
    problem_ids: Option<Vec<i32>>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateContestResponse {
    contest_id: String,
}

#[utoipa::path(
    put,
    path = "/api/contests/{contest_id}",
    params(
        ("contest_id" = String, Path, description = "Contest ID")
    ),
    request_body = UpdateContestRequest,
    responses(
        (status = 200, body = UpdateContestResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "contest"
)]
async fn put_contest(
    state: State,
    claims: Extension<Claims>,
    Path(contest_id): Path<String>,
    Json(p): Json<UpdateContestRequest>,
) -> Result<Json<UpdateContestResponse>> {
    check_permission(
        &state.pool,
        &claims,
        Action::PutContest,
        Resource::Contest(contest_id.parse().unwrap()),
    )
    .await?;

    let contest_id: i32 = contest_id
        .parse()
        .map_err(|_| Error::msg("invalid contest id").status_code(StatusCode::BAD_REQUEST))?;

    // Check if contest exists
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM contests WHERE id = $1)",
        contest_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .unwrap_or(false);

    if !exists {
        bail!(@NOT_FOUND "contest not found");
    }

    // Validate time constraints if both times are being updated
    if let (Some(begin), Some(end)) = (&p.begin_time, &p.end_time) {
        if begin >= end {
            bail!(@BAD_REQUEST "begin time must be before end time");
        }
    }

    // Validate problem_ids length
    if let Some(ref problem_ids) = p.problem_ids {
        if problem_ids.is_empty() {
            bail!(@BAD_REQUEST "at least one problem is required");
        }
        if problem_ids.len() > 10 {
            bail!(@BAD_REQUEST "contest can have at most 10 problems");
        }
    }

    // Update basic contest info
    if p.name.is_some()
        || p.begin_time.is_some()
        || p.end_time.is_some()
        || p.password.is_some()
        || p.contest_type.is_some()
        || p.status.is_some()
    {
        let current = sqlx::query!(
            r#"
            SELECT name, begin_time, end_time, password, type as "type_: ContestType", status as "status_: ContestStatus"
            FROM contests
            WHERE id = $1
            "#,
            contest_id
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;

        let name = p.name.as_ref().unwrap_or(&current.name);
        let begin_time = p.begin_time.as_ref().unwrap_or(&current.begin_time);
        let end_time = p.end_time.as_ref().unwrap_or(&current.end_time);
        let password = if let Some(pwd) = &p.password {
            Some(hash_password(pwd.to_string())?)
        } else {
            current.password.clone()
        };
        let contest_type = p.contest_type.as_ref().unwrap_or(&current.type_);
        let status = p.status.as_ref().unwrap_or(&current.status_);

        if begin_time >= end_time {
            bail!(@BAD_REQUEST "begin time must be before end time");
        }

        sqlx::query!(
            r#"
            UPDATE contests
            SET name = $1, begin_time = $2, end_time = $3, password = $4, type = $5, status = $6
            WHERE id = $7
            "#,
            name,
            begin_time,
            end_time,
            password,
            contest_type as &ContestType,
            status as &ContestStatus,
            contest_id
        )
        .execute(&state.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return Error::msg("contest name already exists")
                        .status_code(StatusCode::BAD_REQUEST);
                }
            }
            Error::msg(format!("database error: {}", e))
        })?;
    }

    // Update description if provided
    if let Some(description) = p.description {
        let mut content = state
            .read_contest_content(contest_id)
            .await
            .unwrap_or(ContestContent {
                description: String::new(),
            });
        content.description = description;
        state
            .write_contest_content(contest_id, &content)
            .await
            .map_err(|e| Error::msg(format!("failed to write contest content: {:?}", e)))?;
    }

    // Update problem list if provided
    if let Some(problem_ids) = p.problem_ids {
        sqlx::query!(
            "DELETE FROM contest_problems WHERE contest_id = $1",
            contest_id
        )
        .execute(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;

        for (i, problem_id) in problem_ids.iter().enumerate() {
            sqlx::query!(
                r#"
                INSERT INTO contest_problems (contest_id, problem_id, number)
                VALUES ($1, $2, $3)
                "#,
                contest_id,
                problem_id,
                i as i32
            )
            .execute(&state.pool)
            .await
            .map_err(|e| Error::msg(format!("failed to add problem to contest: {}", e)))?;
        }
    }

    Ok(Json(UpdateContestResponse {
        contest_id: contest_id.to_string(),
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteContestResponse {
    contest_id: String,
}
#[utoipa::path(
    delete,
    path = "/api/contests/{contest_id}",
    params(
        ("contest_id" = String, Path, description = "Contest ID")
    ),
    responses(
        (status = 200, body = DeleteContestResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "contest"
)]
async fn delete_contest(
    state: State,
    claims: Extension<Claims>,
    Path(contest_id): Path<String>,
) -> Result<Json<DeleteContestResponse>> {
    let contest_id: i32 = contest_id
        .parse()
        .map_err(|_| Error::msg("invalid contest id").status_code(StatusCode::BAD_REQUEST))?;

    check_permission(
        &state.pool,
        &claims,
        Action::DeleteContest,
        Resource::Contest(contest_id),
    )
    .await?;

    // Check if contest exists and get begin_time
    let contest = sqlx::query!("SELECT begin_time FROM contests WHERE id = $1", contest_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    let contest = contest
        .ok_or_else(|| Error::msg("contest not found").status_code(StatusCode::NOT_FOUND))?;

    // Check if contest has started
    let now = chrono::Utc::now();
    if contest.begin_time <= now {
        bail!(@FORBIDDEN "cannot delete contest that has already started");
    }

    // Delete contest problems first (foreign key constraint)
    sqlx::query!(
        "DELETE FROM contest_problems WHERE contest_id = $1",
        contest_id
    )
    .execute(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    // Delete the contest
    sqlx::query!("DELETE FROM contests WHERE id = $1", contest_id)
        .execute(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    // Delete contest content file
    // let _ = state.delete_contest_content(contest_id).await;

    Ok(Json(DeleteContestResponse {
        contest_id: contest_id.to_string(),
    }))
}
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JoinContestRequest {
    password: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/contests/{contest_id}/join",
    params(
        ("contest_id" = String, Path, description = "Contest ID"),
    ),
    request_body = JoinContestRequest,
    responses(
        (status = 200, body = ()),
    ),
    security(("bearer_auth" = [])),
    tag = "contest"
)]
async fn join_contest(
    state: State,
    claims: Extension<Claims>,
    Path(contest_id): Path<String>,
    Json(req): Json<JoinContestRequest>,
) -> Result<()> {
    let contest_id: i32 = contest_id
        .parse()
        .map_err(|_| Error::msg("invalid contest id").status_code(StatusCode::BAD_REQUEST))?;

    let user_id = claims.sub;

    // Get contest info
    let contest = sqlx::query!(
        r#"
        SELECT status as "status_: ContestStatus"
        FROM contests
        WHERE id = $1
        "#,
        contest_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("contest not found").status_code(StatusCode::NOT_FOUND))?;

    // Check if contest is hidden
    if contest.status_ == ContestStatus::Hidden {
        let user_role = role_of_claims(&state.pool, &claims).await?;
        match user_role {
            UserRole::Teacher | UserRole::Admin => {}
            _ => bail!(@NOT_FOUND "contest not found"),
        }
    }

    // Verify password
    check_contest_password(&state.pool, contest_id, req.password).await?;

    // Check if already joined
    let already_joined = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM contest_participants WHERE contest_id = $1 AND user_id = $2)",
        contest_id,
        user_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .unwrap_or(false);

    if already_joined {
        bail!(@CONFLICT "already joined");
    }

    // Join contest
    sqlx::query!(
        "INSERT INTO contest_participants (contest_id, user_id) VALUES ($1, $2)",
        contest_id,
        user_id
    )
    .execute(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    Ok(())
}

pub struct ContestInfo {
    id: i32,
    begin_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetContestRankingResponse {
    rankings: Vec<ContestRankingItem>,
}

#[utoipa::path(
    get,
    path = "/api/contests/{contest_id}/ranking",
    params(
        ("contest_id" = String, Path, description = "Contest ID"),
        GetContestQuery
    ),
    responses(
        (status = 200, body = GetContestRankingResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "contest"
)]
async fn get_contest_ranking(
    state: State,
    claims: Extension<Claims>,
    Path(contest_id): Path<String>,
    Query(query): Query<GetContestQuery>,
) -> Result<Json<GetContestRankingResponse>> {
    let contest_id: i32 = contest_id
        .parse()
        .map_err(|_| Error::msg("invalid contest id").status_code(StatusCode::BAD_REQUEST))?;

    // Get contest info
    let contest = sqlx::query!(
        r#"
        SELECT id, begin_time, end_time, status as "status_: ContestStatus"
        FROM contests
        WHERE id = $1
        "#,
        contest_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("contest not found").status_code(StatusCode::NOT_FOUND))?;

    let user_role = role_of_claims(&state.pool, &claims).await?;

    // Check hidden status
    if contest.status_ == ContestStatus::Hidden {
        match user_role {
            UserRole::Teacher | UserRole::Admin => {}
            _ => bail!(@NOT_FOUND "contest not found"),
        }
    }

    // Verify password
    check_contest_password(&state.pool, contest_id, query.password).await?;

    let contest_info = ContestInfo {
        id: contest.id,
        begin_time: contest.begin_time,
        end_time: contest.end_time,
    };

    // let rankings = calculate_contest_ranking(&state.pool, &contest_info).await?;
    let rankings = ranking_cache::get_contest_ranking_cached(&state, &contest_info)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get contest ranking: {:?}", e);
            Error::msg("Failed to get contest ranking")
                .status_code(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    Ok(Json(GetContestRankingResponse { rankings }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverallRankingItem {
    user_id: String,
    username: String,
    contest_count: i32, // joined count
    total_solved: i32,
    total_penalty: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetOverallRankingResponse {
    rankings: Vec<OverallRankingItem>,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetOverallRankingQuery {
    contest_ids: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/api/contests/overall-ranking",
    params(GetOverallRankingQuery),
    responses(
        (status = 200, body = GetOverallRankingResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "contest"
)]
async fn get_overall_ranking(
    state: State,
    claims: Extension<Claims>,
    Query(query): Query<GetOverallRankingQuery>,
) -> Result<Json<GetOverallRankingResponse>> {
    check_permission(
        &state.pool,
        &claims,
        Action::ViewOverallRanking,
        Resource::Global,
    )
    .await?;

    // Parse contest IDs
    let contest_ids: Vec<i32> = query
        .contest_ids
        .into_iter()
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();

    if contest_ids.is_empty() {
        bail!(@BAD_REQUEST "no valid contest IDs provided");
    }

    // Get all contests info
    let contests = sqlx::query!(
        r#"
        SELECT id, begin_time, end_time
        FROM contests
        WHERE id = ANY($1)
        "#,
        &contest_ids
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    if contests.is_empty() {
        bail!(@NOT_FOUND "no contests found");
    }

    // Calculate ranking for each contest and aggregate
    let mut user_stats: std::collections::HashMap<i32, OverallRankingItem> =
        std::collections::HashMap::new();

    for contest in contests {
        let contest_info = ContestInfo {
            id: contest.id,
            begin_time: contest.begin_time,
            end_time: contest.end_time,
        };

        // let rankings = calculate_contest_ranking(&state.pool, &contest_info).await?;
        let rankings = ranking_cache::get_contest_ranking_cached(&state, &contest_info)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get contest ranking: {:?}", e);
                Error::msg("Failed to get contest ranking")
                    .status_code(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        for ranking in rankings {
            let user_id: i32 = ranking.user_id.parse().unwrap();

            let entry = user_stats
                .entry(user_id)
                .or_insert_with(|| OverallRankingItem {
                    user_id: ranking.user_id.clone(),
                    username: ranking.username.clone(),
                    contest_count: 0,
                    total_solved: 0,
                    total_penalty: 0,
                });

            entry.total_solved += ranking.solved_count;
            entry.total_penalty += ranking.total_penalty;
        }
    }

    // Count actual participation for each user
    for user_entry in user_stats.values_mut() {
        let user_id: i32 = user_entry.user_id.parse().unwrap();

        let participated_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM contest_participants WHERE user_id = $1 AND contest_id = ANY($2)",
            user_id,
            &contest_ids
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?
        .unwrap_or(0);

        user_entry.contest_count = participated_count as i32;
    }

    let mut overall_rankings: Vec<OverallRankingItem> = user_stats.into_values().collect();

    // Sort by total_solved (desc), then by total_penalty (asc), then by contest_count (desc)
    overall_rankings.sort_by(|a, b| {
        b.total_solved
            .cmp(&a.total_solved)
            .then_with(|| a.total_penalty.cmp(&b.total_penalty))
            .then_with(|| b.contest_count.cmp(&a.contest_count))
    });

    Ok(Json(GetOverallRankingResponse {
        rankings: overall_rankings,
    }))
}
