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

### 1. Build the Binary

```bash
# Build in release mode (recommended for production)
cargo build -p standx-point-mm-strategy --release

# Binary will be at: ./target/release/standx-point-mm-strategy
```

### 2. Create Configuration File

Create a YAML configuration file (see `examples/single_task.yaml`):

```yaml
accounts:
  - id: "account-1"
    private_key: "your-wallet-private-key"
    chain: "bsc"
tasks:
  - id: "btc-mm"
    symbol: "BTC-USD"
    account_id: "account-1"
    risk:
      level: "low"
      budget_usd: "50000"
```

### 3. One-Click Start

#### Quick Start (Development/Test)

```bash
# Run directly with cargo (development only)
cargo run -p standx-point-mm-strategy -- --config path/to/config.yaml
```

#### Environment Variable Startup

Use `--env` to load configuration from environment variables:

```bash
export STANDX_MM_PRIVATE_KEY="your-wallet-private-key"
export STANDX_MM_SYMBOL="BTC-USD"
export STANDX_MM_RISK_LEVEL="low"
export STANDX_MM_BUDGET_USD="50000"
# Optional:
# export STANDX_MM_CHAIN="bsc"
# export STANDX_MM_ACCOUNT_ID="account-1"
# export STANDX_MM_TASK_ID="task-btc"
# export STANDX_MM_GUARD_CLOSE_ENABLED="false"
# export STANDX_MM_TP_BPS="30"
# export STANDX_MM_SL_BPS="20"

standx-point-mm-strategy --env --dry-run
```

#### Production Deployment

```bash
# 1. Build release binary
cargo build -p standx-point-mm-strategy --release

# 2. Copy binary to deployment location
cp ./target/release/standx-point-mm-strategy /usr/local/bin/

# 3. Create config directory
mkdir -p /etc/standx-point-mm-strategy

# 4. Copy configuration
cp config.yaml /etc/standx-point-mm-strategy/

# 5. Create systemd service (optional but recommended)
# See "Production Deployment" section below

# 6. Start the service
standx-point-mm-strategy --config /etc/standx-point-mm-strategy/config.yaml
```

### 4. Verify Startup

```bash
# Validate configuration without trading (dry run)
standx-point-mm-strategy --config config.yaml --dry-run

# Start with detailed logging
RUST_LOG=debug standx-point-mm-strategy --config config.yaml

# View logs
tail -f logs/standx-point-mm-strategy.log
```

## CLI Options

```
standx-point-mm-strategy [OPTIONS] [--config <PATH>] [--env] [--dry-run]

Options:
  -c, --config <PATH>     Path to YAML configuration file
      --env              Load configuration from environment variables
      --dry-run          Validate configuration without trading
  -l, --log-level <LEVEL>  Log level: trace, debug, info, warn, error [default: info]
  -h, --help            Print help
  -V, --version         Print version

Subcommands:
  init     Initialize a new configuration file
  migrate  Migrate existing state
```

## Configuration Reference

### Configuration File Structure

```yaml
# Account credentials (can have multiple accounts)
accounts:
  - id: "main-account"                 # Unique account identifier
    private_key: "0x..."               # Wallet private key
    # Optional legacy overrides:
    # jwt_token: "eyJ..."
    # signing_key: "base64-encoded-key"
    chain: "bsc"                       # Chain: "bsc" or "solana"

# Trading tasks (can have multiple tasks)
tasks:
  - id: "btc-mm"                       # Unique task identifier
    symbol: "BTC-USD"                  # Trading pair
    account_id: "main-account"          # Which account to use
    risk:
      level: "low"                     # Risk level: low/medium/high/xhigh
      budget_usd: "100000"             # Budget in USD for quoting
      guard_close_enabled: false        # Optional position guard close toggle
      tp_bps: "30"                      # Optional take-profit distance in bps
      sl_bps: "20"                      # Optional stop-loss distance in bps
```

### Account Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | String | Yes | Unique account identifier, referenced by tasks |
| `private_key` | String | Yes | Wallet private key for authentication |
| `jwt_token` | String | No | JWT authentication token from StandX (legacy override) |
| `signing_key` | String | No | Ed25519 private key for request signing (base64, legacy override) |
| `chain` | String | Yes | Blockchain: `"bsc"` or `"solana"` |

