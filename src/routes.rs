use std::sync::Arc;

use axum::{Router, extract::DefaultBodyLimit, routing::get};

use crate::{
    handlers::{create_paste_multipart, favicon, index, logo, show_paste, show_preview, show_raw_paste, usage},
    state::AppState,
};

pub fn app_router(state: Arc<AppState>, max_paste_size: usize) -> Router {
    Router::new()
        .merge(page_routes(max_paste_size))
        .merge(asset_routes())
        .merge(paste_routes())
        .with_state(state)
}

fn page_routes(max_paste_size: usize) -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(index).post(create_paste_multipart))
        .route("/usage", get(usage))
        .layer(DefaultBodyLimit::max(max_paste_size))
}

fn asset_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/favicon.ico", get(favicon))
        .route("/logo.png", get(logo))
}

fn paste_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{id}/preview.png", get(show_preview))
        .route("/{id}/raw", get(show_raw_paste))
        .route("/{paste_ref}", get(show_paste))
}
