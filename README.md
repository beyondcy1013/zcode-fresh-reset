# ZCode Fresh Reset & Session Sanitizer (`zcode-fresh-reset`)

<p align="center">
  <b>⚡ 一键安全清理 ZCode 客户端本地登录状态、Cookie 会话与套餐权益缓存，快速还原纯净新机环境 ⚡</b>
</p>

<p align="center">
  <a href="https://github.com/beyondcy1013/zcode-fresh-reset/stargazers"><img src="https://img.shields.io/github/stars/beyondcy1013/zcode-fresh-reset" alt="Stars Badge"/></a>
  <a href="https://github.com/beyondcy1013/zcode-fresh-reset/issues"><img src="https://img.shields.io/github/issues/beyondcy1013/zcode-fresh-reset" alt="Issues Badge"/></a>
  <a href="https://github.com/beyondcy1013/zcode-fresh-reset/blob/main/LICENSE"><img src="https://img.shields.io/github/license/beyondcy1013/zcode-fresh-reset" alt="License Badge"/></a>
</p>

---

## 📌 项目简介 (About)

在使用 **ZCode (智谱 BigModel 编程客户端 / Z.ai)** 时，很多开发者遇到以下痛点：
- **换号/切换多账号困难**：客户端经常记住旧账号的 Cookie 和 OAuth Token，无法干净退出或切换。
- **无法弹出新用户免费 Flash 套餐引导**：老账号登录过的机器残留了本地权益缓存（`coding-plan-cache.json`），即使换了新账号也不会弹出新版免费套餐领取界面。
- **环境残留与排错困难**：本地缓存损坏导致客户端无法拉取最新 Plan 或报错。

**`zcode-fresh-reset`** 提供了针对 ZCode 客户端的一键本地重置与备份方案，精准清理相关凭据、网页会话、设备缓存与套餐信息，让客户端秒回“刚下载新机”的纯净状态。

---

## 🔍 深度原理解析 (Why & How)

### 1. 为什么“新安装客户端”会弹出领取 Flash 免费套餐？
- **本地机制**：当客户端检测不到 `credentials.json`、`session/Cookies` 和 `coding-plan-cache.json` 时，判定当前为“首次启动未初始化状态”，会自动激活新手引导及套餐领取弹窗。
- **云端校验**：免费 Flash 套餐最终由 BigModel / ZCode 云端根据账号（UID/手机号/OAuth）和设备特征进行核验下发。

### 2. 为什么登录过的账号不会再弹出？
- **本地有缓存**：`coding-plan-cache.json` 记录了旧套餐数据，客户端启动直接读缓存，不再请求新引导。
- **云端有记录**：同一个账号在服务端已被标记为“已领过”或“老用户”。

### 3. 正确的“新用户领取”操作流程
1. 运行本工具执行 **`python3 zcode_reset.py clean`**，彻底清理本地老账号状态。
2. 启动 ZCode 客户端，此时客户端处于纯净新机状态。
3. 登录**未领取过该福利的新账号**，客户端将正常触发新用户新手引导并成功领取免费 Flash 套餐。

---

## 📂 涉及的关键路径清单

| 类别 | 路径 (Linux / macOS / Windows) | 作用说明 |
| :--- | :--- | :--- |
| **登录凭据** | `~/.zcode/v2/credentials.json` | 用户登录 Token 与授权身份 |
| **套餐缓存** | `~/.zcode/v2/coding-plan-cache.json` | 缓存的 Plan 权益与领取状态 |
| **遥测埋点** | `~/.zcode/v2/telemetry-state.json` | 本地客户端状态数据 |
| **网页会话** | `~/.config/ZCode/session/Cookies` | Electron 登录态 Cookie |
| **页面存储** | `~/.config/ZCode/session/Local Storage/` | 前端页面持久化数据 |
| **设备特征** | `~/.config/ZCode/rum-electron-store/` | 客户端设备监控与特征信息 |
| **升级标记** | `~/.config/ZCode/.updaterId` | 客户端更新器标识 |

---

## 🚀 快速上手 (Quick Start)

### 1. 克隆项目
```bash
git clone https://github.com/beyondcy1013/zcode-fresh-reset.git
cd zcode-fresh-reset
```

### 2. 查看当前状态
```bash
python3 zcode_reset.py inspect
```

### 3. 一键完整重置（推荐）
> ⚠️ **注意**：执行前请确保已**完全退出 ZCode 客户端**。默认会自动在 `~/.zcode/reset_backups/` 下建立完整备份，安全无忧。
```bash
python3 zcode_reset.py clean
```

### 4. 安全模式（仅清理套餐缓存，不影响当前登录凭据）
```bash
python3 zcode_reset.py clean --safe
```

### 5. 仅备份配置
```bash
python3 zcode_reset.py backup
```

---

## 🤝 交流与联系 (Contact & Author)

如果您在使用过程中遇到任何问题，或者有功能建议，欢迎联系与交流：

- **GitHub Issues**: [提交 Issue](https://github.com/beyondcy1013/zcode-fresh-reset/issues)
- **GitHub 主页**: [@beyondcy1013](https://github.com/beyondcy1013)
- **项目仓库**: [https://github.com/beyondcy1013/zcode-fresh-reset](https://github.com/beyondcy1013/zcode-fresh-reset)

🌟 如果这个项目对你有帮助，欢迎在 GitHub 点个 **Star** 支持一下！

---

## 📄 开源协议 (License)

本项目采用 [MIT License](LICENSE) 协议开源。
