# 海鲜市场 · 活鱼称重暂存与取货系统

摊主卖出活鱼后称重入库：保存电子秤设备号与原始重量，活鱼放入编号鱼箱并记录暂存箱位，系统生成取货码并（模拟）短信通知顾客。顾客到店后，柜台凭取货码快速定位订单、鱼箱与箱位，核对重量后交付，取货码立即失效。支持转箱留痕、重量异议复称（双读数保留）。

## 一条命令启动

```bash
./start.sh
```

脚本会构建前端（Svelte + Vite）并启动后端（Rust + Axum + SQLx + SQLite），然后打开 **http://localhost:8080** 即可使用。数据库文件 `fishmarket.db` 自动创建于项目根目录。

环境变量（均有默认值，可不设）：

| 变量 | 默认值 | 说明 |
|---|---|---|
| `DATABASE_URL` | `sqlite://fishmarket.db` | SQLite 连接串 |
| `STATIC_DIR` | `frontend/dist` | 前端静态文件目录 |
| `LISTEN_ADDR` | `0.0.0.0:8080` | 监听地址 |

## 技术栈与结构

```
backend/            Rust + Axum + SQLx（SQLite）
  migrations/       建表 SQL（含"一箱一码"部分唯一索引）
  src/handlers.rs   业务逻辑：入库 / 取货 / 转箱 / 复称
  src/mock.rs       模拟电子秤读数、取货码生成
  tests/            集成测试（见下）
frontend/           Svelte + Vite 单页应用
  src/lib/          称重入库 / 取货柜台 / 转箱 / 复称 / 订单总览 五个页面
start.sh            一条命令启动
```

电子秤（`/api/mock/scale/read`）与短信网关（落 `sms_logs` 表 + 日志）均为模拟实现。

## 关键设计

- **一箱一码**：`pickup_codes` 上有部分唯一索引 `uq_active_code_per_box (box_id) WHERE status='active'`，同一鱼箱不可能同时绑定两张有效取货码；建单与转箱都在 `BEGIN IMMEDIATE` 事务内校验箱状态。
- **并发交付只成功一次**：交付 = 单条原子 `UPDATE pickup_codes SET status='used' WHERE code=? AND status='active'`。SQLite 写锁串行化并发事务，只有一个窗口命中 1 行，其余 `rows_affected=0` → 返回 `409 ALREADY_PROCESSED`（"该取货码已处理"）。
- **转箱留痕**：`transfers` 表记录新旧鱼箱、新旧箱位、经手人；取货码改绑新箱，柜台凭原码即可查到最新箱号与箱位。
- **复称不改销售重量**：原始称重与复核称重都是 `weighings` 表中的追加流水（`kind='original'/'reweigh'`），订单 `sale_weight_grams` 永不被更新。

## API 一览

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/api/orders` | 称重入库：保存秤号/原始重量、绑箱绑码、发模拟短信 |
| GET | `/api/orders` · `/api/orders/{id}` | 订单列表 / 详情（含称重与转箱记录） |
| GET | `/api/pickup/{code}` | 凭取货码查订单、最新箱号箱位 |
| POST | `/api/pickup/{code}/deliver` | 交付（原子失效取货码；重复交付返回 409） |
| POST | `/api/orders/{id}/transfer` | 转箱（新箱号、新箱位、经手人） |
| POST | `/api/orders/{id}/reweigh` | 复称（保留双读数，销售重量不变） |
| GET/POST | `/api/boxes` | 鱼箱列表 / 新增 |
| GET | `/api/mock/scale/read?device_id=...` | 模拟电子秤读数 |
| GET | `/api/mock/sms` | 模拟短信发送记录 |

## 验证

### 自动化测试

```bash
cd backend && cargo test
```

集成测试覆盖题目要求的三个场景（另加箱码约束、交付后禁转箱/复称等）：

- `transfer_then_lookup_by_code_finds_new_box` — 转位后凭取货码仍查到最新箱号/箱位，转箱记录完整
- `reweigh_keeps_both_readings_and_sale_weight_unchanged` — 重量争议保留原始+复核两次读数，销售重量不变
- `concurrent_delivery_only_one_succeeds` — 8 窗口并发交付同一取货码，恰好 1 个成功、7 个收到"已处理"

### 手动验证（服务启动后）

```bash
# 1. 建单（秤号 SCALE-01，原始重量 2500g，入 A-01 箱）
curl -s -X POST localhost:8080/api/orders -H 'content-type: application/json' -d \
  '{"customer_name":"张三","customer_phone":"13800000000","fish_species":"鲈鱼",
    "scale_device_id":"SCALE-01","box_no":"A-01","location":"A区-1","weight_grams":2500}'
# → 记下返回的 pickup_code.code（假设为 ABCD2345）与 order.id

# 2. 转箱到 B-02，再凭取货码查询 → 看到最新箱号 B-02 / 箱位 B区-7
curl -s -X POST localhost:8080/api/orders/1/transfer -H 'content-type: application/json' -d \
  '{"to_box_no":"B-02","to_location":"B区-7","operator":"老王"}'
curl -s localhost:8080/api/pickup/ABCD2345 | jq '.current_box'

# 3. 复称 2430g → 称重流水里原始 2500 与复核 2430 都在，销售重量仍 2500
curl -s -X POST localhost:8080/api/orders/1/reweigh -H 'content-type: application/json' -d \
  '{"scale_device_id":"SCALE-02","weight_grams":2430}'
curl -s localhost:8080/api/pickup/ABCD2345 | jq '{sale: .order.sale_weight_grams, weighings: [.weighings[] | {kind, weight_grams}]}'

# 4. 并发交付：恰好一个 200，其余 409 已处理
seq 1 8 | xargs -P8 -I{} curl -s -o /dev/null -w '%{http_code}\n' \
  -X POST localhost:8080/api/pickup/ABCD2345/deliver
```

前端「取货柜台」页也内置了"⚡ 双窗口并发交付演示"按钮，可直观看到一成功一已处理。
