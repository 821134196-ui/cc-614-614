//! 电子秤与短信网关的模拟实现。
//! 真实部署时替换为串口/厂商 SDK 对接，接口形状保持不变。

use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

/// 市场内登记在册的电子秤设备号
pub const SCALE_DEVICES: [&str; 3] = ["SCALE-01", "SCALE-02", "SCALE-03"];

/// 模拟读取电子秤：返回 0.50 ~ 15.00 kg 之间、精确到 0.01kg 的读数。
/// 未知设备返回 None。
pub fn read_scale(device_id: &str) -> Option<f64> {
    if !SCALE_DEVICES.contains(&device_id) {
        return None;
    }
    let mut rng = rand::thread_rng();
    let kg: f64 = rng.gen_range(0.5..=15.0);
    Some((kg * 100.0).round() / 100.0)
}

/// 6 位数字取货码
pub fn gen_pickup_code() -> String {
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..1_000_000u32))
}

/// 订单号：O + 毫秒时间戳 + 3 位随机
pub fn gen_order_no() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let mut rng = rand::thread_rng();
    format!("O{ms}{:03}", rng.gen_range(0..1000u32))
}
