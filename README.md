# StandX Point

StandX Point 是一个以 Rust 为核心的做市策略工作区，涵盖协议适配、行情接入、风险管理与策略编排。项目采用分层与分包设计，确保协议层与策略层职责清晰。

## Workspace 结构

- **crates/standx-point-adapter**: StandX 协议适配层（Auth/HTTP/WebSocket）。
- **crates/standx-point-mm-strategy**: 做市策略机器人（多账户、共享行情流、风险管理、CLI）。

## 快速开始

### 前置要求

- Rust（建议使用最新稳定版）

### Build

```bash
cargo build --workspace
```

#### CLI 模式（适合服务器运行）

```bash
cargo run -p standx-point-mm-strategy
```

## 开发说明

详细开发指南与规范请参考 [AGENTS.md](./AGENTS.md)。
