## Architecture
- **Position**: Crate source root.
- **Logic**: lib.rs -> modules (types, auth, http, ws).
- **Constraints**: No business logic inside data types.

## Members
- `lib.rs`: Crate entrypoint and module re-exports.
- `auth/`: Authentication primitives and signing helpers.
- `http/`: HTTP client core and API endpoint modules.
- `types/`: Type definitions for API requests/responses and enums.
- `ws/`: WebSocket client and message types.
