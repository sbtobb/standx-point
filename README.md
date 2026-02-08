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

### 一键启动做市机器人

#### CLI 模式（适合服务器运行）

```bash
# 1. 构建做市策略机器人
cargo build -p standx-point-mm-strategy --release

# 2. 创建配置文件（参考 examples/single_task.yaml）
cp crates/standx-point-mm-strategy/examples/single_task.yaml config.yaml

# 3. 编辑配置文件，添加真实的钱包私钥
# 提示：在 config.yaml 中替换 private_key 的值

# 4. 验证配置（干运行）
cargo run -p standx-point-mm-strategy -- --config config.yaml --dry-run

# 5. 启动做市机器人
cargo run -p standx-point-mm-strategy -- --config config.yaml

# 或者使用 release 版本（推荐生产环境）
./target/release/standx-point-mm-strategy --config config.yaml

# 也支持环境变量启动（需显式开启 --env）
# export STANDX_MM_PRIVATE_KEY="your-wallet-private-key"
# export STANDX_MM_SYMBOL="BTC-USD"
# export STANDX_MM_RISK_LEVEL="low"
# export STANDX_MM_BUDGET_USD="50000"
# # Optional:
# # export STANDX_MM_CHAIN="bsc"
# # export STANDX_MM_ACCOUNT_ID="account-1"
# # export STANDX_MM_TASK_ID="task-btc"
# ./target/release/standx-point-mm-strategy --env --dry-run
```

环境变量说明：

- `STANDX_MM_PRIVATE_KEY`：钱包私钥（必填）
- `STANDX_MM_SYMBOL`：交易对（必填）
- `STANDX_MM_RISK_LEVEL`：风险等级（必填，low/medium/high/xhigh）
- `STANDX_MM_BUDGET_USD`：预算（必填，USD）
- `STANDX_MM_CHAIN`：链（可选，bsc/solana，默认 bsc）
- `STANDX_MM_ACCOUNT_ID`：账户 ID（可选，默认使用钱包地址）
- `STANDX_MM_TASK_ID`：任务 ID（可选，默认基于 symbol）

#### Docker 启动（使用环境变量）

```bash
# Build image
docker build -t standx-point-mm-strategy:latest .

# Run with env vars
docker run --rm \
  -e STANDX_MM_PRIVATE_KEY="your-wallet-private-key" \
  -e STANDX_MM_SYMBOL="BTC-USD" \
  -e STANDX_MM_RISK_LEVEL="low" \
  -e STANDX_MM_BUDGET_USD="50000" \
  -e STANDX_MM_CHAIN="bsc" \
  standx-point-mm-strategy:latest
```

#### 配置详情

详细配置说明请参考 [standx-point-mm-strategy README.md](./crates/standx-point-mm-strategy/README.md)

## 开发说明

详细开发指南与规范请参考 [AGENTS.md](./AGENTS.md)。

### 运行测试

```bash
# 运行所有测试
cargo test --workspace

# 运行做市策略机器人测试
cargo test -p standx-point-mm-strategy

# 运行适配器测试
cargo test -p standx-point-adapter
```

## 项目架构

### 协议适配层 (standx-point-adapter)

- StandX API 的完整 Rust 客户端
- HTTP 和 WebSocket 支持
- 认证与签名管理
- 响应类型定义

### 策略层 (standx-point-mm-strategy)

- 多账户管理
- 共享行情流
- 风险管理
- 做市策略实现
- 生命周期管理

## 安全警告

⚠️ **交易涉及风险**

- 本机器人会在交易所放置真实订单
- 可能导致财务损失
- 需要正确的风险配置
- 建议在实盘交易前进行充分测试

## License

MIT License - See LICENSE file for details
