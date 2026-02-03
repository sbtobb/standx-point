## Architecture
- **Position**: Data layer for API schemas.
- **Logic**: Enums + models + request/response structs -> re-export via mod.rs.
- **Constraints**: Data-only types; serde mappings must match API fields.

## Members
- `mod.rs`: Module wiring and public re-exports.
- `enums.rs`: API enums for sides, order types, status, and chains.
- `models.rs`: Core data models returned by API endpoints.
- `requests.rs`: HTTP request payload types.
- `responses.rs`: HTTP response payload types.
