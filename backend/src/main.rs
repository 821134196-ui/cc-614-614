use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info")),
        )
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://fishmarket.db".into());
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "frontend/dist".into());
    let listen_addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());

    let pool = fishmarket::db::setup_database(&database_url)
        .await
        .expect("数据库初始化失败");

    let app = fishmarket::app::build_app(pool).fallback_service(ServeDir::new(&static_dir));

    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .expect("端口绑定失败");
    tracing::info!("海鲜市场活鱼暂存取货系统已启动: http://localhost:8080 (静态目录: {static_dir})");
    axum::serve(listener, app).await.expect("服务运行失败");
}
