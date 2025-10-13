use std::sync::Arc;

use axum::{Extension, Json, Router, extract::Path, http::StatusCode, middleware};
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    AppState, Result, State,
    auth::{Claims, generate_jwt_token, hash_password, jwt_auth_middleware, verify_password},
    bail,
    error::Error,
    perm::{Action, Resource, UserRole, check_permission},
};

pub fn top_routes() -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
        .route("/{user_id}/role", put(put_role))
        .route("/{user_id}/role", get(get_role))
        .layer(middleware::from_fn_with_state(state, jwt_auth_middleware))
}

fn is_valid_email(email: &str) -> bool {
    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    email_regex.is_match(email)
}
fn is_all_digit(phone: &str) -> bool {
    phone.chars().all(|c| c.is_ascii_digit())
}

#[derive(Debug, sqlx::Type, PartialEq)]
#[sqlx(type_name = "user_status_enum")]
#[sqlx(rename_all = "lowercase")]
pub enum UserStatus {
    Active,
    Inactive,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterRequest {
    phone: String,
    email: String,
    username: String,
    user_code: String,
    password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterResponse {
    user_id: String,
    token: String,
}

#[utoipa::path(
    post,
    path = "/api/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Register an account", body = RegisterResponse)
    ),
    tag = "user"
)]
async fn register(state: State, Json(p): Json<RegisterRequest>) -> Result<Json<RegisterResponse>> {
    if p.phone.is_empty()
        || p.email.is_empty()
        || p.username.is_empty()
        || p.user_code.is_empty()
        || p.password.is_empty()
    {
        bail!(@BAD_REQUEST "all fields are required");
    }

    if !is_valid_email(&p.email) {
        bail!(@BAD_REQUEST "invalid email");
    }
    if !is_all_digit(&p.phone) {
        bail!(@BAD_REQUEST "invalid phone");
    }
    if !is_all_digit(&p.user_code) {
        bail!(@BAD_REQUEST "invalid user code");
    }

    let password_hash = hash_password(p.password)?;

    let user_id: i32 = sqlx::query_scalar!(
        r#"
    INSERT INTO users (phone, email, username, user_code, user_role, password, status)
    VALUES ($1, $2, $3, $4, $5, $6, 'active')
    RETURNING id
    "#,
        p.phone,
        p.email,
        p.username,
        p.user_code,
        UserRole::Student as UserRole,
        password_hash
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return Error::msg("phone, email, username, or user_code already exists")
                    .status_code(StatusCode::BAD_REQUEST);
            }
        }
        Error::msg(format!("database error: {}", e))
    })?;

    let token = generate_jwt_token(
        &user_id,
        state.config.jwt_expiry,
        state.config.jwt_secret.clone(),
    )
    .map_err(|e| Error::msg(format!("Token generation failed: {}", e)))?;

    Ok(Json(RegisterResponse {
        user_id: user_id.to_string(),
        token,
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoginRequest {
    /// phone or email
    identifier: String,
    password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoginResponse {
    user_id: String,
    token: String,
}

#[utoipa::path(
    post,
    path = "/api/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login to account", body = LoginResponse)
    ),
    tag = "user"
)]
async fn login(state: State, Json(p): Json<LoginRequest>) -> Result<Json<LoginResponse>> {
    if p.identifier.is_empty() || p.password.is_empty() {
        bail!("identifier and password are required");
    }

    let user = sqlx::query!(
        r#"
    SELECT id, password, status as "status: UserStatus"
    FROM users
    WHERE username = $1 OR phone = $1 OR email = $1
    "#,
        p.identifier
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("invalid credentials").status_code(StatusCode::UNAUTHORIZED))?;

    if user.status != UserStatus::Active {
        bail!("account is not active");
    }

    verify_password(p.password, user.password)?;

    let token = generate_jwt_token(
        &user.id,
        state.config.jwt_expiry,
        state.config.jwt_secret.clone(),
    )
    .map_err(|e| Error::msg(format!("token generation failed: {}", e)))?;

    Ok(Json(LoginResponse {
        user_id: user.id.to_string(),
        token,
    }))
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PutRoleRequest {
    user_role: UserRole,
}

#[utoipa::path(
    put,
    path = "/api/users/{user_id}/role",
    request_body = PutRoleRequest,
    params(
        ("user_id" = String, Path, description = "User ID to update role")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Update user role successfully", body = ())
    ),
    tag = "user",
)]
async fn put_role(
    state: State,
    claims: Extension<Claims>,
    Path(user_id): Path<String>,
    Json(p): Json<PutRoleRequest>,
) -> Result<()> {
    check_permission(
        &state.pool,
        &claims,
        Action::PutRole,
        Resource::User(user_id.parse().unwrap()),
    )
    .await
    .unwrap();

    let user_id_int: i32 = user_id
        .parse()
        .map_err(|_| Error::msg("invalid user_id").status_code(StatusCode::BAD_REQUEST))?;

    let _updated = sqlx::query!(
        r#"
        UPDATE users
        SET user_role = $1
        WHERE id = $2
        RETURNING id
        "#,
        p.user_role as UserRole,
        user_id_int
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("user not found").status_code(StatusCode::NOT_FOUND))?;

    Ok(())
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GetRoleResponse {
    role: UserRole,
}

#[utoipa::path(
    get,
    path = "/api/users/{user_id}/role",
    params(
        ("user_id" = String, Path, description = "User ID to get role")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "User role get successfully", body = GetRoleResponse)
    ),
    tag = "user",
)]
async fn get_role(
    state: State,
    claims: Extension<Claims>,
    Path(user_id): Path<String>,
) -> Result<Json<GetRoleResponse>> {
    check_permission(
        &state.pool,
        &claims,
        Action::GetRole,
        Resource::User(user_id.parse().unwrap()),
    )
    .await
    .unwrap();

    let user_id_int: i32 = user_id
        .parse()
        .map_err(|_| Error::msg("invalid user_id").status_code(StatusCode::BAD_REQUEST))?;

    println!("{user_id_int}");

    let role = sqlx::query!(
        r#"
        SELECT user_role as "user_role: UserRole" FROM users
        WHERE id = $1
        "#,
        user_id_int
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| {
        println!("111");
        Error::msg("user not found").status_code(StatusCode::NOT_FOUND)
    })?
    .user_role;

    Ok(Json(GetRoleResponse { role }))
}
