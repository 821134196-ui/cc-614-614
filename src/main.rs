use fish_market::{app, db};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "market.db".to_string());
    let pool = db::init_pool(&db_path).await;
    db::migrate(&pool).await;
    db::seed(&pool).await;

    let app = app::build_app(pool);
    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind failed");
    println!("海鲜市场活鱼暂存取货系统已启动: http://localhost:8080");
    axum::serve(listener, app).await.expect("server error");
}
