use axum::extract::DefaultBodyLimit;
use axum::{
    Extension, Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    middleware,
};
use chrono::{DateTime, Utc};
use koioj_common::judge::{JudgeTask, SubmissionResult, TestCase, TestCaseJudgeResult};
use koioj_common::{bail, judge::Language};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::route::contests::verify_contest_problem_access;
use crate::{
    AppState, Result, State,
    auth::{Claims, jwt_auth_accept_guest_middleware, jwt_auth_middleware},
    error::Error,
    models::*,
    perm::{Action, Resource, UserRole, check_permission, role_of_claims},
};

pub fn top_routes() -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
        .route("/{problem_id}/solutions", get(list_solutions))
        .route("/{problem_id}/solutions/{solution_id}", get(get_solution))
        .merge(
            Router::new()
                .route("/{problem_id}", get(get_problem))
                .route("/", get(list_problems))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    jwt_auth_accept_guest_middleware,
                )),
        )
        .merge(
            Router::new()
                .route("/", post(create_problem))
                .route("/{problem_id}", put(put_problem))
                .route("/{problem_id}", delete(delete_problem))
                .merge(
                    Router::new()
                        .route("/{problem_id}/test-cases", post(add_test_cases))
                        .layer(DefaultBodyLimit::max(256 * 1024 * 1024)),
                )
                .route("/{problem_id}/test-cases", get(get_test_cases))
                .route("/{problem_id}/solutions", post(create_solution))
                .route(
                    "/{problem_id}/solutions/{solution_id}",
                    delete(delete_solution),
                )
                .route("/{problem_id}/submissions", post(submit))
                .route("/{problem_id}/submissions", get(list_submissions))
                .route(
                    "/{problem_id}/submissions/{submission_id}",
                    get(get_submission),
                )
                .route("/{problem_id}/ac-status", get(get_ac_status))
                .layer(middleware::from_fn_with_state(state, jwt_auth_middleware)),
        )
}

