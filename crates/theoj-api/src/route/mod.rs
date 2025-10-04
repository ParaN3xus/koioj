mod misc;
mod user;

use std::sync::Arc;

use axum::Router;

use crate::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    use axum::routing::*;
    Router::new()
        .merge(misc::top_routes())
        .merge(user::top_routes())
        .nest("/route", user::routes())
}
