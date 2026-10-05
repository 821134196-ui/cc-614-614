use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Order {
    pub id: i64,
    pub order_no: String,
    pub customer_name: String,
    pub customer_phone: String,
    pub fish_type: String,
    pub device_id: String,
    pub sale_weight: f64,
    pub box_id: i64,
    pub slot_id: i64,
    pub status: String,
    pub created_at: String,
    pub delivered_at: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct BoxRow {
    pub id: i64,
    pub code: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct SlotRow {
    pub id: i64,
    pub code: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PickupCode {
    pub id: i64,
    pub code: String,
    pub order_id: i64,
    pub box_id: i64,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct WeighRecord {
    pub id: i64,
    pub order_id: i64,
    pub kind: String,
    pub device_id: String,
    pub weight: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TransferView {
    pub id: i64,
    pub operator: String,
    pub created_at: String,
    pub from_box: String,
    pub to_box: String,
    pub from_slot: String,
    pub to_slot: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct OrderListRow {
    pub id: i64,
    pub order_no: String,
    pub customer_name: String,
    pub fish_type: String,
    pub sale_weight: f64,
    pub status: String,
    pub created_at: String,
    pub delivered_at: Option<String>,
    pub box_code: String,
    pub slot_code: String,
    pub pickup_code: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct BoxWithStatus {
    pub id: i64,
    pub code: String,
    pub occupied: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct SmsLog {
    pub id: i64,
    pub phone: String,
    pub content: String,
    pub created_at: String,
}
