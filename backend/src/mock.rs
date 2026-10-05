//! 模拟外部设备/服务：电子秤读数、取货码与单号生成。
//! 短信发送不写外部接口，直接落 sms_logs 表并打日志（见 handlers）。

use rand::Rng;

/// 取货码字母表：去掉易混淆字符 0/O/1/I
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

pub fn gen_pickup_code() -> String {
    let mut rng = rand::rng();
    (0..8)
        .map(|_| CODE_ALPHABET[rng.random_range(0..CODE_ALPHABET.len())] as char)
        .collect()
}

pub fn gen_order_no() -> String {
    let mut rng = rand::rng();
    format!(
        "O{}-{:06}",
        chrono::Local::now().format("%Y%m%d"),
        rng.random_range(0..1_000_000u32)
    )
}

/// 模拟电子秤读数：0.8kg ~ 6.5kg 的活鱼
pub fn random_weight_grams() -> i64 {
    rand::rng().random_range(800..=6500)
}
