use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::error::{ApiError, ApiResult};
use crate::mock;
use crate::models::*;

// ---------- 请求体 ----------

#[derive(Deserialize)]
pub struct CreateOrderReq {
    pub customer_name: String,
    pub customer_phone: Option<String>,
    pub fish_type: String,
    pub device_id: String,
    pub box_code: String,
    pub slot_code: String,
    /// 手工录入重量；为空则由电子秤设备读数
    pub weight: Option<f64>,
}

#[derive(Deserialize)]
pub struct TransferReq {
    pub to_box_code: String,
    pub to_slot_code: String,
    pub operator: String,
}

#[derive(Deserialize)]
pub struct ReweighReq {
    pub device_id: String,
    pub weight: Option<f64>,
}

// ---------- 内部工具 ----------

/// 订单完整视图：当前箱号/箱位（转箱后即为最新）、全部称重记录、转箱历史、取货码
async fn order_detail(pool: &SqlitePool, order_id: i64) -> ApiResult<Value> {
    let order = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = ?")
        .bind(order_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found("订单不存在"))?;
    let box_row = sqlx::query_as::<_, BoxRow>("SELECT * FROM boxes WHERE id = ?")
        .bind(order.box_id)
        .fetch_one(pool)
        .await?;
    let slot_row = sqlx::query_as::<_, SlotRow>("SELECT * FROM slots WHERE id = ?")
        .bind(order.slot_id)
        .fetch_one(pool)
        .await?;
    let records = sqlx::query_as::<_, WeighRecord>(
        "SELECT * FROM weigh_records WHERE order_id = ? ORDER BY id",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;
    let transfers = sqlx::query_as::<_, TransferView>(
        "SELECT t.id, t.operator, t.created_at,
                fb.code AS from_box, tb.code AS to_box,
                fs.code AS from_slot, ts.code AS to_slot
         FROM transfers t
         JOIN boxes fb ON fb.id = t.from_box_id
         JOIN boxes tb ON tb.id = t.to_box_id
         JOIN slots fs ON fs.id = t.from_slot_id
         JOIN slots ts ON ts.id = t.to_slot_id
         WHERE t.order_id = ? ORDER BY t.id",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;
    let code = sqlx::query_as::<_, PickupCode>(
        "SELECT * FROM pickup_codes WHERE order_id = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await?;

    Ok(json!({
        "id": order.id,
        "order_no": order.order_no,
        "customer_name": order.customer_name,
        "customer_phone": order.customer_phone,
        "fish_type": order.fish_type,
        "device_id": order.device_id,
        "sale_weight": order.sale_weight,
        "status": order.status,
        "created_at": order.created_at,
        "delivered_at": order.delivered_at,
        "box_code": box_row.code,
        "slot_code": slot_row.code,
        "pickup_code": code.map(|c| json!({"code": c.code, "status": c.status})),
        "weigh_records": records,
        "transfers": transfers,
    }))
}

async fn find_box(pool: &SqlitePool, code: &str) -> ApiResult<BoxRow> {
    sqlx::query_as::<_, BoxRow>("SELECT * FROM boxes WHERE code = ?")
        .bind(code)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("鱼箱 {code} 不存在")))
}

async fn find_slot(pool: &SqlitePool, code: &str) -> ApiResult<SlotRow> {
    sqlx::query_as::<_, SlotRow>("SELECT * FROM slots WHERE code = ?")
        .bind(code)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("箱位 {code} 不存在")))
}

// ---------- 称重入库 ----------

