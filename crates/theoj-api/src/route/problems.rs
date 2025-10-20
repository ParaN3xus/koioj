use std::sync::Arc;

use axum::{Router, middleware};

use crate::{AppState, auth::jwt_auth_middleware};

pub fn top_routes() -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
}

pub fn routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new().layer(middleware::from_fn_with_state(state, jwt_auth_middleware))
}
