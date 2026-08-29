# ZCode Fresh Reset & Session Sanitizer (`zcode-fresh-reset`)

一款用于安全管理与重置 ZCode 客户端本地状态的工具，支持登录信息与会话清理、本地套餐权益缓存重置、设备标识刷新以及自动状态备份。

---

## 核心机制与原理分析

### 1. 为什么“首次使用/新装客户端”会弹出领取 Flash 免费套餐？
- **本地层面**：客户端在没有本地登录凭据 (`credentials.json`)、会话 Cookie 以及权益缓存 (`coding-plan-cache.json`) 时，会判定处于“未初始化状态”，并在启动或引导登录时触发“新用户套餐 / 活动弹窗”。
- **服务端层面**：**免费 Flash 套餐的最终判定和下发是在服务端完成的**（基于账号 UID / OAuth 绑定的手机号/邮箱/微信，以及设备特征指纹）。

### 2. 为什么已登录过的账号不会再次弹出？
1. **本地有缓存**：`coding-plan-cache.json` 记录了当前账号的已领用套餐信息，客户端直接读取缓存而不再请求领取弹窗。
2. **服务端已登记**：同一账号在 BigModel/ZCode 后端已有领取历史记录。即使用户在本地重置了客户端，若重新登录**同一个老账号**，服务端拉取权益时依然返回“已领取”或“不满足首次领取条件”。

### 3. 如何才能像新用户一样领取？
1. **清理本地旧状态**：使用本项目清理本地 `credentials.json`、`session/Cookies`、`coding-plan-cache.json`、`rum-electron-store` 等残留文件，将客户端还原为纯净的“新装机状态”。
2. **接入新账号**：重新启动 ZCode 时，登录**未领取过该免费套餐的新账号**（或由新手机号注册的账号）。客户端会完整展示新手引导并成功向服务端申请领取 Flash 免费套餐。

---

## 涉及的关键本地路径清单

| 类别 | 路径 | 作用说明 |
| :--- | :--- | :--- |
| **登录凭据** | `~/.zcode/v2/credentials.json` | 存储当前登录用户的 token/身份信息 |
| **权益缓存** | `~/.zcode/v2/coding-plan-cache.json` | 缓存已查询到的 Plan 权益、配额及领取状态 |
| **网页会话** | `~/.config/ZCode/session/Cookies` | Electron 登录态会话与 Cookie |
| **存储数据** | `~/.config/ZCode/session/Local Storage/` | 页面前端本地状态与配置 |
| **设备埋点** | `~/.config/ZCode/rum-electron-store/` | 客户端设备分析与监控特征 |
| **升级标识** | `~/.config/ZCode/.updaterId` | 客户端更新器标识 |

---

## 功能特性

- 🔍 **状态检测 (`inspect`)**：一键检测当前机器上 ZCode 的运行状态和本地缓存文件存在情况。
- 📦 **自动备份 (`backup`)**：在清理前自动归档当前登录凭据和配置，防止误操作。
- 🧹 **深度重置 (`clean`)**：彻底清理凭据、Cookie、Local Storage 和套餐缓存，恢复纯净新装机状态。
- 🛡️ **安全模式 (`clean --safe`)**：仅清除权益缓存和 Cookie，保留核心凭据。

---

## 使用方法

### 1. 检查当前本地状态
```bash
python3 zcode_reset.py inspect
```

### 2. 执行完整重置（清理登录态与缓存）
> **注意**：执行前请确保已完全退出 ZCode 客户端。
```bash
python3 zcode_reset.py clean
```

### 3. 仅备份配置（不执行清理）
```bash
python3 zcode_reset.py backup
```

---

## 开源协议

MIT License