/// POST /api/orders
/// 称重：保存电子秤设备号与原始重量 → 放入编号鱼箱 → 记录箱位 → 绑定取货码 → 模拟短信通知
pub async fn create_order(
    State(pool): State<SqlitePool>,
    Json(req): Json<CreateOrderReq>,
) -> ApiResult<Json<Value>> {
    if req.customer_name.trim().is_empty() {
        return Err(ApiError::bad_request("顾客姓名不能为空"));
    }
    if req.fish_type.trim().is_empty() {
        return Err(ApiError::bad_request("鱼类品种不能为空"));
    }
    if !mock::SCALE_DEVICES.contains(&req.device_id.as_str()) {
        return Err(ApiError::bad_request(format!(
            "未知电子秤设备号 {}",
            req.device_id
        )));
    }
    let weight = match req.weight {
        Some(w) if w > 0.0 => (w * 100.0).round() / 100.0,
        Some(_) => return Err(ApiError::bad_request("重量必须为正数")),
        None => mock::read_scale(&req.device_id).expect("device checked above"),
    };

    let box_row = find_box(&pool, &req.box_code).await?;
    let slot_row = find_slot(&pool, &req.slot_code).await?;
    let order_no = mock::gen_order_no();
    let phone = req.customer_phone.clone().unwrap_or_default();

    let mut tx = crate::db::begin_write(&pool).await?;

    let order_id = sqlx::query(
        "INSERT INTO orders(order_no, customer_name, customer_phone, fish_type,
                            device_id, sale_weight, box_id, slot_id)
         VALUES (?,?,?,?,?,?,?,?)",
    )
    .bind(&order_no)
    .bind(&req.customer_name)
    .bind(&phone)
    .bind(&req.fish_type)
    .bind(&req.device_id)
    .bind(weight)
    .bind(box_row.id)
    .bind(slot_row.id)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();

    // 原始称重记录（设备号 + 原始重量）
    sqlx::query("INSERT INTO weigh_records(order_id, kind, device_id, weight) VALUES (?,'original',?,?)")
        .bind(order_id)
        .bind(&req.device_id)
        .bind(weight)
        .execute(&mut *tx)
        .await?;

    // 绑定取货码；若鱼箱已有有效取货码，唯一索引会拒绝（一箱一码）
    let mut bound_code: Option<String> = None;
    for _ in 0..5 {
        let candidate = mock::gen_pickup_code();
        let r = sqlx::query("INSERT INTO pickup_codes(code, order_id, box_id) VALUES (?,?,?)")
            .bind(&candidate)
            .bind(order_id)
            .bind(box_row.id)
            .execute(&mut *tx)
            .await;
        match r {
            Ok(_) => {
                bound_code = Some(candidate);
                break;
            }
            Err(e) => {
                let retryable = matches!(&e, sqlx::Error::Database(db)
                    if db.is_unique_violation() && db.message().contains("pickup_codes.code"));
                if !retryable {
                    return Err(e.into()); // 一箱一码冲突在此转为 409
                }
            }
        }
    }
    let code = bound_code.ok_or_else(|| ApiError::internal("取货码生成失败，请重试"))?;

    // 模拟短信下发取货码
    let sms = format!(
        "【海鲜市场】取货码 {code}｜订单 {order_no}｜{} {:.2}kg｜鱼箱 {}｜箱位 {}。请凭码到柜台取货。",
        req.fish_type, weight, box_row.code, slot_row.code
    );
    sqlx::query("INSERT INTO sms_logs(phone, content) VALUES (?,?)")
        .bind(&phone)
        .bind(&sms)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(json!({
        "order": order_detail(&pool, order_id).await?,
        "pickup_code": code,
        "sms": sms,
    })))
}

// ---------- 取货柜台 ----------

/// GET /api/pickup/{code}
/// 柜台凭取货码查货：返回订单、当前最新箱号与箱位（转箱后自动跟随）、重量记录
pub async fn pickup_lookup(
    State(pool): State<SqlitePool>,
    Path(code): Path<String>,
) -> ApiResult<Json<Value>> {
    let pc = sqlx::query_as::<_, PickupCode>("SELECT * FROM pickup_codes WHERE code = ?")
        .bind(&code)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| ApiError::not_found("取货码不存在"))?;
    Ok(Json(json!({
        "code": pc.code,
        "code_status": pc.status,
        "order": order_detail(&pool, pc.order_id).await?,
    })))
}

/// POST /api/pickup/{code}/deliver
/// 核对无误后交付：取货码原子失效。两个窗口同时办理同一订单时，
/// UPDATE ... WHERE status='active' 只会有一个成功，另一个收到"已处理"提示。
pub async fn deliver(
    State(pool): State<SqlitePool>,
    Path(code): Path<String>,
) -> ApiResult<Json<Value>> {
    let mut tx = crate::db::begin_write(&pool).await?;
    let pc = sqlx::query_as::<_, PickupCode>("SELECT * FROM pickup_codes WHERE code = ?")
        .bind(&code)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::not_found("取货码不存在"))?;
    if pc.status != "active" {
        return Err(ApiError::conflict("该取货码已使用，订单已交付，请勿重复操作"));
    }
    // 原子比较并设置：并发下仅一个请求能把 active 改为 used
    let res = sqlx::query("UPDATE pickup_codes SET status = 'used' WHERE code = ? AND status = 'active'")
        .bind(&code)
        .execute(&mut *tx)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::conflict("该取货码已处理，订单已交付"));
    }
    let res = sqlx::query(
        "UPDATE orders SET status = 'delivered', delivered_at = datetime('now')
         WHERE id = ? AND status = 'stored'",
    )
    .bind(pc.order_id)
    .execute(&mut *tx)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::conflict("订单已处理"));
    }
    tx.commit().await?;
    Ok(Json(json!({
        "message": "交付成功，取货码已失效",
        "order": order_detail(&pool, pc.order_id).await?,
    })))
}

// ---------- 转箱 ----------

