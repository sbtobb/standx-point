# Decisions - standx-point-gui

## 2026-02-03 Initial Setup

### Framework Choice
- GPUI 0.2.2 + gpui-component 0.5.0 (per plan requirements)
- SQLite with rusqlite for persistence
- Encrypted credentials (ring or aes-gcm)

### Architecture
- Reuse adapter's StandxClient and StandxWebSocket
- Task state machine: Draft → Pending → Running → Paused → Stopped → Failed
- Manual crash recovery (tasks marked as Paused on startup)
- Max 5 accounts in MVP

## 2026-02-04 Build Dependency Pinning

- Pin core-foundation/core-graphics/core-text crates to the core-text-v21.0.0 tag to keep a single source revision and avoid mixed Core Graphics types.

## 2026-02-04 Database Layer Security

- 使用 AES-256-GCM 加密账户凭证，密钥种子优先读取 `STANDX_DB_PASSPHRASE`，否则回退到本机用户/主机信息。
- 加密结果使用 hex 编码存入 TEXT 字段，解密时按 nonce(12) + ciphertext 解析。
