use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use sqlx::SqlitePool;

use crate::mock;
use crate::models::*;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

// ---------- 错误类型 ----------

pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self { status, code, message: message.into() }
    }
    fn bad_request(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "BAD_REQUEST", msg)
    }
    fn not_found(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", msg)
    }
    fn conflict(code: &'static str, msg: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, msg)
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!(error = ?e, "database error");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", "数据库内部错误")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({ "error": { "code": self.code, "message": self.message } })),
        )
            .into_response()
    }
}

// ---------- 公共查询 ----------

pub async fn order_detail(pool: &SqlitePool, order_id: i64) -> Result<OrderDetail, ApiError> {
    let order = sqlx::query_as::<_, OrderRow>("SELECT * FROM orders WHERE id = ?")
        .bind(order_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found("订单不存在"))?;
    let pickup_code =
        sqlx::query_as::<_, PickupCodeRow>("SELECT * FROM pickup_codes WHERE order_id = ?")
            .bind(order_id)
            .fetch_optional(pool)
            .await?;
    let current_box = match &pickup_code {
        Some(pc) => sqlx::query_as::<_, BoxRow>("SELECT * FROM boxes WHERE id = ?")
            .bind(pc.box_id)
            .fetch_optional(pool)
            .await?,
        None => None,
    };
    let weighings = sqlx::query_as::<_, WeighingRow>(
        "SELECT * FROM weighings WHERE order_id = ? ORDER BY id",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;
    let transfers = sqlx::query_as::<_, TransferView>(
        r#"SELECT t.id, t.order_id,
                  fb.box_no AS from_box_no, tb.box_no AS to_box_no,
                  t.from_location, t.to_location, t.operator, t.created_at
           FROM transfers t
           JOIN boxes fb ON fb.id = t.from_box_id
           JOIN boxes tb ON tb.id = t.to_box_id
           WHERE t.order_id = ? ORDER BY t.id"#,
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;
    Ok(OrderDetail { order, pickup_code, current_box, weighings, transfers })
}

/// 模拟短信网关：落 sms_logs 表 + 打日志
async fn send_sms<'e, E>(exec: E, phone: &str, content: &str) -> Result<(), ApiError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    sqlx::query("INSERT INTO sms_logs (phone, content) VALUES (?, ?)")
        .bind(phone)
        .bind(content)
        .execute(exec)
        .await?;
    tracing::info!(target: "mock_sms", phone, "发送模拟短信: {content}");
    Ok(())
}

// ---------- 称重入库 ----------

#[derive(Debug, Deserialize)]
pub struct CreateOrderReq {
    customer_name: String,
    customer_phone: String,
    fish_species: String,
    unit_price_cents: Option<i64>,
    scale_device_id: String,
    box_no: String,
    location: String,
    /// 前端从模拟电子秤读到的重量；缺省时后端模拟一次读数
    weight_grams: Option<i64>,
}

pub async fn create_order(
    State(s): State<AppState>,
    Json(req): Json<CreateOrderReq>,
) -> Result<Json<OrderDetail>, ApiError> {
    if req.customer_name.trim().is_empty()
        || req.customer_phone.trim().is_empty()
        || req.fish_species.trim().is_empty()
    {
        return Err(ApiError::bad_request("顾客姓名、电话和鱼种必填"));
    }
    if req.location.trim().is_empty() {
        return Err(ApiError::bad_request("暂存箱位必填"));
    }
    let weight = req
        .weight_grams
        .filter(|w| *w > 0)
        .unwrap_or_else(mock::random_weight_grams);

    // BEGIN IMMEDIATE：立即拿写锁，串行化鱼箱分配，避免并发抢同一箱
    let mut tx = s.pool.begin_with("BEGIN IMMEDIATE").await?;

    let bx = sqlx::query_as::<_, BoxRow>("SELECT * FROM boxes WHERE box_no = ?")
        .bind(req.box_no.trim())
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::bad_request("鱼箱不存在"))?;
    if bx.status != "idle" {
        return Err(ApiError::conflict(
            "BOX_BUSY",
            format!("鱼箱 {} 正在使用中，请选择其他鱼箱", bx.box_no),
        ));
    }

    let order_no = mock::gen_order_no();
    let order_id = sqlx::query(
        r#"INSERT INTO orders
           (order_no, customer_name, customer_phone, fish_species, sale_weight_grams, unit_price_cents)
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&order_no)
    .bind(req.customer_name.trim())
    .bind(req.customer_phone.trim())
    .bind(req.fish_species.trim())
    .bind(weight)
    .bind(req.unit_price_cents.unwrap_or(0))
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();

    // 原始称重记录：保存电子秤设备号与原始重量
    sqlx::query(
        "INSERT INTO weighings (order_id, scale_device_id, weight_grams, kind, note) VALUES (?, ?, ?, 'original', '销售原始称重')",
    )
    .bind(order_id)
    .bind(req.scale_device_id.trim())
    .bind(weight)
    .execute(&mut *tx)
    .await?;

    // 绑定取货码（撞码则重试；uq_active_code_per_box 兜底一箱一码）
    let mut code = String::new();
    for attempt in 0..6 {
        let candidate = mock::gen_pickup_code();
        let result =
            sqlx::query("INSERT INTO pickup_codes (code, order_id, box_id) VALUES (?, ?, ?)")
                .bind(&candidate)
                .bind(order_id)
                .bind(bx.id)
                .execute(&mut *tx)
                .await;
        match result {
            Ok(_) => {
                code = candidate;
                break;
            }
            Err(sqlx::Error::Database(e))
                if e.message().contains("pickup_codes.code") && attempt < 5 =>
            {
                continue; // 取货码撞了，换一张
            }
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                return Err(ApiError::conflict(
                    "BOX_BUSY",
                    "该鱼箱已绑定其他有效取货码，请选择其他鱼箱",
                ));
            }
            Err(e) => return Err(e.into()),
        }
    }

    sqlx::query("UPDATE boxes SET status = 'in_use', location = ? WHERE id = ?")
        .bind(req.location.trim())
        .bind(bx.id)
        .execute(&mut *tx)
        .await?;

    let sms = format!(
        "【海鲜市场】取货码 {code}：{} {:.2}kg，鱼箱 {}，暂存位 {}。请凭码到柜台取货。",
        req.fish_species.trim(),
        weight as f64 / 1000.0,
        bx.box_no,
        req.location.trim()
    );
    send_sms(&mut *tx, req.customer_phone.trim(), &sms).await?;

    tx.commit().await?;
    order_detail(&s.pool, order_id).await.map(Json)
}

