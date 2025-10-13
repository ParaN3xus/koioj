mod misc;
mod users;

#[cfg(feature = "embed-frontend")]
mod web;

use crate::AppState;
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
            .nest("/users", users::routes(state.clone())),
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
        users::put_role
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "users", description = "User management")
    )
)]
pub struct ApiDoc;

struct SecurityAddon;
impl Modify for SecurityAddon {
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
