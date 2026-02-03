## Architecture
- **Position**: Auth layer for StandX adapter.
- **Logic**: Key generation/signing -> auth flows -> wallet signature abstraction.
- **Constraints**: No hardcoded keys; no wallet implementation secrets.

## Members
- `mod.rs`: Module wiring and public exports for auth.
- `signer.rs`: Ed25519 key management and request signing helpers.
- `jwt.rs`: JWT token storage and lifecycle helpers.
- `manager.rs`: Auth flow orchestration across prepare-signin/login and JWT storage.
- `wallet.rs`: Wallet signer trait and mock implementation for tests.

## Conventions (Optional)
- Keep crypto helpers deterministic in tests where possible.
