#!/usr/bin/env bash
# 一键同步推送到 GitHub 和 Gitee
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

BRANCH="${1:-main}"

echo "🚀 开始同步推送到 GitHub 和 Gitee (${BRANCH})..."

# 确保双推送 URL 已配置
git remote set-url --add --push origin git@github.com:beyondcy1013/zcode-fresh-reset.git 2>/dev/null || true
git remote set-url --add --push origin git@gitee.com:beyondcy1013/zcode-fresh-reset.git 2>/dev/null || true

echo "📤 正在推送到远端仓库..."
git push origin "${BRANCH}"

echo "✅ 双平台推送完成！"
