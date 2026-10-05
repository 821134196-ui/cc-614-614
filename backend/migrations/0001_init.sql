-- 海鲜市场活鱼称重暂存与取货系统 - 初始 schema

-- 销售订单：销售重量（sale_weight_grams）一经写入不再修改，重量争议通过 weighings 中的复核记录体现
CREATE TABLE orders (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    order_no          TEXT NOT NULL UNIQUE,
    customer_name     TEXT NOT NULL,
    customer_phone    TEXT NOT NULL,
    fish_species      TEXT NOT NULL,
    sale_weight_grams INTEGER NOT NULL,
    unit_price_cents  INTEGER NOT NULL DEFAULT 0,
    status            TEXT NOT NULL DEFAULT 'stored' CHECK (status IN ('stored', 'delivered')),
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    delivered_at      TEXT
);

-- 称重流水：原始称重(kind='original')与复核称重(kind='reweigh')都保留，永不更新
CREATE TABLE weighings (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id        INTEGER NOT NULL REFERENCES orders (id),
    scale_device_id TEXT NOT NULL,
    weight_grams    INTEGER NOT NULL,
    kind            TEXT NOT NULL CHECK (kind IN ('original', 'reweigh')),
    note            TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_weighings_order ON weighings (order_id);

-- 编号鱼箱：status idle=空闲 / in_use=使用中，location 为当前暂存箱位
CREATE TABLE boxes (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    box_no   TEXT NOT NULL UNIQUE,
    location TEXT NOT NULL DEFAULT '',
    status   TEXT NOT NULL DEFAULT 'idle' CHECK (status IN ('idle', 'in_use'))
);

-- 取货码：一个订单一张码（order_id UNIQUE）；交付成功后 status -> 'used' 立即失效
CREATE TABLE pickup_codes (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    code       TEXT NOT NULL UNIQUE,
    order_id   INTEGER NOT NULL UNIQUE REFERENCES orders (id),
    box_id     INTEGER NOT NULL REFERENCES boxes (id),
    status     TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'used', 'void')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    used_at    TEXT
);
-- 核心约束：同一鱼箱同一时刻只能绑定一张有效取货码
CREATE UNIQUE INDEX uq_active_code_per_box ON pickup_codes (box_id) WHERE status = 'active';

-- 转箱记录：新旧鱼箱、新旧箱位、经手人
CREATE TABLE transfers (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id      INTEGER NOT NULL REFERENCES orders (id),
    from_box_id   INTEGER NOT NULL REFERENCES boxes (id),
    to_box_id     INTEGER NOT NULL REFERENCES boxes (id),
    from_location TEXT NOT NULL,
    to_location   TEXT NOT NULL,
    operator      TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_transfers_order ON transfers (order_id);

-- 模拟短信发送记录
CREATE TABLE sms_logs (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    phone      TEXT NOT NULL,
    content    TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
