#!/usr/bin/env bash
# 海鲜市场活鱼暂存取货系统 —— 一条命令启动
# 用法: ./start.sh
set -euo pipefail
cd "$(dirname "$0")"

export PATH="$HOME/.cargo/bin:$PATH"

echo "==> [1/2] 构建前端 (Svelte + Vite)..."
(cd frontend && npm install --no-fund --no-audit && npm run build)

echo "==> [2/2] 启动后端 (Rust + Axum + SQLite)..."
echo "    打开 http://localhost:8080 使用系统"
cargo run --release --manifest-path backend/Cargo.toml
