//! 端到端集成测试：直接对 Axum Router 发请求，覆盖三个关键业务保证：
//! 1. 转箱后凭取货码仍能查到最新箱号与箱位
//! 2. 重量争议复称后原始值与复核值都保留，销售重量不被修改
//! 3. 多窗口并发交付同一取货码，只成功一次，其余收到"已处理"提示

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn test_app() -> (Router, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let pool = fishmarket::db::setup_database_file(&dir.path().join("test.db"))
        .await
        .unwrap();
    (fishmarket::app::build_app(pool), dir)
}

async fn request(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let payload = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let res = app.clone().oneshot(builder.body(payload).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), 1 << 20).await.unwrap();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn get(app: &Router, uri: &str) -> (StatusCode, Value) {
    request(app, "GET", uri, None).await
}

async fn post(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    request(app, "POST", uri, Some(body)).await
}

/// 建一单：返回 (order_id, pickup_code)
async fn create_order(app: &Router, box_no: &str, location: &str, weight: i64) -> (i64, String) {
    let (status, body) = post(
        app,
        "/api/orders",
        json!({
            "customer_name": "张三",
            "customer_phone": "13800000000",
            "fish_species": "鲈鱼",
            "unit_price_cents": 5600,
            "scale_device_id": "SCALE-01",
            "box_no": box_no,
            "location": location,
            "weight_grams": weight
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "建单失败: {body}");
    (
        body["order"]["id"].as_i64().unwrap(),
        body["pickup_code"]["code"].as_str().unwrap().to_string(),
    )
}

#[tokio::test]
async fn transfer_then_lookup_by_code_finds_new_box() {
    let (app, _dir) = test_app().await;
    let (order_id, code) = create_order(&app, "A-01", "A区-1", 2500).await;

    // 转箱：A-01 -> B-02，记录经手人
    let (status, body) = post(
        &app,
        &format!("/api/orders/{order_id}/transfer"),
        json!({"to_box_no": "B-02", "to_location": "B区-7", "operator": "老王"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "转箱失败: {body}");
    assert_eq!(body["current_box"]["box_no"], "B-02");
    assert_eq!(body["current_box"]["location"], "B区-7");

    // 柜台凭原取货码查询：必须看到最新箱号与箱位
    let (status, detail) = get(&app, &format!("/api/pickup/{code}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["current_box"]["box_no"], "B-02");
    assert_eq!(detail["current_box"]["location"], "B区-7");

    // 转箱记录完整：新旧鱼箱、位置、经手人
    let transfers = detail["transfers"].as_array().unwrap();
    assert_eq!(transfers.len(), 1);
    assert_eq!(transfers[0]["from_box_no"], "A-01");
    assert_eq!(transfers[0]["to_box_no"], "B-02");
    assert_eq!(transfers[0]["from_location"], "A区-1");
    assert_eq!(transfers[0]["to_location"], "B区-7");
    assert_eq!(transfers[0]["operator"], "老王");

    // 旧箱已释放，新箱被占用
    let (_, boxes) = get(&app, "/api/boxes").await;
    let find = |no: &str| {
        boxes
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["box_no"] == no)
            .cloned()
            .unwrap()
    };
    assert_eq!(find("A-01")["status"], "idle");
    assert_eq!(find("B-02")["status"], "in_use");
}

#[tokio::test]
async fn reweigh_keeps_both_readings_and_sale_weight_unchanged() {
    let (app, _dir) = test_app().await;
    let (order_id, code) = create_order(&app, "A-02", "A区-2", 2500).await;

    // 顾客异议，复称得 2430g
    let (status, body) = post(
        &app,
        &format!("/api/orders/{order_id}/reweigh"),
        json!({"scale_device_id": "SCALE-02", "weight_grams": 2430, "note": "顾客异议复称"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "复称失败: {body}");

    let (_, detail) = get(&app, &format!("/api/pickup/{code}")).await;

    // 销售重量不被修改
    assert_eq!(detail["order"]["sale_weight_grams"], 2500);

    // 原始值与复核值都保留
    let weighings = detail["weighings"].as_array().unwrap();
    assert_eq!(weighings.len(), 2);
    assert_eq!(weighings[0]["kind"], "original");
    assert_eq!(weighings[0]["weight_grams"], 2500);
    assert_eq!(weighings[0]["scale_device_id"], "SCALE-01");
    assert_eq!(weighings[1]["kind"], "reweigh");
    assert_eq!(weighings[1]["weight_grams"], 2430);
    assert_eq!(weighings[1]["scale_device_id"], "SCALE-02");
}

#[tokio::test]
async fn concurrent_delivery_only_one_succeeds() {
    let (app, _dir) = test_app().await;
    let (_order_id, code) = create_order(&app, "A-03", "A区-3", 3000).await;

    // 8 个窗口同时交付同一取货码
    let mut handles = Vec::new();
    for _ in 0..8 {
        let app = app.clone();
        let code = code.clone();
        handles.push(tokio::spawn(async move {
            post(&app, &format!("/api/pickup/{code}/deliver"), json!({})).await
        }));
    }

    let mut ok = 0;
    let mut already = 0;
    for h in handles {
        let (status, body) = h.await.unwrap();
        match status {
            StatusCode::OK => ok += 1,
            StatusCode::CONFLICT => {
                assert_eq!(body["error"]["code"], "ALREADY_PROCESSED", "冲突提示应为已处理: {body}");
                already += 1;
            }
            s => panic!("非预期状态码 {s}: {body}"),
        }
    }
    assert_eq!(ok, 1, "只能有一个窗口交付成功");
    assert_eq!(already, 7, "其余窗口必须收到已处理提示");

    // 取货码已失效，订单已交付
    let (_, detail) = get(&app, &format!("/api/pickup/{code}")).await;
    assert_eq!(detail["pickup_code"]["status"], "used");
    assert_eq!(detail["order"]["status"], "delivered");

    // 再交付一次：仍是已处理提示
    let (status, body) = post(&app, &format!("/api/pickup/{code}/deliver"), json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "ALREADY_PROCESSED");
}

#[tokio::test]
async fn one_box_cannot_bind_two_active_codes() {
    let (app, _dir) = test_app().await;
    create_order(&app, "A-04", "A区-4", 2000).await;

    // 同一鱼箱再开一单 -> 拒绝
    let (status, body) = post(
        &app,
        "/api/orders",
        json!({
            "customer_name": "李四",
            "customer_phone": "13900000000",
            "fish_species": "石斑鱼",
            "scale_device_id": "SCALE-01",
            "box_no": "A-04",
            "location": "A区-5",
            "weight_grams": 1800
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "BOX_BUSY");
}

#[tokio::test]
async fn delivered_order_cannot_transfer_or_reweigh() {
    let (app, _dir) = test_app().await;
    let (order_id, code) = create_order(&app, "A-05", "A区-6", 2200).await;

    let (status, _) = post(&app, &format!("/api/pickup/{code}/deliver"), json!({})).await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = post(
        &app,
        &format!("/api/orders/{order_id}/transfer"),
        json!({"to_box_no": "B-03", "to_location": "B区-1", "operator": "老王"}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "ORDER_DELIVERED");

    let (status, body) = post(
        &app,
        &format!("/api/orders/{order_id}/reweigh"),
        json!({"scale_device_id": "SCALE-01", "weight_grams": 2100}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "ORDER_DELIVERED");
}

#[tokio::test]
async fn transfer_to_busy_box_rejected() {
    let (app, _dir) = test_app().await;
    let (order1, _c1) = create_order(&app, "A-06", "A区-1", 2000).await;
    let (_order2, _c2) = create_order(&app, "A-07", "A区-2", 2000).await;

    // 转到被占用的 A-07 -> 拒绝
    let (status, body) = post(
        &app,
        &format!("/api/orders/{order1}/transfer"),
        json!({"to_box_no": "A-07", "to_location": "A区-9", "operator": "老王"}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "BOX_BUSY");
}
