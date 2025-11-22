mod contests;
pub mod judge;
mod misc;
mod problems;
mod users;

#[cfg(feature = "embed-frontend")]
mod web;

use crate::{AppState, error::ErrorResponse};
use axum::Router;
use std::sync::Arc;
use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    use axum::routing::*;
    let mut router = Router::new().nest(
        "/api",
        Router::new()
            .merge(misc::top_routes())
            .merge(users::top_routes())
            .merge(problems::top_routes())
            .merge(contests::top_routes())
            .nest("/users", users::routes(state.clone()))
            .nest("/problems", problems::routes(state.clone()))
            .nest("/judge", judge::routes(state.clone()))
            .nest("/contests", contests::routes(state.clone())),
    );
    #[cfg(debug_assertions)]
    {
        use utoipa_swagger_ui::SwaggerUi;
        router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));
    };
    #[cfg(feature = "embed-frontend")]
    {
        router = router.merge(web::top_routes());
    }

    router
}

#[derive(OpenApi)]
#[openapi(
    paths(
        misc::ping,
        misc::version,
        users::register,
        users::login,
        users::get_role,
        users::put_role,
        users::get_profile,
        users::put_profile,
        users::change_password,
        users::delete_user,
        problems::get_problem,
        problems::list_solutions,
        problems::get_solution,
        problems::list_problems,
        problems::create_problem,
        problems::put_problem,
        problems::delete_problem,
        problems::add_test_cases,
        problems::get_test_cases,
        problems::create_solution,
        problems::delete_solution,
        problems::submit,
        problems::list_submissions,
        problems::get_submission,
        problems::get_ac_status,
    ),
    modifiers(&JWTAuthAddon),
    tags(
        (name = "health"),
        (name = "users"),
        (name = "problems"),
    ),
    components(
        schemas(ErrorResponse),
    )
)]
pub struct ApiDoc;

struct JWTAuthAddon;
impl Modify for JWTAuthAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            )
        }
    }
}