#[derive(Serialize, Deserialize, ToSchema, Debug, sqlx::Type, PartialEq)]
#[sqlx(type_name = "problem_status_enum")]
#[sqlx(rename_all = "lowercase")]
pub enum ProblemStatus {
    Active,
    Hidden,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateProblemRequest {
    name: String,
    description: String,
    input_description: String,
    output_description: String,
    samples: Vec<TestCaseData>,
    note: Option<String>,
    time_limit: i32,
    mem_limit: i32,
    status: ProblemStatus,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateProblemResponse {
    problem_id: i32,
}

#[utoipa::path(
    post,
    path = "/api/problems",
    request_body = CreateProblemRequest,
    responses(
        (status = 200, body = CreateProblemResponse),
    ),
    security(("bearer_auth" = [])),
    tag = "problem"
)]
async fn create_problem(
    state: State,
    claims: Extension<Claims>,
    Json(p): Json<CreateProblemRequest>,
) -> Result<Json<CreateProblemResponse>> {
    check_permission(
        &state.pool,
        &claims,
        Action::CreateProblem,
        Resource::Global,
    )
    .await?;

    if p.name.is_empty()
        || p.description.is_empty()
        || p.input_description.is_empty()
        || p.output_description.is_empty()
    {
        bail!(@BAD_REQUEST "required fields are missing");
    }

    if p.time_limit <= 0 || p.mem_limit <= 0 {
        bail!(@BAD_REQUEST "time_limit and mem_limit must be positive");
    }

    let problem_id: i32 = sqlx::query_scalar!(
        r#"
        INSERT INTO problems (name, time_limit, mem_limit, status)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        p.name,
        p.time_limit,
        p.mem_limit,
        p.status as ProblemStatus
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return Error::msg("problem name already exists")
                    .status_code(StatusCode::BAD_REQUEST);
            }
        }
        Error::msg(format!("database error: {}", e))
    })?;

    let content = ProblemContent {
        description: p.description,
        input_description: p.input_description,
        output_description: p.output_description,
        samples: p.samples,
        note: p.note,
    };

    state.write_problem_content(problem_id, &content).await?;

    Ok(Json(CreateProblemResponse {
        problem_id: problem_id,
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListProblemsQuery {
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProblemListItem {
    problem_id: i32,
    name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListProblemsResponse {
    problems: Vec<ProblemListItem>,
    total: i64,
}

#[utoipa::path(
    get,
    path = "/api/problems",
    params(
        ("page" = Option<i64>, Query),
        ("pageSize" = Option<i64>, Query),
    ),
    responses(
        (status = 200, body = ListProblemsResponse),
    ),
    tag = "problem"
)]
async fn list_problems(
    state: State,
    claims: Extension<Claims>,
    Query(q): Query<ListProblemsQuery>,
) -> Result<Json<ListProblemsResponse>> {
    let user_role = role_of_claims(&state.pool, &claims).await?;

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let (count_query, select_query) = match user_role {
        UserRole::Teacher | UserRole::Admin => (
            "SELECT COUNT(*) FROM problems",
            r#"
            SELECT id, name
            FROM problems
            ORDER BY id
            LIMIT $1 OFFSET $2
            "#,
        ),
        _ => (
            "SELECT COUNT(*) FROM problems WHERE status = 'active'",
            r#"
            SELECT id, name
            FROM problems
            WHERE status = 'active'
            ORDER BY id
            LIMIT $1 OFFSET $2
            "#,
        ),
    };

    let total: i64 = sqlx::query_scalar(count_query)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    let problems = sqlx::query(select_query)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?
        .into_iter()
        .map(|row| ProblemListItem {
            problem_id: row.get::<i32, _>("id"),
            name: row.get::<String, _>("name"),
        })
        .collect();

    Ok(Json(ListProblemsResponse { problems, total }))
}

#[derive(Deserialize, IntoParams)]
struct GetProblemQuery {
    #[serde(rename = "contestId")]
    contest_id: Option<i32>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetProblemResponse {
    problem_id: i32,
    name: String,
    description: String,
    input_description: String,
    output_description: String,
    samples: Vec<TestCaseData>,
    note: Option<String>,
    time_limit: i32,
    mem_limit: i32,
    status: ProblemStatus,
}

#[utoipa::path(
    get,
    path = "/api/problems/{problem_id}",
    params(
        ("problem_id" = i32, Path),
        GetProblemQuery
    ),
    responses(
        (status = 200, body = GetProblemResponse),
    ),
    tag = "problem"
)]
async fn get_problem(
    state: State,
    claims: Extension<Claims>,
    Path(problem_id): Path<i32>,
    Query(query): Query<GetProblemQuery>,
) -> Result<Json<GetProblemResponse>> {
    let user_role = role_of_claims(&state.pool, &claims).await?;

    let should_check_active = if let Some(cid) = query.contest_id {
        verify_contest_problem_access(&state.pool, cid, problem_id, claims.sub).await?;
        false // don't check active for contest problems
    } else {
        !matches!(user_role, UserRole::Teacher | UserRole::Admin)
    };

    #[derive(Debug)]
    struct ProblemRecord {
        id: i32,
        name: String,
        time_limit: i32,
        mem_limit: i32,
        status: ProblemStatus,
    }
    let problem = if should_check_active {
        sqlx::query_as!(
            ProblemRecord,
            r#"
        SELECT id, name, time_limit, mem_limit, status as "status: ProblemStatus"
        FROM problems
        WHERE id = $1 AND status = 'active'
        "#,
            problem_id
        )
        .fetch_optional(&state.pool)
        .await
    } else {
        sqlx::query_as!(
            ProblemRecord,
            r#"
        SELECT id, name, time_limit, mem_limit, status as "status: ProblemStatus"
        FROM problems
        WHERE id = $1
        "#,
            problem_id
        )
        .fetch_optional(&state.pool)
        .await
    }
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("problem not found").status_code(StatusCode::NOT_FOUND))?;
    let content = state.read_problem_content(problem_id).await?;
    Ok(Json(GetProblemResponse {
        problem_id: problem.id,
        name: problem.name,
        description: content.description,
        input_description: content.input_description,
        output_description: content.output_description,
        samples: content.samples,
        note: content.note,
        time_limit: problem.time_limit,
        mem_limit: problem.mem_limit,
        status: problem.status,
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PutProblemRequest {
    name: Option<String>,
    description: Option<String>,
    input_description: Option<String>,
    output_description: Option<String>,
    samples: Option<Vec<TestCaseData>>,
    note: Option<String>,
    time_limit: Option<i32>,
    mem_limit: Option<i32>,
    status: Option<ProblemStatus>,
}

#[utoipa::path(
    put,
    path = "/api/problems/{problem_id}",
    request_body = PutProblemRequest,
    params(
        ("problem_id" = i32, Path)
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = ()),
    ),
    tag = "problem"
)]
async fn put_problem(
    state: State,
    claims: Extension<Claims>,
    Path(problem_id): Path<i32>,
    Json(p): Json<PutProblemRequest>,
) -> Result<()> {
    check_permission(
        &state.pool,
        &claims,
        Action::PutProblem,
        Resource::Problem(problem_id),
    )
    .await?;

    let mut content = state.read_problem_content(problem_id).await?;

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| Error::msg(format!("failed to start transaction: {}", e)))?;

    if let Some(name) = &p.name {
        if !name.is_empty() {
            sqlx::query!(
                r#"
                UPDATE problems SET name = $1, updated_at = NOW() WHERE id = $2
                "#,
                name,
                problem_id
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                if let sqlx::Error::Database(db_err) = &e {
                    if db_err.is_unique_violation() {
                        return Error::msg("problem name already exists")
                            .status_code(StatusCode::BAD_REQUEST);
                    }
                }
                Error::msg(format!("database error: {}", e))
            })?;
        }
    }

    if let Some(desc) = p.description {
        content.description = desc;
    }
    if let Some(input_desc) = p.input_description {
        content.input_description = input_desc;
    }
    if let Some(output_desc) = p.output_description {
        content.output_description = output_desc;
    }
    if let Some(samples) = p.samples {
        content.samples = samples;
    }
    if let Some(note) = p.note {
        content.note = Some(note);
    }

    if let Some(time_limit) = p.time_limit {
        if time_limit <= 0 {
            bail!(@BAD_REQUEST "time_limit must be positive");
        }
        sqlx::query!(
            r#"
            UPDATE problems SET time_limit = $1, updated_at = NOW() WHERE id = $2
            "#,
            time_limit,
            problem_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;
    }

    if let Some(mem_limit) = p.mem_limit {
        if mem_limit <= 0 {
            bail!(@BAD_REQUEST "mem_limit must be positive");
        }
        sqlx::query!(
            r#"
            UPDATE problems SET mem_limit = $1, updated_at = NOW() WHERE id = $2
            "#,
            mem_limit,
            problem_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;
    }

    if let Some(status) = p.status {
        sqlx::query!(
            r#"
            UPDATE problems SET status = $1, updated_at = NOW() WHERE id = $2
            "#,
            status as ProblemStatus,
            problem_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;
    }

    state.write_problem_content(problem_id, &content).await?;

    tx.commit()
        .await
        .map_err(|e| Error::msg(format!("failed to commit transaction: {}", e)))?;

    Ok(())
}

#[utoipa::path(
    delete,
    path = "/api/problems/{problem_id}",
    params(
        ("problem_id" = i32, Path)
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = ()),
    ),
    tag = "problem"
)]
async fn delete_problem(
    state: State,
    claims: Extension<Claims>,
    Path(problem_id): Path<i32>,
) -> Result<()> {
    check_permission(
        &state.pool,
        &claims,
        Action::DeleteProblem,
        Resource::Problem(problem_id),
    )
    .await?;

    let used_in_contest: Option<i32> = sqlx::query_scalar!(
        r#"
        SELECT contest_id FROM contest_problems WHERE problem_id = $1 LIMIT 1
        "#,
        problem_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    if used_in_contest.is_some() {
        bail!(@BAD_REQUEST "can't delete a using problem")
    }

    sqlx::query!(
        r#"
        DELETE FROM problems WHERE id = $1
        "#,
        problem_id
    )
    .execute(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    Ok(())
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddTestCasesRequest {
    test_cases: Vec<TestCaseData>,
}

#[utoipa::path(
    post,
    path = "/api/problems/{problem_id}/test-cases",
    request_body = AddTestCasesRequest,
    params(
        ("problem_id" = i32, Path)
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = ()),
    ),
    tag = "problem"
)]
async fn add_test_cases(
    state: State,
    claims: Extension<Claims>,
    Path(problem_id): Path<i32>,
    Json(p): Json<AddTestCasesRequest>,
) -> Result<()> {
    check_permission(
        &state.pool,
        &claims,
        Action::AddTestCases,
        Resource::Problem(problem_id),
    )
    .await?;

    sqlx::query!(
        r#"
        SELECT id FROM problems WHERE id = $1
        "#,
        problem_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("problem not found").status_code(StatusCode::NOT_FOUND))?;

    if p.test_cases.is_empty() {
        bail!(@BAD_REQUEST "test_cases cannot be empty");
    }

    for test_case in p.test_cases.iter() {
        let result = sqlx::query!(
            r#"
        INSERT INTO test_cases (problem_id) VALUES ($1) RETURNING id
        "#,
            problem_id
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;

        let test_case_id = result.id;
        state.write_test_cases(test_case_id, test_case).await?;
    }
    Ok(())
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetTestCasesResponse {
    test_cases: Vec<i32>,
}

#[utoipa::path(
    get,
    path = "/api/problems/{problem_id}/test-cases",
    params(
        ("problem_id" = i32, Path)
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = GetTestCasesResponse),
    ),
    tag = "problem"
)]
async fn get_test_cases(
    state: State,
    claims: Extension<Claims>,
    Path(problem_id): Path<i32>,
) -> Result<Json<GetTestCasesResponse>> {
    check_permission(
        &state.pool,
        &claims,
        Action::GetTestCases,
        Resource::Problem(problem_id),
    )
    .await?;

    let test_case_records = sqlx::query!(
        r#"
        SELECT id FROM test_cases WHERE problem_id = $1 ORDER BY id
        "#,
        problem_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    let test_case_ids: Vec<i32> = test_case_records
        .into_iter()
        .map(|record| record.id)
        .collect();

    Ok(Json(GetTestCasesResponse {
        test_cases: test_case_ids,
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateSolutionRequest {
    title: String,
    content: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateSolutionResponse {
    solution_id: i32,
}

#[utoipa::path(
    post,
    path = "/api/problems/{problem_id}/solutions",
    request_body = CreateSolutionRequest,
    params(
        ("problem_id" = i32, Path)
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = CreateSolutionResponse),
    ),
    tag = "problem"
)]
async fn create_solution(
    state: State,
    claims: Extension<Claims>,
    Path(problem_id): Path<i32>,
    Json(p): Json<CreateSolutionRequest>,
) -> Result<Json<CreateSolutionResponse>> {
    check_permission(
        &state.pool,
        &claims,
        Action::CreateSolution,
        Resource::Problem(problem_id),
    )
    .await?;

    if p.title.is_empty() || p.content.is_empty() {
        bail!(@BAD_REQUEST "title and content are required");
    }

    sqlx::query!(
        r#"
        SELECT id FROM problems WHERE id = $1
        "#,
        problem_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("problem not found").status_code(StatusCode::NOT_FOUND))?;

    let solution_id: i32 = sqlx::query_scalar!(
        r#"
        INSERT INTO solutions (problem_id, author, title)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        problem_id,
        claims.sub,
        p.title
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    let solution_content = SolutionContent { content: p.content };

    state
        .write_solution_content(solution_id, &solution_content)
        .await?;

    Ok(Json(CreateSolutionResponse {
        solution_id: solution_id,
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SolutionListItem {
    solution_id: i32,
    title: String,
    author_id: i32,
    author_name: String,
    created_at: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListSolutionsResponse {
    solutions: Vec<SolutionListItem>,
}

#[utoipa::path(
    get,
    path = "/api/problems/{problem_id}/solutions",
    params(
        ("problem_id" = i32, Path)
    ),
    responses(
        (status = 200, body = ListSolutionsResponse),
    ),
    tag = "problem"
)]
async fn list_solutions(
    state: State,
    Path(problem_id): Path<i32>,
) -> Result<Json<ListSolutionsResponse>> {
    let _problem = sqlx::query!(
        "SELECT id FROM problems WHERE id = $1 AND status = 'active'",
        problem_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("invalid problem_id"))?;

    let solutions = sqlx::query!(
        r#"
        SELECT s.id, s.title, s.author, s.created_at, u.username
        FROM solutions s
        JOIN users u ON s.author = u.id
        WHERE s.problem_id = $1
        ORDER BY s.created_at DESC
        "#,
        problem_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .into_iter()
    .map(|row| SolutionListItem {
        solution_id: row.id,
        title: row.title,
        author_id: row.author,
        author_name: row.username,
        created_at: row.created_at.to_rfc3339(),
    })
    .collect();

    Ok(Json(ListSolutionsResponse { solutions }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetSolutionResponse {
    solution_id: i32,
    title: String,
    content: String,
    author_id: i32,
    author_name: String,
    created_at: String,
}

#[utoipa::path(
    get,
    path = "/api/problems/{problem_id}/solutions/{solution_id}",
    params(
        ("problem_id" = i32, Path),
        ("solution_id" = i32, Path)
    ),
    responses(
        (status = 200, body = GetSolutionResponse),
    ),
    tag = "problem"
)]
async fn get_solution(
    state: State,
    Path((problem_id, solution_id)): Path<(i32, i32)>,
) -> Result<Json<GetSolutionResponse>> {
    let solution = sqlx::query!(
        r#"
        SELECT s.id, s.title, s.author, s.created_at, u.username
        FROM solutions s
        JOIN users u ON s.author = u.id
        JOIN problems p ON s.problem_id = p.id
        WHERE s.id = $1 AND s.problem_id = $2 AND p.status = 'active'
        "#,
        solution_id,
        problem_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("solution not found").status_code(StatusCode::NOT_FOUND))?;

    let solution_content = state.read_solution_content(solution_id).await?;

    Ok(Json(GetSolutionResponse {
        solution_id: solution.id,
        title: solution.title,
        content: solution_content.content,
        author_id: solution.author,
        author_name: solution.username,
        created_at: solution.created_at.to_rfc3339(),
    }))
}

#[utoipa::path(
    delete,
    path = "/api/problems/{problem_id}/solutions/{solution_id}",
    params(
        ("problem_id" = i32, Path),
        ("solution_id" = i32, Path)
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = ()),
    ),
    tag = "problem"
)]
async fn delete_solution(
    state: State,
    claims: Extension<Claims>,
    Path((problem_id, solution_id)): Path<(i32, i32)>,
) -> Result<()> {
    check_permission(
        &state.pool,
        &claims,
        Action::DeleteSolution,
        Resource::Solution(solution_id),
    )
    .await?;

    let deleted = sqlx::query!(
        r#"
        DELETE FROM solutions
        WHERE id = $1 AND problem_id = $2
        RETURNING id
        "#,
        solution_id,
        problem_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    if deleted.is_none() {
        bail!(@NOT_FOUND "solution not found");
    }

    Ok(())
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubmitRequest {
    code: String,
    lang: Language,
    contest_id: Option<i32>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubmitResponse {
    submission_id: i32,
}

#[utoipa::path(
    post,
    path = "/api/problems/{problem_id}/submissions",
    request_body = SubmitRequest,
    params(
        ("problem_id" = i32, Path)
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = SubmitResponse),
    ),
    tag = "problem"
)]
async fn submit(
    state: State,
    claims: Extension<Claims>,
    Path(problem_id): Path<i32>,
    Json(p): Json<SubmitRequest>,
) -> Result<Json<SubmitResponse>> {
    if p.code.is_empty() {
        bail!(@BAD_REQUEST "code and lang are required");
    }

    let contest_id = p.contest_id;

    // submitting to a contest's problem
    if let Some(cid) = contest_id {
        // verify contest exists and is in valid time range
        let _contest_exists = sqlx::query!(
            r#"
            SELECT id FROM contests 
            WHERE id = $1 
            AND status = 'active'
            AND begin_time <= NOW()
            AND end_time >= NOW()
            "#,
            cid
        )
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?
        .ok_or_else(|| {
            Error::msg("contest not in valid time range").status_code(StatusCode::FORBIDDEN)
        })?;

        // verify that this user participates in this contest
        let participant = sqlx::query!(
            r#"
            SELECT user_id FROM contest_participants WHERE contest_id = $1 AND user_id = $2
            "#,
            cid,
            claims.sub
        )
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?;

        if participant.is_none() {
            bail!(@FORBIDDEN "user not participating in this contest");
        }

        // verify that this problem is in this contest
        sqlx::query!(
            r#"
            SELECT contest_id FROM contest_problems 
            WHERE contest_id = $1 AND problem_id = $2
            "#,
            cid,
            problem_id
        )
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?
        .ok_or_else(|| {
            Error::msg("problem not in this contest").status_code(StatusCode::NOT_FOUND)
        })?;

        // for contest submissions, we don't check if problem is active
        // just verify the problem exists
        sqlx::query!(
            r#"
            SELECT id FROM problems WHERE id = $1
            "#,
            problem_id
        )
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?
        .ok_or_else(|| Error::msg("problem not found").status_code(StatusCode::NOT_FOUND))?;
    } else {
        // for normal submissions, check if problem exists and is active
        sqlx::query!(
            r#"
            SELECT id FROM problems WHERE id = $1 AND status = 'active'
            "#,
            problem_id
        )
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?
        .ok_or_else(|| Error::msg("problem not found").status_code(StatusCode::NOT_FOUND))?;
    }

    let submission = sqlx::query!(
        r#"
        INSERT INTO submissions (user_id, contest_id, problem_id, lang, result)
        VALUES ($1, $2, $3, $4, 'pending')
        RETURNING id, created_at
        "#,
        claims.sub,
        contest_id,
        problem_id,
        p.lang.to_string()
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    let code_for_judge = p.code.clone();
    let submission_code = SubmissionCode { code: p.code };

    state
        .write_submission_code(submission.id, &submission_code)
        .await?;

    let problem_limits = sqlx::query!(
        r#"
        SELECT time_limit, mem_limit FROM problems WHERE id = $1
        "#,
        problem_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;

    let test_case_records = sqlx::query!(
        r#"
        SELECT id FROM test_cases WHERE problem_id = $1 ORDER BY id
        "#,
        problem_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?;
    let mut test_cases = Vec::new();
    for record in test_case_records {
        let test_case_data = state.read_test_cases(record.id).await?;
        test_cases.push(TestCase {
            id: record.id,
            data: test_case_data,
        });
    }
    let task = JudgeTask {
        submission_id: submission.id,
        lang: p.lang,
        code: code_for_judge,
        time_limit: problem_limits.time_limit,
        memory_limit: problem_limits.mem_limit,
        test_cases,
    };
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = state_clone.submit_judge_task(task).await {
            tracing::error!("Failed to submit judge task: {:?}", e);

            if let Err(update_err) = sqlx::query!(
                r#"
                UPDATE submissions SET result = 'unknown_error', updated_at = NOW() WHERE id = $1
                "#,
                submission.id
            )
            .execute(&state_clone.pool)
            .await
            {
                tracing::error!("Failed to update submission status: {:?}", update_err);
            }

            // Update ranking cache if this is a contest submission
            // UnknownError is treated as a failed attempt
            if let Some(contest_id) = contest_id {
                if let Err(e) = crate::route::contests::ranking_cache::update_ranking_on_submission(
                    &state,
                    contest_id,
                    claims.sub,
                    problem_id,
                    SubmissionResult::UnknownError,
                    submission.created_at,
                )
                .await
                {
                    tracing::error!("Failed to update ranking cache: {:?}", e);
                    // Don't fail the whole operation if cache update fails
                }
            }
        }
    });

    Ok(Json(SubmitResponse {
        submission_id: submission.id,
    }))
}

#[derive(Serialize, Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListSubmissionsQuery {
    page: Option<i64>,
    page_size: Option<i64>,
    contest_id: Option<i32>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubmissionListItem {
    submission_id: i32,
    user_id: i32,
    username: String,
    problem_id: i32,
    problem_name: String,
    lang: String,
    result: SubmissionResult,
    time_consumption: Option<i32>,
    mem_consumption: Option<i32>,
    created_at: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListSubmissionsResponse {
    submissions: Vec<SubmissionListItem>,
    total: i64,
}

#[utoipa::path(
    get,
    path = "/api/problems/{problem_id}/submissions",
    params(
        ("problem_id" = i32, Path, description = "Problem ID"),
        ListSubmissionsQuery
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = ListSubmissionsResponse),
    ),
    tag = "problem"
)]
async fn list_submissions(
    state: State,
    claims: Extension<Claims>,
    Path(problem_id): Path<i32>,
    Query(q): Query<ListSubmissionsQuery>,
) -> Result<Json<ListSubmissionsResponse>> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let requester_role = role_of_claims(&state.pool, &claims).await?;

    // to unify records returnd by if branches
    #[derive(Debug)]
    struct SubmissionWithDetails {
        id: i32,
        user_id: i32,
        problem_id: i32,
        lang: String,
        result: SubmissionResult,
        time_consumption: Option<i32>,
        mem_consumption: Option<i32>,
        created_at: Option<DateTime<Utc>>,
        username: String,
        problem_name: String,
    }
    let (total, submissions) = if requester_role == UserRole::Admin
        || requester_role == UserRole::Teacher
    {
        let total: i64 = if let Some(cid) = q.contest_id {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM submissions WHERE problem_id = $1 AND contest_id = $2",
                problem_id,
                cid
            )
            .fetch_one(&state.pool)
            .await?
            .unwrap_or(0)
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM submissions WHERE problem_id = $1",
                problem_id
            )
            .fetch_one(&state.pool)
            .await?
            .unwrap_or(0)
        };

        let submissions = if let Some(cid) = q.contest_id {
            sqlx::query_as!(
                SubmissionWithDetails,
                r#"
                SELECT s.id, s.user_id, s.problem_id, s.lang, 
                    s.result as "result: SubmissionResult",
                    s.time_consumption, s.mem_consumption, s.created_at,
                    u.username, p.name as problem_name
                FROM submissions s
                JOIN users u ON s.user_id = u.id
                JOIN problems p ON s.problem_id = p.id
                WHERE s.problem_id = $1 AND s.contest_id = $2
                ORDER BY s.created_at DESC
                LIMIT $3 OFFSET $4
                "#,
                problem_id,
                cid,
                page_size,
                offset
            )
            .fetch_all(&state.pool)
            .await?
        } else {
            sqlx::query_as!(
                SubmissionWithDetails,
                r#"
                SELECT s.id, s.user_id, s.problem_id, s.lang, 
                    s.result as "result: SubmissionResult",
                    s.time_consumption, s.mem_consumption, s.created_at,
                    u.username, p.name as problem_name
                FROM submissions s
                JOIN users u ON s.user_id = u.id
                JOIN problems p ON s.problem_id = p.id
                WHERE s.problem_id = $1
                ORDER BY s.created_at DESC
                LIMIT $2 OFFSET $3
                "#,
                problem_id,
                page_size,
                offset
            )
            .fetch_all(&state.pool)
            .await?
        };
        (total, submissions)
    } else {
        let total: i64 = if let Some(cid) = q.contest_id {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM submissions WHERE problem_id = $1 AND user_id = $2 AND contest_id = $3",
                problem_id,
                claims.sub,
                cid
            )
            .fetch_one(&state.pool)
            .await?
            .unwrap_or(0)
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM submissions WHERE problem_id = $1 AND user_id = $2",
                problem_id,
                claims.sub
            )
            .fetch_one(&state.pool)
            .await?
            .unwrap_or(0)
        };

        let submissions = if let Some(cid) = q.contest_id {
            sqlx::query_as!(
                SubmissionWithDetails,
                r#"
                SELECT s.id, s.user_id, s.problem_id, s.lang, 
                    s.result as "result: SubmissionResult",
                    s.time_consumption, s.mem_consumption, s.created_at,
                    u.username, p.name as problem_name
                FROM submissions s
                JOIN users u ON s.user_id = u.id
                JOIN problems p ON s.problem_id = p.id
                WHERE s.problem_id = $1 AND s.user_id = $2 AND s.contest_id = $3
                ORDER BY s.created_at DESC
                LIMIT $4 OFFSET $5
                "#,
                problem_id,
                claims.sub,
                cid,
                page_size,
                offset
            )
            .fetch_all(&state.pool)
            .await?
        } else {
            sqlx::query_as!(
                SubmissionWithDetails,
                r#"
                SELECT s.id, s.user_id, s.problem_id, s.lang, 
                    s.result as "result: SubmissionResult",
                    s.time_consumption, s.mem_consumption, s.created_at,
                    u.username, p.name as problem_name
                FROM submissions s
                JOIN users u ON s.user_id = u.id
                JOIN problems p ON s.problem_id = p.id
                WHERE s.problem_id = $1 AND s.user_id = $2
                ORDER BY s.created_at DESC
                LIMIT $3 OFFSET $4
                "#,
                problem_id,
                claims.sub,
                page_size,
                offset
            )
            .fetch_all(&state.pool)
            .await?
        };
        (total, submissions)
    };

    let submission_list: Vec<SubmissionListItem> = submissions
        .into_iter()
        .map(|row| SubmissionListItem {
            submission_id: row.id,
            user_id: row.user_id,
            username: row.username,
            problem_id: row.problem_id,
            problem_name: row.problem_name,
            lang: row.lang,
            result: row.result,
            time_consumption: row.time_consumption,
            mem_consumption: row.mem_consumption,
            created_at: row
                .created_at
                .expect("created_at should not be null")
                .to_rfc3339(),
        })
        .collect();

    Ok(Json(ListSubmissionsResponse {
        submissions: submission_list,
        total,
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestCaseResultItem {
    test_case_id: i32,
    result: TestCaseJudgeResult,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetSubmissionResponse {
    submission_id: i32,
    user_id: i32,
    username: String,
    problem_id: i32,
    problem_name: String,
    lang: String,
    code: String,
    result: SubmissionResult,
    time_consumption: Option<i32>,
    mem_consumption: Option<i32>,
    test_case_results: Vec<TestCaseResultItem>,
    created_at: String,
}

#[utoipa::path(
    get,
    path = "/api/problems/{problem_id}/submissions/{submission_id}",
    params(
        ("problem_id" = i32, Path),
        ("submission_id" = i32, Path)
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = GetSubmissionResponse),
    ),
    tag = "problem"
)]
async fn get_submission(
    state: State,
    claims: Extension<Claims>,
    Path((problem_id, submission_id)): Path<(i32, i32)>,
) -> Result<Json<GetSubmissionResponse>> {
    check_permission(
        &state.pool,
        &claims,
        Action::GetSubmission,
        Resource::Submission(submission_id),
    )
    .await?;

    let submission = sqlx::query!(
        r#"
        SELECT s.id, s.user_id, s.problem_id, s.lang, 
               s.result as "result: SubmissionResult",
               s.time_consumption, s.mem_consumption, s.created_at,
               u.username, p.name as problem_name
        FROM submissions s
        JOIN users u ON s.user_id = u.id
        JOIN problems p ON s.problem_id = p.id
        WHERE s.id = $1 AND s.problem_id = $2
        "#,
        submission_id,
        problem_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("submission not found").status_code(StatusCode::NOT_FOUND))?;

    let submission_code = state.read_submission_code(submission_id).await?;

    let test_case_results = sqlx::query!(
        r#"
        SELECT test_case_id, result as "result: TestCaseJudgeResult"
        FROM submission_test_cases
        WHERE submission_id = $1
        ORDER BY test_case_id
        "#,
        submission_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .into_iter()
    .map(|row| TestCaseResultItem {
        test_case_id: row.test_case_id,
        result: row.result,
    })
    .collect();

    Ok(Json(GetSubmissionResponse {
        submission_id: submission.id,
        user_id: submission.user_id,
        username: submission.username,
        problem_id: submission.problem_id,
        problem_name: submission.problem_name,
        lang: submission.lang,
        code: submission_code.code,
        result: submission.result,
        time_consumption: submission.time_consumption,
        mem_consumption: submission.mem_consumption,
        test_case_results,
        created_at: submission.created_at.to_rfc3339(),
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetAcStatusResponse {
    tried: bool,
    status: Option<SubmissionResult>,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
struct GetAcStatusQuery {
    contest_id: Option<i32>,
}

#[utoipa::path(
    get,
    path = "/api/problems/{problem_id}/ac-status",
    params(
        ("problem_id" = i32, Path),
        ("contest_id" = Option<i32>, Query, description = "Optional contest ID to filter submissions")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = GetAcStatusResponse),
    ),
    tag = "problem"
)]
async fn get_ac_status(
    state: State,
    claims: Extension<Claims>,
    Path(problem_id): Path<i32>,
    Query(params): Query<GetAcStatusQuery>,
) -> Result<Json<GetAcStatusResponse>> {
    let accepted_count: i64 = if let Some(contest_id) = params.contest_id {
        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) FROM submissions
            WHERE problem_id = $1 AND user_id = $2 AND contest_id = $3 AND result = 'accepted'
            "#,
            problem_id,
            claims.sub,
            contest_id
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?
        .unwrap_or(0)
    } else {
        sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) FROM submissions
            WHERE problem_id = $1 AND user_id = $2 AND result = 'accepted'
            "#,
            problem_id,
            claims.sub
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::msg(format!("database error: {}", e)))?
        .unwrap_or(0)
    };

    let status = if accepted_count > 0 {
        Some(SubmissionResult::Accepted)
    } else {
        if let Some(contest_id) = params.contest_id {
            sqlx::query_scalar!(
                r#"
                SELECT result as "result: SubmissionResult" FROM submissions
                WHERE problem_id = $1 AND user_id = $2 AND contest_id = $3
                ORDER BY created_at DESC
                LIMIT 1
                "#,
                problem_id,
                claims.sub,
                contest_id
            )
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| Error::msg(format!("database error: {}", e)))?
        } else {
            sqlx::query_scalar!(
                r#"
                SELECT result as "result: SubmissionResult" FROM submissions
                WHERE problem_id = $1 AND user_id = $2
                ORDER BY created_at DESC
                LIMIT 1
                "#,
                problem_id,
                claims.sub
            )
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| Error::msg(format!("database error: {}", e)))?
        }
    };

    Ok(Json(GetAcStatusResponse {
        tried: status.is_some(),
        status: status,
    }))
}
