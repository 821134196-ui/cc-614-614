use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub struct ApiError(pub StatusCode, pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl ApiError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self(StatusCode::BAD_REQUEST, msg.into())
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self(StatusCode::NOT_FOUND, msg.into())
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self(StatusCode::CONFLICT, msg.into())
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, msg.into())
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db) = &e {
            if db.is_unique_violation() {
                let m = db.message();
                // SQLite 对部分唯一索引报错时给出列名（pickup_codes.box_id），
                // 对普通唯一约束给出列名或索引名，两者都要匹配
                if m.contains("one_active_code_per_box") || m.contains("pickup_codes.box_id") {
                    return ApiError::conflict("该鱼箱已绑定有效取货码，同一鱼箱不能同时绑定两张有效取货码");
                }
                if m.contains("one_active_code_per_order") || m.contains("pickup_codes.order_id") {
                    return ApiError::conflict("该订单已存在有效取货码");
                }
                if m.contains("pickup_codes.code") {
                    return ApiError::conflict("取货码冲突，请重试");
                }
                return ApiError::conflict(format!("唯一性约束冲突: {m}"));
            }
        }
        ApiError::internal(format!("数据库错误: {e}"))
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
