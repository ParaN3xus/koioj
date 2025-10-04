use std::sync::Arc;

use axum::Router;

use crate::AppState;

pub fn top_routes() -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
}

pub fn routes() -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
}
