use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::time::Duration;

pub async fn init_pool(path: &str) -> SqlitePool {
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
        .expect("failed to connect sqlite")
}

/// 写事务：BEGIN IMMEDIATE 在事务开始时就获取写锁，
/// 配合 busy_timeout 串行化并发写，避免"读后写"升级死锁（SQLITE_BUSY）
pub async fn begin_write(
    pool: &SqlitePool,
) -> sqlx::Result<sqlx::Transaction<'static, sqlx::Sqlite>> {
    pool.begin_with("BEGIN IMMEDIATE").await
}

const SCHEMA: &[&str] = &[
    // 编号鱼箱
    "CREATE TABLE IF NOT EXISTS boxes (
        id   INTEGER PRIMARY KEY AUTOINCREMENT,
        code TEXT NOT NULL UNIQUE
    )",
    // 暂存箱位
    "CREATE TABLE IF NOT EXISTS slots (
        id   INTEGER PRIMARY KEY AUTOINCREMENT,
        code TEXT NOT NULL UNIQUE
    )",
    // 销售订单：sale_weight 为原始销售重量，一经写入不再修改（争议走复称记录）
    "CREATE TABLE IF NOT EXISTS orders (
        id             INTEGER PRIMARY KEY AUTOINCREMENT,
        order_no       TEXT NOT NULL UNIQUE,
        customer_name  TEXT NOT NULL,
        customer_phone TEXT NOT NULL DEFAULT '',
        fish_type      TEXT NOT NULL,
        device_id      TEXT NOT NULL,
        sale_weight    REAL NOT NULL,
        box_id         INTEGER NOT NULL REFERENCES boxes(id),
        slot_id        INTEGER NOT NULL REFERENCES slots(id),
        status         TEXT NOT NULL DEFAULT 'stored' CHECK (status IN ('stored','delivered')),
        created_at     TEXT NOT NULL DEFAULT (datetime('now')),
        delivered_at   TEXT
    )",
    // 取货码：active / used
    "CREATE TABLE IF NOT EXISTS pickup_codes (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        code       TEXT NOT NULL UNIQUE,
        order_id   INTEGER NOT NULL REFERENCES orders(id),
        box_id     INTEGER NOT NULL REFERENCES boxes(id),
        status     TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','used')),
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    )",
    // 约束：同一鱼箱同一时间只能绑定一张有效取货码
    "CREATE UNIQUE INDEX IF NOT EXISTS one_active_code_per_box
        ON pickup_codes(box_id) WHERE status = 'active'",
    // 同一订单同一时间也只有一张有效取货码
    "CREATE UNIQUE INDEX IF NOT EXISTS one_active_code_per_order
        ON pickup_codes(order_id) WHERE status = 'active'",
    // 称重记录：original 原始读数 + reweigh 复核读数，全部保留
    "CREATE TABLE IF NOT EXISTS weigh_records (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        order_id   INTEGER NOT NULL REFERENCES orders(id),
        kind       TEXT NOT NULL CHECK (kind IN ('original','reweigh')),
        device_id  TEXT NOT NULL,
        weight     REAL NOT NULL,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    )",
    // 转箱记录：新旧鱼箱、新旧箱位、经手人
    "CREATE TABLE IF NOT EXISTS transfers (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        order_id     INTEGER NOT NULL REFERENCES orders(id),
        from_box_id  INTEGER NOT NULL REFERENCES boxes(id),
        to_box_id    INTEGER NOT NULL REFERENCES boxes(id),
        from_slot_id INTEGER NOT NULL REFERENCES slots(id),
        to_slot_id   INTEGER NOT NULL REFERENCES slots(id),
        operator     TEXT NOT NULL,
        created_at   TEXT NOT NULL DEFAULT (datetime('now'))
    )",
    // 模拟短信发件箱
    "CREATE TABLE IF NOT EXISTS sms_logs (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        phone      TEXT NOT NULL,
        content    TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    )",
];

pub async fn migrate(pool: &SqlitePool) {
    for stmt in SCHEMA {
        sqlx::query(stmt)
            .execute(pool)
            .await
            .expect("migration failed");
    }
}

/// 预置鱼箱与箱位（幂等）
pub async fn seed(pool: &SqlitePool) {
    for i in 1..=12 {
        sqlx::query("INSERT OR IGNORE INTO boxes(code) VALUES (?)")
            .bind(format!("BOX-{i:03}"))
            .execute(pool)
            .await
            .unwrap();
    }
    for prefix in ["A", "B"] {
        for i in 1..=8 {
            sqlx::query("INSERT OR IGNORE INTO slots(code) VALUES (?)")
                .bind(format!("{prefix}-{i:02}"))
                .execute(pool)
                .await
                .unwrap();
        }
    }
}
