# StandX Market Making Strategy Bot

面向生产的 StandX 做市机器人，支持多账户、共享行情流与谨慎风险管理。

## Features

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

#### CLI 模式
使用 `--config` 指定配置文件，适合服务器长期运行。
```bash
# Run with config
./target/release/standx-point-mm-strategy --config config.yaml

# Dry run (validate config without trading)
./target/release/standx-point-mm-strategy --config config.yaml --dry-run
```

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
