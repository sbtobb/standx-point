# StandX Market Making Strategy Bot

A production-ready market making bot for StandX exchange with multi-account support, shared market data streams, and conservative risk management.

## Features

- **Multi-Account Support**: Run multiple trading tasks simultaneously with isolated state
- **Shared Market Data**: Single WebSocket connection feeds all tasks via `tokio::sync::watch` channels
- **Conservative Strategy**: 5-10 bps from mark price for safe market making
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
tasks:
  - id: "btc-mm"
    symbol: "BTC-USD"
    credentials:
      jwt_token: "your-jwt-token"
      signing_key: "your-ed25519-key"
    risk:
      level: "conservative"
      max_position_usd: "50000"
      price_jump_threshold_bps: 5
    sizing:
      base_qty: "0.1"
      tiers: 2
```

### 2. Run the Bot

```bash
# Build release binary
cargo build --release --package standx-point-mm-strategy

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
| `credentials.jwt_token` | String | JWT authentication token |
| `credentials.signing_key` | String | Ed25519 private key (base64) |
| `risk.level` | String | "conservative", "moderate", or "aggressive" |
| `risk.max_position_usd` | String | Maximum position size in USD |
| `risk.price_jump_threshold_bps` | u32 | Price velocity threshold |
| `sizing.base_qty` | String | Base order quantity |
| `sizing.tiers` | u8 | Number of price tiers (1-3) |

### Risk Levels

- **Conservative**: 5-10 bps from mark price, safest
- **Moderate**: 3-8 bps from mark price, balanced
- **Aggressive**: 0-5 bps from mark price, highest points

## Strategy Details

### Market Making Logic

1. **Price Monitoring**: Watches mark price via WebSocket
2. **Quote Placement**: Places bilateral PostOnly orders at configured bps offset
3. **Tier Management**: Maintains L1 (0-5bps), L2 (5-10bps), L3 (10-20bps) ladders
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
- Use conservative risk settings
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
