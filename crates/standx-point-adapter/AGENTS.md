## Architecture
- **Position**: StandX adapter crate root.
- **Logic**: Crate surface -> module organization.
- **Constraints**: Keep types data-only and serde-aligned.

## Members
- `Cargo.toml`: Crate manifest and dependency definitions.
- `src/`: Rust source for the adapter crate.
  - `auth/`: Authentication module (JWT, Ed25519, wallet signing)
  - `http/`: HTTP client and endpoint implementations
  - `types/`: Request/response types and enums
  - `ws/`: WebSocket client for real-time data
- `tests/`: Integration tests for adapter components.
- `examples/`: Usage examples for the crate.

## API Usage Flow

### Authentication
1. Generate Ed25519 temporary keypair
2. Call `prepare_signin(chain, address)` to get signedData
3. Parse signedData JWT to extract message
4. Sign message with wallet (WalletSigner trait)
5. Call `login(signature, signedData)` to get JWT token
6. Store JWT token (valid for 7 days by default)

### Request Types

**Public Endpoints** (No authentication):
- Symbol info, price, depth book, recent trades, kline history

**JWT-Authenticated Endpoints**:
- Query orders, positions, balance, trades

**Body-Signature Endpoints** (JWT + Ed25519 signature):
- New order, cancel order, change leverage, transfer margin

## API Limits & Constraints

### WebSocket
- **Max Connection Duration**: 24 hours (auto-reconnect required)
- **Ping/Pong**: Server sends ping every 10 seconds
- **Timeout**: Connection dropped if no pong within 5 minutes

### Authentication
- **JWT Expiry**: 7 days default (configurable via expiresSeconds)
- **Key Format**: Ed25519 for request signing
- **Signature Format**: Base64-encoded Ed25519 signature

### Rate Limiting
- Not explicitly documented; implement conservative retry with exponential backoff
- Use StandxError::is_retryable() to determine retryable errors

## Conventions
- Use rust_decimal for price/quantity (not f64)
- All async functions use Tokio
- Error handling via StandxError enum
- Body signature format: "{version},{request_id},{timestamp},{payload}"