### Task Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | String | Yes | Unique task identifier |
| `symbol` | String | Yes | Trading pair (e.g., "BTC-USD") |
| `account_id` | String | Yes | Account identifier from `accounts` section |
| `risk.level` | String | Yes | Risk level: `"low"`, `"medium"`, `"high"`, or `"xhigh"` |
| `risk.budget_usd` | String | Yes | Budget in USD for quoting (名义金额) |
| `risk.guard_close_enabled` | Bool | No | Enable position guard close orders (default: false) |
| `risk.tp_bps` | String | No | Take-profit distance in bps (`"1"` = 0.01%) |
| `risk.sl_bps` | String | No | Stop-loss distance in bps (`"1"` = 0.01%) |

当 `risk.tp_bps`/`risk.sl_bps` 提供时，做市挂单会在提交时携带止盈止损触发价，成交后由系统自动创建对应的减仓单。
当未提供时，默认 `tp_bps = maker_fee + taker_fee`（bps），`sl_bps` 按风险等级放大：low=2x、medium=3x、high=4x、xhigh=5x。

### Risk Level Details

| Level | Tiers | Band (bps) | Description |
|-------|-------|------------|-------------|
| **Low** | 5 | 5-30 | 最保守，适合初学者 |
| **Medium** | 3 | 5-15 | 中等风险 |
| **High** | 2 | 5-10 | 激进策略 |
| **XHigh** | 1 | 5-8 | 最高风险 |

### Budget Sizing Calculation

The `budget_usd` represents **双边挂单名义金额总和** (total notional value for both sides):

```
单边预算 = budget_usd / 2
基础数量 = base_qty = per_side_budget / mark_price / total_weight
分层数量 = tier_qty = base_qty * tier_weight * band_multiplier
```

Final order quantity is adjusted by:
- `qty_tick_decimals` alignment
- `min_order_qty` threshold (values below are zeroed)

### Example Configurations

#### Single Task (Low Risk)

```yaml
accounts:
  - id: "main"
    private_key: "0x..."
    chain: "bsc"
tasks:
  - id: "btc-mm"
    symbol: "BTC-USD"
    account_id: "main"
    risk:
      level: "low"
      budget_usd: "50000"
```

#### Multiple Tasks (Different Risk Levels)

```yaml
accounts:
  - id: "main"
    private_key: "0x..."
    chain: "bsc"
tasks:
  - id: "btc-low"
    symbol: "BTC-USD"
    account_id: "main"
    risk:
      level: "low"
      budget_usd: "100000"
  - id: "btc-high"
    symbol: "BTC-USD"
    account_id: "main"
    risk:
      level: "high"
      budget_usd: "50000"
  - id: "eth-mm"
    symbol: "ETH-USD"
    account_id: "main"
    risk:
      level: "medium"
      budget_usd: "75000"
```

## Production Deployment

### Systemd Service

Create `/etc/systemd/system/standx-point-mm-strategy.service`:

```ini
[Unit]
Description=StandX Point MM Strategy Bot
After=network.target

[Service]
Type=simple
User=standx
Group=standx
WorkingDirectory=/etc/standx-point-mm-strategy
ExecStart=/usr/local/bin/standx-point-mm-strategy --config /etc/standx-point-mm-strategy/config.yaml
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/standx-point-mm-strategy

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable standx-point-mm-strategy
sudo systemctl start standx-point-mm-strategy

# Check status
sudo systemctl status standx-point-mm-strategy

# View logs
sudo journalctl -u standx-point-mm-strategy -f
```

## Monitoring

### Log Files

Logs are written to `logs/standx-point-mm-strategy.log` with daily rotation:

```bash
# View current log
tail -f logs/standx-point-mm-strategy.log

# View specific date
tail -f logs/standx-point-mm-strategy.log.2026-02-08
```

### Log Levels

| Level | Use Case |
|-------|----------|
| `trace` | Very detailed debugging |
| `debug` | Development debugging |
| `info` | Normal operation (default) |
| `warn` | Warnings and elevated risk |
| `error` | Errors requiring attention |

### Health Checks

```bash
# Check if process is running
pgrep -f standx-point-mm-strategy

# Monitor resource usage
htop -p $(pgrep -f standx-point-mm-strategy)

# Check network connections
netstat -tulpn | grep standx-point
```

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
