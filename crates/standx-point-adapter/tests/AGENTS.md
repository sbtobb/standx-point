## Architecture
- **Position**: Integration test suite for standx-point-adapter.
- **Logic**: Mock server setup -> request/response assertions -> component sanity checks.
- **Constraints**: No external API calls; keep scenarios minimal.

## Members
- `auth_tests.rs`: Integration tests for auth manager creation and wallet signer behavior.
- `http_tests.rs`: Integration tests for client config, credentials, and HTTP mock scaffolds.
- `ws_tests.rs`: Integration tests for WebSocket client initialization behavior.
- `common/`: Shared test utilities and mock helpers.

## Conventions (Optional)
- Use wiremock for HTTP mocking.
- Use tokio-test utilities for async assertions.
