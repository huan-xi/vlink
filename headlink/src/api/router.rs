use axum::Router;
use crate::api::state::AppState;

pub fn api() -> Router<AppState> {
    Router::new()
        .nest("/network", Router::new())
}