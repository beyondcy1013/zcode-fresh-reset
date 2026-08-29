#!/usr/bin/env bash
# 一键同步推送到 GitHub 和 Gitee
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

BRANCH="${1:-main}"

echo "🚀 开始同步推送到 GitHub 和 Gitee (${BRANCH})..."

echo "📤 [1/2] 正在推送到 GitHub..."
git push https://github.com/beyondcy1013/zcode-fresh-reset.git "${BRANCH}"

echo "📤 [2/2] 正在推送到 Gitee..."
git push git@gitee.com:beyondcy1013/zcode-fresh-reset.git "${BRANCH}"

echo "✅ 双平台推送全部完成！"
