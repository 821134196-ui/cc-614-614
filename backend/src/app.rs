use axum::routing::{get, post};
use axum::Router;
use sqlx::SqlitePool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers::{self, AppState};

pub fn build_app(pool: SqlitePool) -> Router {
    let state = AppState { pool };
    Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/orders", get(handlers::list_orders).post(handlers::create_order))
        .route("/api/orders/{id}", get(handlers::get_order))
        .route("/api/orders/{id}/transfer", post(handlers::transfer))
        .route("/api/orders/{id}/reweigh", post(handlers::reweigh))
        .route("/api/pickup/{code}", get(handlers::lookup_by_code))
        .route("/api/pickup/{code}/deliver", post(handlers::deliver))
        .route("/api/boxes", get(handlers::list_boxes).post(handlers::create_box))
        .route("/api/mock/scale/read", get(handlers::scale_read))
        .route("/api/mock/sms", get(handlers::list_sms))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
