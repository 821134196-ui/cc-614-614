use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct OrderRow {
    pub id: i64,
    pub order_no: String,
    pub customer_name: String,
    pub customer_phone: String,
    pub fish_species: String,
    pub sale_weight_grams: i64,
    pub unit_price_cents: i64,
    pub status: String,
    pub created_at: String,
    pub delivered_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WeighingRow {
    pub id: i64,
    pub order_id: i64,
    pub scale_device_id: String,
    pub weight_grams: i64,
    pub kind: String,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BoxRow {
    pub id: i64,
    pub box_no: String,
    pub location: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PickupCodeRow {
    pub id: i64,
    pub code: String,
    pub order_id: i64,
    pub box_id: i64,
    pub status: String,
    pub created_at: String,
    pub used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SmsRow {
    pub id: i64,
    pub phone: String,
    pub content: String,
    pub created_at: String,
}

/// 转箱记录（含新旧箱号，供前台展示）
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TransferView {
    pub id: i64,
    pub order_id: i64,
    pub from_box_no: String,
    pub to_box_no: String,
    pub from_location: String,
    pub to_location: String,
    pub operator: String,
    pub created_at: String,
}

/// 订单完整视图：订单 + 取货码 + 当前鱼箱 + 全部称重记录 + 转箱历史
#[derive(Debug, Serialize)]
pub struct OrderDetail {
    pub order: OrderRow,
    pub pickup_code: Option<PickupCodeRow>,
    pub current_box: Option<BoxRow>,
    pub weighings: Vec<WeighingRow>,
    pub transfers: Vec<TransferView>,
}

/// 订单列表行
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrderListItem {
    pub id: i64,
    pub order_no: String,
    pub customer_name: String,
    pub customer_phone: String,
    pub fish_species: String,
    pub sale_weight_grams: i64,
    pub status: String,
    pub created_at: String,
    pub delivered_at: Option<String>,
    pub pickup_code: Option<String>,
    pub code_status: Option<String>,
    pub box_no: Option<String>,
    pub location: Option<String>,
}

/// 鱼箱列表行（含当前绑定订单）
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BoxListItem {
    pub id: i64,
    pub box_no: String,
    pub location: String,
    pub status: String,
    pub order_id: Option<i64>,
    pub order_no: Option<String>,
}
