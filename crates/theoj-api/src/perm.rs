use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::Claims;
use crate::bail;
use crate::error::{Error, Result};

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "user_role_enum")]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub(crate) enum UserRole {
    Admin,
    Teacher,
    Student,
    Guest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    PutRole,
    GetRole,
    PutProfile,
    GetProfile,
    DeleteUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resource {
    Global,
    User(i32),
}

pub async fn check_permission(
    pool: &sqlx::PgPool,
    claims: &Claims,
    action: Action,
    resource: Resource,
) -> Result<()> {
    let current_user = sqlx::query!(
        r#"
        SELECT user_role as "user_role: UserRole"
        FROM users
        WHERE id = $1
        "#,
        claims.sub
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::msg(format!("database error: {}", e)))?
    .ok_or_else(|| Error::msg("current user not found").status_code(StatusCode::UNAUTHORIZED))?;

    let has_permission = match (current_user.user_role, action, resource) {
        (UserRole::Admin, _, _) => true,
        (_, Action::GetRole, _) => true,
        (_, Action::GetProfile, _) => true,
        (_, Action::PutProfile, Resource::User(id_to_put)) => claims.sub == id_to_put,
        (_, Action::DeleteUser, Resource::User(id_to_del)) => claims.sub == id_to_del,
        _ => false,
    };

    if !has_permission {
        bail!(@FORBIDDEN "insufficient permissions for this operation");
    }

    Ok(())
}
