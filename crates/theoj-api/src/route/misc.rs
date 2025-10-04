use std::sync::Arc;

use crate::Result;
use axum::{Json, Router};
use serde::Serialize;

use crate::AppState;

pub fn top_routes() -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
        .route("/ping", get(ping))
        .route("/version", get(version))
}

async fn ping() -> Result<String> {
    Ok("pong".to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionInfo {
    api_version: String,
}
async fn version() -> Result<Json<VersionInfo>> {
    let version_info = VersionInfo {
        api_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    Ok(Json(version_info))
}
