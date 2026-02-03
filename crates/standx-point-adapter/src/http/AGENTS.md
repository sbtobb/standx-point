## Architecture
- **Position**: HTTP layer for REST API communication.
- **Logic**: Client config + request building -> HTTP execution -> error mapping.
- **Constraints**: No automatic retry logic; avoid business state tracking.

## Members
- `mod.rs`: Module wiring and public re-exports.
- `client.rs`: HTTP client configuration and request primitives.
- `error.rs`: Unified error types for HTTP operations.
- `public.rs`: Public market data endpoints (no auth required).
- `signature.rs`: Body signature generator for authenticated trading requests.
- `trade.rs`: Trading endpoint stubs requiring auth and body signatures.
- `user.rs`: User account query endpoints (orders, positions, balance) requiring JWT auth.
