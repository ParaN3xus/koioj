mod misc;
mod user;
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
            .merge(user::top_routes())
            .nest("/user", user::routes(state.clone())),
    );
    #[cfg(debug_assertions)]
    {
        use utoipa_swagger_ui::SwaggerUi;
        generate_openapi();
        router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));
    };
    router = router.merge(web::top_routes());

    router
}

#[derive(OpenApi)]
#[openapi(
    paths(
        misc::ping,
        misc::version,
        user::register,
        user::login
    ),
    components(schemas(
        misc::VersionResponse,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "user", description = "User management")
    )
)]
pub struct ApiDoc;

pub fn generate_openapi() {
    let output_path = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");

    std::fs::write(output_path, ApiDoc::openapi().to_pretty_json().unwrap())
        .expect("Failed to write openapi.json");

    tracing::debug!("openapi.json generated: {}", output_path);
}

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
