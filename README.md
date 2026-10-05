# 海鲜市场 · 活鱼称重暂存与取货系统

摊主卖出活鱼后称重入箱暂存，顾客凭取货码稍后到柜台取货。系统把 **订单 ↔ 鱼箱 ↔ 暂存箱位 ↔ 取货码** 绑定起来，支持转箱、复称争议处理与并发安全交付。

## 一条命令启动

```bash
./start.sh
```

自动安装前端依赖、构建前端、启动后端（首次运行会编译 Rust 依赖，需几分钟）。
打开 http://localhost:8080 即可使用。SQLite 数据库文件为 `market.db`，自动建表并预置 12 个鱼箱（BOX-001~012）与 16 个箱位（A-01~08、B-01~08）。

## 技术栈

- **后端**：Rust + Axum + SQLx（运行时查询，无需编译期数据库）
- **前端**：Svelte + Vite，构建产物由 Axum 托管
- **数据库**：SQLite（WAL 模式）
- **电子秤 / 短信**：模拟接口（`GET /api/mock/scale/{device}/read`，短信写入 `sms_logs` 表并可在 API 查询）

## 核心流程

| 环节 | 接口 | 说明 |
|---|---|---|
| 称重入库 | `POST /api/orders` | 保存电子秤设备号与原始重量，绑定编号鱼箱、暂存箱位，生成取货码并模拟短信下发 |
| 柜台查货 | `GET /api/pickup/{code}` | 凭取货码返回订单、**当前最新**箱号与箱位、全部称重记录 |
| 交付 | `POST /api/pickup/{code}/deliver` | 核对后交付，取货码**原子失效** |
| 转箱 | `POST /api/orders/{id}/transfer` | 记录新旧鱼箱、新旧箱位、经手人；取货码跟随新箱 |
| 复称 | `POST /api/orders/{id}/reweigh` | 新增复核读数，原始值与复核值都保留，销售重量不变 |

## 关键设计

- **并发只交付一次**：交付用单条原子语句
  `UPDATE pickup_codes SET status='used' WHERE code=? AND status='active'`，
  按影响行数判定成败。两个窗口同时办理同一订单，只有一个成功，另一个收到 `409 已处理` 提示。
- **一箱一码**：部分唯一索引 `one_active_code_per_box ON pickup_codes(box_id) WHERE status='active'`，
  开单与转箱两个入口都由数据库层兜底，交付后鱼箱自动释放。
- **重量争议不改销售重量**：`orders.sale_weight` 一经写入不再修改；
  原始读数与每次复称都存入 `weigh_records`（`kind = original / reweigh`）。
- **转箱可追溯**：`transfers` 表记录 from/to 鱼箱、箱位与经手人；
  取货码的 `box_id` 随箱迁移，柜台查码永远看到最新位置。

## 验证（集成测试）

```bash
cargo test
```

四个测试覆盖需求验证点：

1. `transfer_then_pickup_code_finds_new_location` — 转箱后凭取货码仍查到最新箱号/箱位，交付正常
2. `reweigh_keeps_both_readings_and_sale_weight_unchanged` — 复称保留两次读数，销售重量不变
3. `concurrent_delivery_only_succeeds_once` — 8 个并发交付请求，恰好 1 个成功、7 个收到已处理提示
4. `one_box_cannot_hold_two_active_codes` — 同一鱼箱不能绑定两张有效取货码，交付后可复用

## 目录结构

```
├── src/
│   ├── main.rs       # 入口：连库、迁移、种子数据、起服务
│   ├── app.rs        # 路由 + 静态文件托管
│   ├── db.rs         # 连接池、schema、种子数据
│   ├── handlers.rs   # 称重/取货/转箱/复称/查询 API
│   ├── mock.rs       # 电子秤与短信模拟
│   ├── models.rs     # 表结构映射
│   └── error.rs      # 统一错误 → HTTP 状态码
├── frontend/         # Svelte + Vite（称重入库/取货柜台/转箱/复称/订单列表 五个页签）
├── tests/integration.rs
└── start.sh
```
