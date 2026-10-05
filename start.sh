#!/usr/bin/env bash
# 海鲜市场活鱼称重暂存与取货系统 —— 一条命令启动
set -euo pipefail
cd "$(dirname "$0")"

export PATH="$HOME/.cargo/bin:$PATH"

# 1. 构建前端（已构建过则跳过；REBUILD_FRONTEND=1 强制重建）
if [ ! -d frontend/node_modules ]; then
  echo "==> 安装前端依赖"
  (cd frontend && npm install)
fi
if [ ! -d frontend/dist ] || [ "${REBUILD_FRONTEND:-0}" = "1" ]; then
  echo "==> 构建前端"
  (cd frontend && npm run build)
fi

# 2. 启动后端（托管 API 与前端静态文件，SQLite 自动建库建表）
echo "==> 启动服务  http://localhost:8080"
exec cargo run