// ---------- 取货柜台 ----------

/// 凭取货码查询：返回订单、最新箱号与箱位、称重与转箱记录
pub async fn lookup_by_code(
    State(s): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<OrderDetail>, ApiError> {
    let code = code.trim().to_uppercase();
    let pc = sqlx::query_as::<_, PickupCodeRow>("SELECT * FROM pickup_codes WHERE code = ?")
        .bind(&code)
        .fetch_optional(&s.pool)
        .await?
        .ok_or_else(|| ApiError::not_found("取货码不存在"))?;
    order_detail(&s.pool, pc.order_id).await.map(Json)
}

/// 交付：原子地把取货码 active -> used。
/// 两个窗口同时交付同一订单时，SQLite 写锁串行化，只有一个事务的 UPDATE 命中
/// status='active'，另一个 rows_affected=0 -> 409 已处理提示。
pub async fn deliver(
    State(s): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<OrderDetail>, ApiError> {
    let code = code.trim().to_uppercase();
    let mut tx = s.pool.begin_with("BEGIN IMMEDIATE").await?;

    let res = sqlx::query(
        "UPDATE pickup_codes SET status = 'used', used_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE code = ? AND status = 'active'",
    )
    .bind(&code)
    .execute(&mut *tx)
    .await?;

    if res.rows_affected() == 0 {
        let exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pickup_codes WHERE code = ?")
                .bind(&code)
                .fetch_one(&mut *tx)
                .await?;
        return Err(if exists > 0 {
            ApiError::conflict(
                "ALREADY_PROCESSED",
                "该取货码已处理（可能已在其他窗口交付），不能重复交付",
            )
        } else {
            ApiError::not_found("取货码不存在")
        });
    }

    let pc = sqlx::query_as::<_, PickupCodeRow>("SELECT * FROM pickup_codes WHERE code = ?")
        .bind(&code)
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE orders SET status = 'delivered', delivered_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(pc.order_id)
    .execute(&mut *tx)
    .await?;
    // 交付后鱼箱释放回空闲
    sqlx::query("UPDATE boxes SET status = 'idle' WHERE id = ?")
        .bind(pc.box_id)
        .execute(&mut *tx)
        .await?;

    let order = sqlx::query_as::<_, OrderRow>("SELECT * FROM orders WHERE id = ?")
        .bind(pc.order_id)
        .fetch_one(&mut *tx)
        .await?;
    let sms = format!("【海鲜市场】订单 {} 已交付完成，感谢惠顾。", order.order_no);
    send_sms(&mut *tx, &order.customer_phone, &sms).await?;

    tx.commit().await?;
    order_detail(&s.pool, pc.order_id).await.map(Json)
}

// ---------- 转箱 ----------

#[derive(Debug, Deserialize)]
pub struct TransferReq {
    to_box_no: String,
    to_location: String,
    operator: String,
}

pub async fn transfer(
    State(s): State<AppState>,
    Path(order_id): Path<i64>,
    Json(req): Json<TransferReq>,
) -> Result<Json<OrderDetail>, ApiError> {
    if req.to_box_no.trim().is_empty()
        || req.to_location.trim().is_empty()
        || req.operator.trim().is_empty()
    {
        return Err(ApiError::bad_request("新鱼箱编号、新箱位和经手人必填"));
    }

    let mut tx = s.pool.begin_with("BEGIN IMMEDIATE").await?;

    let order = sqlx::query_as::<_, OrderRow>("SELECT * FROM orders WHERE id = ?")
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::not_found("订单不存在"))?;
    if order.status != "stored" {
        return Err(ApiError::conflict("ORDER_DELIVERED", "订单已交付，不能转箱"));
    }

    let pc = sqlx::query_as::<_, PickupCodeRow>(
        "SELECT * FROM pickup_codes WHERE order_id = ? AND status = 'active'",
    )
    .bind(order_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::conflict("NO_ACTIVE_CODE", "订单没有有效取货码"))?;

    let from_box = sqlx::query_as::<_, BoxRow>("SELECT * FROM boxes WHERE id = ?")
        .bind(pc.box_id)
        .fetch_one(&mut *tx)
        .await?;
    if req.to_box_no.trim() == from_box.box_no {
        return Err(ApiError::bad_request("新鱼箱不能与当前鱼箱相同"));
    }
    let to_box = sqlx::query_as::<_, BoxRow>("SELECT * FROM boxes WHERE box_no = ?")
        .bind(req.to_box_no.trim())
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::bad_request("目标鱼箱不存在"))?;
    if to_box.status != "idle" {
        return Err(ApiError::conflict(
            "BOX_BUSY",
            format!("目标鱼箱 {} 正在使用中", to_box.box_no),
        ));
    }

    // 取货码改绑新箱（一箱一码由唯一索引兜底），新旧箱状态与箱位联动更新
    sqlx::query("UPDATE pickup_codes SET box_id = ? WHERE id = ? AND status = 'active'")
        .bind(to_box.id)
        .bind(pc.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE boxes SET status = 'idle' WHERE id = ?")
        .bind(from_box.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE boxes SET status = 'in_use', location = ? WHERE id = ?")
        .bind(req.to_location.trim())
        .bind(to_box.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        r#"INSERT INTO transfers (order_id, from_box_id, to_box_id, from_location, to_location, operator)
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(order_id)
    .bind(from_box.id)
    .bind(to_box.id)
    .bind(&from_box.location)
    .bind(req.to_location.trim())
    .bind(req.operator.trim())
    .execute(&mut *tx)
    .await?;

    let sms = format!(
        "【海鲜市场】订单 {} 已转箱：新鱼箱 {}，新箱位 {}。取货码 {} 不变。",
        order.order_no,
        to_box.box_no,
        req.to_location.trim(),
        pc.code
    );
    send_sms(&mut *tx, &order.customer_phone, &sms).await?;

    tx.commit().await?;
    order_detail(&s.pool, order_id).await.map(Json)
}

// ---------- 复称（重量异议） ----------

#[derive(Debug, Deserialize)]
pub struct ReweighReq {
    scale_device_id: String,
    /// 复核读数；缺省时后端模拟一次读数
    weight_grams: Option<i64>,
    note: Option<String>,
}

/// 顾客对重量有异议时复称：新增一条 kind='reweigh' 的称重记录。
/// 原始值与复核值都保留在 weighings 流水里，订单销售重量 sale_weight_grams 不变。
pub async fn reweigh(
    State(s): State<AppState>,
    Path(order_id): Path<i64>,
    Json(req): Json<ReweighReq>,
) -> Result<Json<OrderDetail>, ApiError> {
    let mut tx = s.pool.begin_with("BEGIN IMMEDIATE").await?;

    let order = sqlx::query_as::<_, OrderRow>("SELECT * FROM orders WHERE id = ?")
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::not_found("订单不存在"))?;
    if order.status != "stored" {
        return Err(ApiError::conflict("ORDER_DELIVERED", "订单已交付，不能复称"));
    }

    let weight = req
        .weight_grams
        .filter(|w| *w > 0)
        .unwrap_or_else(mock::random_weight_grams);
    let note = req
        .note
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "顾客异议复称".to_string());

    sqlx::query(
        "INSERT INTO weighings (order_id, scale_device_id, weight_grams, kind, note) VALUES (?, ?, ?, 'reweigh', ?)",
    )
    .bind(order_id)
    .bind(req.scale_device_id.trim())
    .bind(weight)
    .bind(note.trim())
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    order_detail(&s.pool, order_id).await.map(Json)
}

// ---------- 列表 / 详情 ----------

pub async fn list_orders(
    State(s): State<AppState>,
) -> Result<Json<Vec<OrderListItem>>, ApiError> {
    let items = sqlx::query_as::<_, OrderListItem>(
        r#"SELECT o.id, o.order_no, o.customer_name, o.customer_phone, o.fish_species,
                  o.sale_weight_grams, o.status, o.created_at, o.delivered_at,
                  pc.code AS pickup_code, pc.status AS code_status,
                  b.box_no, b.location
           FROM orders o
           LEFT JOIN pickup_codes pc ON pc.order_id = o.id
           LEFT JOIN boxes b ON b.id = pc.box_id
           ORDER BY o.id DESC"#,
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(items))
}

pub async fn get_order(
    State(s): State<AppState>,
    Path(order_id): Path<i64>,
) -> Result<Json<OrderDetail>, ApiError> {
    order_detail(&s.pool, order_id).await.map(Json)
}

#[derive(Debug, Deserialize)]
pub struct BoxFilter {
    status: Option<String>,
}

pub async fn list_boxes(
    State(s): State<AppState>,
    Query(f): Query<BoxFilter>,
) -> Result<Json<Vec<BoxListItem>>, ApiError> {
    const SQL: &str = r#"SELECT b.id, b.box_no, b.location, b.status,
                                pc.order_id, o.order_no
                         FROM boxes b
                         LEFT JOIN pickup_codes pc ON pc.box_id = b.id AND pc.status = 'active'
                         LEFT JOIN orders o ON o.id = pc.order_id"#;
    let items = match f.status.as_deref() {
        Some(st) => {
            sqlx::query_as::<_, BoxListItem>(&format!("{SQL} WHERE b.status = ? ORDER BY b.box_no"))
                .bind(st)
                .fetch_all(&s.pool)
                .await?
        }
        None => {
            sqlx::query_as::<_, BoxListItem>(&format!("{SQL} ORDER BY b.box_no"))
                .fetch_all(&s.pool)
                .await?
        }
    };
    Ok(Json(items))
}

#[derive(Debug, Deserialize)]
pub struct CreateBoxReq {
    box_no: String,
    location: Option<String>,
}

pub async fn create_box(
    State(s): State<AppState>,
    Json(req): Json<CreateBoxReq>,
) -> Result<Json<BoxRow>, ApiError> {
    if req.box_no.trim().is_empty() {
        return Err(ApiError::bad_request("鱼箱编号必填"));
    }
    let location = req.location.unwrap_or_else(|| "空箱架".into());
    let result = sqlx::query("INSERT INTO boxes (box_no, location) VALUES (?, ?)")
        .bind(req.box_no.trim())
        .bind(location.trim())
        .execute(&s.pool)
        .await;
    match result {
        Ok(r) => {
            let bx = sqlx::query_as::<_, BoxRow>("SELECT * FROM boxes WHERE id = ?")
                .bind(r.last_insert_rowid())
                .fetch_one(&s.pool)
                .await?;
            Ok(Json(bx))
        }
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(ApiError::conflict(
            "BOX_EXISTS",
            format!("鱼箱 {} 已存在", req.box_no.trim()),
        )),
        Err(e) => Err(e.into()),
    }
}

// ---------- 模拟接口：电子秤 / 短信 ----------

#[derive(Debug, Deserialize)]
pub struct ScaleQuery {
    device_id: Option<String>,
    /// 演示/测试时可强制指定读数
    weight: Option<i64>,
}

pub async fn scale_read(Query(q): Query<ScaleQuery>) -> Json<serde_json::Value> {
    let weight = q
        .weight
        .filter(|w| *w > 0)
        .unwrap_or_else(mock::random_weight_grams);
    Json(json!({
        "device_id": q.device_id.unwrap_or_else(|| "SCALE-01".into()),
        "weight_grams": weight,
        "unit": "g",
        "stable": true,
        "read_at": chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn list_sms(State(s): State<AppState>) -> Result<Json<Vec<SmsRow>>, ApiError> {
    let rows = sqlx::query_as::<_, SmsRow>("SELECT * FROM sms_logs ORDER BY id DESC LIMIT 50")
        .fetch_all(&s.pool)
        .await?;
    Ok(Json(rows))
}
