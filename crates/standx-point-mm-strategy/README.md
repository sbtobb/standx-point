# StandX Market Making Strategy Bot

面向生产的 StandX 做市机器人，支持多账户、共享行情流与谨慎风险管理。

## Features

- **TUI Management**: Interactive terminal interface for managing accounts and tasks
- **Multi-Account Support**: Run multiple trading tasks simultaneously with isolated state
- **Shared Market Data**: Single WebSocket connection feeds all tasks via `tokio::sync::watch` channels
- **Low 策略**: 5 层、5-30 bps 区间做市
- **Risk Management**: Price jump protection, depth monitoring, position limits, fill rate tracking
- **Automatic Reconnection**: Exponential backoff for WebSocket reconnection (max 30s)
- **Graceful Shutdown**: SIGTERM handling with order cancellation and position closure
- **Uptime Tracking**: Track active quoting time for monthly token rewards

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                 Market Data Hub                              │
│  (Single WebSocket + watch channel broadcast)               │
└──────────────┬──────────────────────────────────────┘
               │
      ┌────────┴────────┬───────────────┐
      ▼                 ▼               ▼
┌──────────┐     ┌──────────┐    ┌──────────┐
│ Task 1   │     │ Task 2   │    │ Task N   │
│ (Acct A) │     │ (Acct B) │    │ (Acct X) │
│ - Strategy│     │ - Strategy│    │ - Strategy│
│ - Risk    │     │ - Risk    │    │ - Risk    │
│ - Execution│    │ - Execution│   │ - Execution│
└──────────┘     └──────────┘    └──────────┘
```

## Quick Start

### 1. Configuration

Create a configuration file (see `examples/single_task.yaml`):

```yaml
accounts:
  - id: "account-1"
    jwt_token: "your-jwt-token"
    signing_key: "your-ed25519-key"
    chain: "bsc"
tasks:
  - id: "btc-mm"
    symbol: "BTC-USD"
    account_id: "account-1"
    risk:
      level: "low"
      budget_usd: "50000"
```

### 2. Run the Bot

Bot 支持两种运行模式：交互式 TUI 模式和传统 CLI 模式。

#### TUI 模式 (推荐)
直接运行程序而不带 `--config` 参数即可进入交互式 TUI 界面。
```bash
./target/release/standx-point-mm-strategy
```

#### CLI 模式
使用 `--config` 指定配置文件，适合服务器长期运行。
```bash
# Run with config
./target/release/standx-point-mm-strategy --config config.yaml

