use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::Claims;
use crate::error::{Error, Result};
use theoj_common::bail;

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
    CreateProblem,
    PutProblem,
    DeleteProblem,
    GetTestCases,
    AddTestCases,
    CreateSolution,
    DeleteSolution,
    GetSubmission,
    CreateContest,
    PutContest,
    DeleteContest,
    ViewOverallRanking,
    CreateTrainingPlan,
    PutTrainingPlan,
    DeleteTrainingPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resource {
    Global,
    User(i32),
    Problem(i32),
    Solution(i32),
    Submission(i32),
    Contest(i32),
    TrainingPlan(i32),
}

impl Resource {
    pub async fn owner_id(self, pool: &sqlx::PgPool) -> Result<i32> {
        match self {
            Resource::Global => Ok(1),
            Resource::User(id) => Ok(id),
            Resource::Problem(_) => Ok(-1),
            Resource::Solution(id) => {
                let result = sqlx::query_scalar!("SELECT author FROM solutions WHERE id = $1", id)
                    .fetch_one(pool)
                    .await?;

                Ok(result)
            }
            Resource::Submission(id) => {
                let result =
                    sqlx::query_scalar!("SELECT user_id FROM submissions WHERE id = $1", id)
                        .fetch_one(pool)
                        .await?;

                Ok(result)
            }
            Resource::Contest(id) => {
                let result =
                    sqlx::query_scalar!("SELECT creator_id FROM contests WHERE id = $1", id)
                        .fetch_one(pool)
                        .await?;

                Ok(result)
            }
            Resource::TrainingPlan(id) => {
                let result =
                    sqlx::query_scalar!("SELECT creator_id FROM training_plans WHERE id = $1", id)
                        .fetch_one(pool)
                        .await?;

                Ok(result)
            }
        }
    }
}

pub async fn role_of_claims(pool: &sqlx::PgPool, claims: &Claims) -> Result<UserRole> {
    let user_role = match claims.sub {
        -1 => UserRole::Guest,
        _ => {
            sqlx::query!(
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
            .ok_or_else(|| {
                Error::msg("current user not found").status_code(StatusCode::UNAUTHORIZED)
            })?
            .user_role
        }
    };

    Result::Ok(user_role)
}

pub async fn check_permission(
    pool: &sqlx::PgPool,
    claims: &Claims,
    action: Action,
    resource: Resource,
) -> Result<()> {
    let user_role = role_of_claims(pool, claims).await?;

    let has_permission = match (user_role, action, resource) {
        (_, _, Resource::TrainingPlan(0)) => false,

        (UserRole::Admin, _, _) => true,
        (_, Action::GetRole, _) => true,
        (_, Action::GetProfile, _) => true,
        (_, Action::PutProfile, Resource::User(id_to_put)) => claims.sub == id_to_put,
        (_, Action::DeleteUser, Resource::User(id_to_del)) => claims.sub == id_to_del,

        (UserRole::Teacher, Action::CreateProblem, _) => true,
        (UserRole::Teacher, Action::PutProblem, _) => true,
        (UserRole::Teacher, Action::DeleteProblem, _) => true,

        (UserRole::Teacher, Action::AddTestCases, _) => true,
        (UserRole::Teacher, Action::GetTestCases, _) => true,

        (UserRole::Teacher, Action::CreateSolution, _) => true,
        (UserRole::Teacher, Action::DeleteSolution, solution) => {
            claims.sub == solution.owner_id(pool).await?
        }

        (UserRole::Teacher, Action::CreateContest, _) => true,
        (UserRole::Teacher, Action::PutContest, contest) => {
            claims.sub == contest.owner_id(pool).await?
        }
        (UserRole::Teacher, Action::DeleteContest, contest) => {
            claims.sub == contest.owner_id(pool).await?
        }
        (UserRole::Teacher, Action::ViewOverallRanking, _) => true,

        (UserRole::Teacher, Action::GetSubmission, _) => true,
        (UserRole::Student, Action::GetSubmission, submission) => {
            claims.sub == submission.owner_id(pool).await?
        }

        (UserRole::Teacher, Action::CreateTrainingPlan, _) => true,
        (UserRole::Teacher, Action::PutTrainingPlan, training_plan) => {
            claims.sub == training_plan.owner_id(pool).await?
        }
        (UserRole::Teacher, Action::DeleteTrainingPlan, training_plan) => {
            claims.sub == training_plan.owner_id(pool).await?
        }
        _ => false,
    };

    if !has_permission {
        bail!(@FORBIDDEN "insufficient permissions for this operation");
    }

    Ok(())
}
