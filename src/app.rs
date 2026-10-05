use axum::{routing::get, routing::post, Router};
use sqlx::SqlitePool;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::handlers;

pub fn build_app(pool: SqlitePool) -> Router {
    let api = Router::new()
        .route("/orders", get(handlers::list_orders).post(handlers::create_order))
        .route("/orders/{id}", get(handlers::get_order))
        .route("/orders/{id}/transfer", post(handlers::transfer))
        .route("/orders/{id}/reweigh", post(handlers::reweigh))
        .route("/pickup/{code}", get(handlers::pickup_lookup))
        .route("/pickup/{code}/deliver", post(handlers::deliver))
        .route("/boxes", get(handlers::list_boxes))
        .route("/slots", get(handlers::list_slots))
        .route("/mock/scale/{device}/read", get(handlers::scale_read))
        .route("/mock/sms/logs", get(handlers::sms_logs))
        .with_state(pool);

    Router::new()
        .nest("/api", api)
        // 前端构建产物由后端托管，一条命令即可启动完整系统
        .fallback_service(
            ServeDir::new("frontend/dist")
                .fallback(ServeFile::new("frontend/dist/index.html")),
        )
        .layer(CorsLayer::permissive())
}
