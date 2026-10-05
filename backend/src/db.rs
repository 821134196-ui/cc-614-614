use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

fn connect_options(opts: SqliteConnectOptions) -> SqliteConnectOptions {
    opts.create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(10))
        .foreign_keys(true)
}

async fn build_pool(opts: SqliteConnectOptions) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(connect_options(opts))
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    seed_boxes(&pool).await?;
    Ok(pool)
}

/// 以 sqlite:// URL 建立连接池（生产入口使用）
pub async fn setup_database(url: &str) -> Result<SqlitePool, sqlx::Error> {
    build_pool(SqliteConnectOptions::from_str(url)?).await
}

/// 以文件路径建立连接池（测试使用，文件库 + 多连接才能真实验证并发交付）
pub async fn setup_database_file(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    build_pool(SqliteConnectOptions::new().filename(path)).await
}

/// 预置编号鱼箱（幂等）
async fn seed_boxes(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for prefix in ["A", "B", "C"] {
        for n in 1..=8 {
            sqlx::query("INSERT OR IGNORE INTO boxes (box_no, location) VALUES (?, '空箱架')")
                .bind(format!("{prefix}-{n:02}"))
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}