/// POST /api/orders/{id}/transfer
/// 记录新旧鱼箱、新旧箱位与经手人；取货码跟随新箱，柜台查码即见最新位置
pub async fn transfer(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(req): Json<TransferReq>,
) -> ApiResult<Json<Value>> {
    if req.operator.trim().is_empty() {
        return Err(ApiError::bad_request("经手人不能为空"));
    }
    let to_box = find_box(&pool, &req.to_box_code).await?;
    let to_slot = find_slot(&pool, &req.to_slot_code).await?;

    let mut tx = crate::db::begin_write(&pool).await?;
    let order = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::not_found("订单不存在"))?;
    if order.status != "stored" {
        return Err(ApiError::conflict("订单已交付，不能转箱"));
    }
    if to_box.id == order.box_id && to_slot.id == order.slot_id {
        return Err(ApiError::bad_request("新鱼箱/箱位与当前相同，无需转箱"));
    }

    // 有效取货码随箱迁移；目标箱若已被其他有效码占用，唯一索引拒绝（一箱一码）
    sqlx::query("UPDATE pickup_codes SET box_id = ? WHERE order_id = ? AND status = 'active'")
        .bind(to_box.id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE orders SET box_id = ?, slot_id = ? WHERE id = ?")
        .bind(to_box.id)
        .bind(to_slot.id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO transfers(order_id, from_box_id, to_box_id, from_slot_id, to_slot_id, operator)
         VALUES (?,?,?,?,?,?)",
    )
    .bind(id)
    .bind(order.box_id)
    .bind(to_box.id)
    .bind(order.slot_id)
    .bind(to_slot.id)
    .bind(&req.operator)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(json!({
        "message": "转箱完成",
        "order": order_detail(&pool, id).await?,
    })))
}

// ---------- 复称（重量争议） ----------

/// POST /api/orders/{id}/reweigh
/// 顾客异议时复称：新增一条 reweigh 记录，原始值与复核值都保留，销售重量不变
pub async fn reweigh(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(req): Json<ReweighReq>,
) -> ApiResult<Json<Value>> {
    let order = sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| ApiError::not_found("订单不存在"))?;
    if order.status != "stored" {
        return Err(ApiError::conflict("订单已交付，不能复称"));
    }
    if !mock::SCALE_DEVICES.contains(&req.device_id.as_str()) {
        return Err(ApiError::bad_request(format!(
            "未知电子秤设备号 {}",
            req.device_id
        )));
    }
    let weight = match req.weight {
        Some(w) if w > 0.0 => (w * 100.0).round() / 100.0,
        Some(_) => return Err(ApiError::bad_request("重量必须为正数")),
        None => mock::read_scale(&req.device_id).expect("device checked above"),
    };
    sqlx::query("INSERT INTO weigh_records(order_id, kind, device_id, weight) VALUES (?,'reweigh',?,?)")
        .bind(id)
        .bind(&req.device_id)
        .bind(weight)
        .execute(&pool)
        .await?;
    Ok(Json(json!({
        "message": "复称已记录，原始读数与复核读数均已保留",
        "order": order_detail(&pool, id).await?,
    })))
}

// ---------- 查询 ----------

pub async fn list_orders(State(pool): State<SqlitePool>) -> ApiResult<Json<Value>> {
    let rows = sqlx::query_as::<_, OrderListRow>(
        "SELECT o.id, o.order_no, o.customer_name, o.fish_type, o.sale_weight,
                o.status, o.created_at, o.delivered_at,
                b.code AS box_code, s.code AS slot_code,
                (SELECT pc.code FROM pickup_codes pc WHERE pc.order_id = o.id
                  ORDER BY pc.id DESC LIMIT 1) AS pickup_code
         FROM orders o
         JOIN boxes b ON b.id = o.box_id
         JOIN slots s ON s.id = o.slot_id
         ORDER BY o.id DESC",
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(json!({ "orders": rows })))
}

pub async fn get_order(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({ "order": order_detail(&pool, id).await? })))
}

pub async fn list_boxes(State(pool): State<SqlitePool>) -> ApiResult<Json<Value>> {
    let rows = sqlx::query_as::<_, BoxWithStatus>(
        "SELECT b.id, b.code,
                EXISTS(SELECT 1 FROM pickup_codes pc
                       WHERE pc.box_id = b.id AND pc.status = 'active') AS occupied
         FROM boxes b ORDER BY b.code",
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(json!({ "boxes": rows })))
}

pub async fn list_slots(State(pool): State<SqlitePool>) -> ApiResult<Json<Value>> {
    let rows = sqlx::query_as::<_, SlotRow>("SELECT * FROM slots ORDER BY code")
        .fetch_all(&pool)
        .await?;
    Ok(Json(json!({ "slots": rows })))
}

// ---------- 模拟设备接口 ----------

/// GET /api/mock/scale/{device}/read —— 模拟电子秤读数
pub async fn scale_read(Path(device): Path<String>) -> ApiResult<Json<Value>> {
    let weight = mock::read_scale(&device)
        .ok_or_else(|| ApiError::not_found(format!("电子秤 {device} 不在线")))?;
    Ok(Json(json!({ "device_id": device, "weight": weight, "unit": "kg" })))
}

/// GET /api/mock/sms/logs —— 模拟短信网关发件箱
pub async fn sms_logs(State(pool): State<SqlitePool>) -> ApiResult<Json<Value>> {
    let rows = sqlx::query_as::<_, SmsLog>("SELECT * FROM sms_logs ORDER BY id DESC LIMIT 50")
        .fetch_all(&pool)
        .await?;
    Ok(Json(json!({ "logs": rows })))
}