# Dry run (validate config without trading)
./target/release/standx-point-mm-strategy --config config.yaml --dry-run
```

## Using the TUI

TUI 提供了一个直观的界面来管理交易账户和做市任务。

### 布局说明
- **Status Bar (顶部)**: 显示当前模式（Accounts/Tasks）以及系统状态消息。
- **Sidebar (左侧)**: 显示账户或任务列表，支持通过快捷键切换。
- **Detail View (右侧)**: 显示选中项的详细信息、风险参数或运行状态。
- **Bottom Menu (底部)**: 常用快捷键提示。

### 快捷键
| 键位 | 动作 |
|-------|------|
| `F1` | 显示/关闭帮助弹层 |
| `F2` | 切换到账户管理 (Accounts) |
| `F3` | 切换到任务管理 (Tasks) |
| `F4` | 显示/隐藏敏感凭证 (JWT/Key) |
| `j / ↓` | 向下移动选择 |
| `k / ↑` | 向上移动选择 |
| `h / ←` | 聚焦侧边栏 |
| `l / →` | 聚焦详情视图 |
| `Tab` | 循环切换焦点 |
| `Enter` | 确认/选择 |
| `n` | 创建新项目 (Account/Task) |
| `e` | 编辑选中项目 |
| `d` | 删除选中项目 |
| `s` | 启动选中任务 |
| `x` | 停止选中任务 |
| `q / Esc` | 退出程序 |

### 操作流程
1. **创建账户**: 按 `F2` 进入账户界面，按 `n` 打开表单，输入 `JWT Token` 和 `Signing Key`。
2. **创建任务**: 按 `F3` 进入任务界面，按 `n` 创建任务。需选择已关联的账户，并指定 `Symbol` (如 `BTC-USD`)。
3. **启动策略**: 在任务列表中选中任务，按 `s` 启动。此时 Status Bar 会实时反映任务的运行状态。

### 故障排除
- **终端尺寸**: TUI 要求最小尺寸为 `80x24`。若终端过小，界面会显示覆盖提示。
- **无价格更新**: 检查网络连接。程序启动时会尝试连接 WebSocket 订阅市场数据。
- **测试模式**: 开发者可设置环境变量 `STANDX_TUI_TEST_EXIT_AFTER_TICKS=N` 让程序在运行 N 个 tick 后自动退出。

### 3. Monitor

The bot uses `tracing` for structured logging:

```bash
# Set log level
RUST_LOG=info ./target/release/standx-point-mm-strategy --config config.yaml
```

## Configuration Reference

### Task Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | String | Unique task identifier |
| `symbol` | String | Trading pair (e.g., "BTC-USD") |
| `account_id` | String | Account identifier from `accounts` |
| `risk.level` | String | "low", "medium", "high", or "xhigh" |
| `risk.budget_usd` | String | Budget in USD used for quoting |

### Account Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | String | Account identifier referenced by tasks |
| `jwt_token` | String | JWT authentication token |
| `signing_key` | String | Ed25519 private key (base64) |
| `chain` | String | "bsc" or "solana" |

### Derived Parameters

- `tiers` 由 `risk.level` 派生（low=5, medium=3, high=2, xhigh=1）。
- `base_qty` is derived from `risk.budget_usd` and current mark price using a risk-based utilization (10%/20%/30%).

### Risk Levels

- **Low**: 5 tiers, 5-30 bps band
- **Medium**: 3 tiers, 5-15 bps band
- **High**: 2 tiers, 5-10 bps band
- **XHigh**: 1 tier, 5-8 bps band

## Strategy Details

### Market Making Logic

1. **Price Monitoring**: Watches mark price via WebSocket
2. **Quote Placement**: Places bilateral PostOnly orders at configured bps offset
3. **Tier Management**: Maintains L1 (5-10bps), L2 (10-15bps), L3 (15-20bps) ladders
4. **Price Drift**: Cancels and replaces orders when price moves >1 bps
5. **Partial Fills**: Re-quotes remaining quantity
6. **Full Fills**: Enters cooldown period to avoid immediate re-entry

### Uptime Tracking

Tracks active quoting time for StandX monthly token rewards:
- Requires bilateral quotes within 10 bps
- Minimum 30 minutes per hour
- 5M token pool distributed monthly

## Risk Management

### Guards

- **Price Jump**: Pauses trading if price changes > threshold bps/second
- **Depth Monitoring**: Pauses if order book depth drops below threshold
- **Position Limit**: Stops new orders if position exceeds limit
- **Fill Rate**: Pauses if fills exceed threshold per minute
- **Spread Monitoring**: Avoids quoting when spread > threshold

### States

- `Safe`: Normal operation
- `Caution`: Some metrics elevated (logs warnings)
- `Halt`: Trading paused (notifies tasks)

## Development

### Testing

```bash
# Run all tests
cargo test --package standx-point-mm-strategy

# Run specific module tests
cargo test --package standx-point-mm-strategy strategy
cargo test --package standx-point-mm-strategy risk

# Run integration tests
cargo test --package standx-point-mm-strategy --test integration_test
```

### Project Structure

```
crates/standx-point-mm-strategy/
├── src/
│   ├── lib.rs              # Public API
│   ├── config.rs           # Configuration parsing
│   ├── market_data.rs      # Market data hub
│   ├── task.rs             # Task manager
│   ├── order_state.rs      # Order state machine
│   ├── strategy.rs         # Market making logic
│   ├── risk.rs             # Risk management
│   └── main.rs             # Binary entry point
├── tests/                  # Integration tests
├── examples/               # Example configurations
└── README.md              # This file
```

## Safety & Warnings

⚠️ **Trading involves risk. This bot:**
- Places real orders on the exchange
- Can result in financial loss
- Requires proper risk configuration
- Should be tested thoroughly before live trading

**Recommendations:**
- Start with small position sizes
- 使用 low 风险等级
- Monitor logs closely
- Have a kill switch ready

## License

MIT License - See LICENSE file for details

## Contributing

Contributions welcome! Please ensure:
- Tests pass (`cargo test`)
- No compiler warnings (`cargo clippy`)
- Code follows Rust conventions (`cargo fmt`)

## Support

For issues and questions:
- Open an issue on GitHub
- Check existing documentation
- Review example configurations
