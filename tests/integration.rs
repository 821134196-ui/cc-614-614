use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// 每个测试独立的 SQLite 文件（WAL 模式，连接池 >1，真实并发）
async fn setup() -> axum::Router {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "fishmarket-test-{}-{nanos}-{n}.db",
        std::process::id()
    ));
    let pool = fish_market::db::init_pool(path.to_str().unwrap()).await;
    fish_market::db::migrate(&pool).await;
    fish_market::db::seed(&pool).await;
    fish_market::app::build_app(pool)
}

async fn req(app: &axum::Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let r = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.map(|b| b.to_string()).unwrap_or_default()))
        .unwrap();
    let res = app.clone().oneshot(r).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

async fn create_order(app: &axum::Router, box_code: &str, slot: &str, weight: f64) -> (Value, String) {
    let (status, v) = req(
        app,
        "POST",
        "/api/orders",
        Some(json!({
            "customer_name": "测试顾客",
            "customer_phone": "13800000000",
            "fish_type": "鲈鱼",
            "device_id": "SCALE-01",
            "box_code": box_code,
            "slot_code": slot,
            "weight": weight,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create order failed: {v}");
    let code = v["pickup_code"].as_str().unwrap().to_string();
    (v["order"].clone(), code)
}

/// 场景一：转箱后凭取货码仍能查到最新箱号与箱位，并可正常交付
#[tokio::test]
async fn transfer_then_pickup_code_finds_new_location() {
    let app = setup().await;
    let (order, code) = create_order(&app, "BOX-001", "A-01", 2.5).await;
    let order_id = order["id"].as_i64().unwrap();

    // 转箱：BOX-001/A-01 → BOX-002/B-03，经手人老王
    let (status, v) = req(
        &app,
        "POST",
        &format!("/api/orders/{order_id}/transfer"),
        Some(json!({"to_box_code": "BOX-002", "to_slot_code": "B-03", "operator": "老王"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "transfer failed: {v}");
    assert_eq!(v["order"]["box_code"], "BOX-002");
    assert_eq!(v["order"]["slot_code"], "B-03");
    assert_eq!(v["order"]["transfers"][0]["from_box"], "BOX-001");
    assert_eq!(v["order"]["transfers"][0]["to_box"], "BOX-002");
    assert_eq!(v["order"]["transfers"][0]["operator"], "老王");

    // 柜台输入取货码 → 看到的是最新箱号与箱位
    let (status, v) = req(&app, "GET", &format!("/api/pickup/{code}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["order"]["box_code"], "BOX-002");
    assert_eq!(v["order"]["slot_code"], "B-03");
    assert_eq!(v["code_status"], "active");

    // 转箱后交付仍正常，且取货码立即失效
    let (status, v) = req(&app, "POST", &format!("/api/pickup/{code}/deliver"), None).await;
    assert_eq!(status, StatusCode::OK, "deliver failed: {v}");
    assert_eq!(v["order"]["status"], "delivered");

    let (status, _) = req(&app, "POST", &format!("/api/pickup/{code}/deliver"), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

/// 场景二：重量争议复称，原始值与复核值都保留，销售重量不被修改
#[tokio::test]
async fn reweigh_keeps_both_readings_and_sale_weight_unchanged() {
    let app = setup().await;
    let (order, _code) = create_order(&app, "BOX-003", "A-02", 5.25).await;
    let order_id = order["id"].as_i64().unwrap();

    // 顾客异议 → 复称两次（5.10、5.18）
    for w in [5.10, 5.18] {
        let (status, v) = req(
            &app,
            "POST",
            &format!("/api/orders/{order_id}/reweigh"),
            Some(json!({"device_id": "SCALE-02", "weight": w})),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "reweigh failed: {v}");
    }

    let (status, v) = req(&app, "GET", &format!("/api/orders/{order_id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    // 销售重量保持原始值
    assert_eq!(v["order"]["sale_weight"], 5.25);
    let records = v["order"]["weigh_records"].as_array().unwrap();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0]["kind"], "original");
    assert_eq!(records[0]["weight"], 5.25);
    assert_eq!(records[0]["device_id"], "SCALE-01");
    assert_eq!(records[1]["kind"], "reweigh");
    assert_eq!(records[1]["weight"], 5.10);
    assert_eq!(records[2]["kind"], "reweigh");
    assert_eq!(records[2]["weight"], 5.18);
}

/// 场景三：两个窗口同时办理同一订单，只能交付一次，另一个收到已处理提示
#[tokio::test]
async fn concurrent_delivery_only_succeeds_once() {
    let app = setup().await;
    let (_order, code) = create_order(&app, "BOX-004", "A-03", 3.3).await;

    // 并发发起 8 次交付（模拟两个甚至多个窗口同时扫码）
    let mut handles = Vec::new();
    for _ in 0..8 {
        let app = app.clone();
        let code = code.clone();
        handles.push(tokio::spawn(async move {
            req(&app, "POST", &format!("/api/pickup/{code}/deliver"), None).await
        }));
    }
    let mut ok = 0;
    let mut conflict = 0;
    let mut conflict_msg = String::new();
    for h in handles {
        let (status, v) = h.await.unwrap();
        match status {
            StatusCode::OK => ok += 1,
            StatusCode::CONFLICT => {
                conflict += 1;
                conflict_msg = v["error"].as_str().unwrap_or("").to_string();
            }
            s => panic!("unexpected status {s}: {v}"),
        }
    }
    assert_eq!(ok, 1, "exactly one delivery should succeed");
    assert_eq!(conflict, 7, "all others should get 409 already-processed");
    assert!(
        conflict_msg.contains("已处理") || conflict_msg.contains("已使用"),
        "conflict message should say already processed, got: {conflict_msg}"
    );

    // 订单只被交付一次
    let (_, v) = req(&app, "GET", &format!("/api/pickup/{code}"), None).await;
    assert_eq!(v["order"]["status"], "delivered");
    assert_eq!(v["code_status"], "used");
}

/// 约束：同一鱼箱不能同时绑定两张有效取货码（开单与转箱两个入口都拦截）
#[tokio::test]
async fn one_box_cannot_hold_two_active_codes() {
    let app = setup().await;
    create_order(&app, "BOX-005", "A-04", 1.0).await;

    // 第二个订单用同一鱼箱 → 409
    let (status, v) = req(
        &app,
        "POST",
        "/api/orders",
        Some(json!({
            "customer_name": "李四", "fish_type": "草鱼", "device_id": "SCALE-01",
            "box_code": "BOX-005", "slot_code": "A-05", "weight": 2.0,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(v["error"].as_str().unwrap().contains("取货码"), "got: {v}");

    // 转箱到被占用的鱼箱 → 409
    let (order_b, _code_b) = create_order(&app, "BOX-006", "A-06", 1.5).await;
    let id_b = order_b["id"].as_i64().unwrap();
    let (status, v) = req(
        &app,
        "POST",
        &format!("/api/orders/{id_b}/transfer"),
        Some(json!({"to_box_code": "BOX-005", "to_slot_code": "A-07", "operator": "老王"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(v["error"].as_str().unwrap().contains("取货码"));

    // 交付后鱼箱释放，可以再次绑定新订单
    let (_, v) = req(&app, "GET", "/api/orders", None).await;
    let first = v["orders"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["box_code"] == "BOX-005")
        .unwrap()
        .clone();
    let code = first["pickup_code"].as_str().unwrap();
    let (status, _) = req(&app, "POST", &format!("/api/pickup/{code}/deliver"), None).await;
    assert_eq!(status, StatusCode::OK);

    let (status, v) = req(
        &app,
        "POST",
        "/api/orders",
        Some(json!({
            "customer_name": "王五", "fish_type": "鲫鱼", "device_id": "SCALE-02",
            "box_code": "BOX-005", "slot_code": "A-08", "weight": 0.8,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "box should be reusable after delivery: {v}");
}
