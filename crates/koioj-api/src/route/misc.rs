use crate::{AppState, Result};
use axum::{Json, Router};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

pub fn top_routes() -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
        .route("/ping", get(ping))
        .route("/version", get(version))
}

#[utoipa::path(
    get,
    path = "/api/ping",
    responses(
        (status = 200, body = String)
    ),
    tag = "health"
)]
async fn ping() -> Result<String> {
    Ok("pong".to_string())
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VersionResponse {
    api_version: String,
}

#[utoipa::path(
    get,
    path = "/api/version",
    responses(
        (status = 200, body = VersionResponse)
    ),
    tag = "health"
)]
async fn version() -> Result<Json<VersionResponse>> {
    let version_info = VersionResponse {
        api_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    Ok(Json(version_info))
}
