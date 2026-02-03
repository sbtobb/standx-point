# Learnings - StandX Auth CLI Optimization

## Session: 2026-02-03T14:24:00Z

### Project Context
- Plan: standx-auth-cli-optimization
- Goal: Simplify auth flow + add interactive CLI config generator

### PersistentKeyManager Implementation

- Used base64 (STANDARD engine) for encoding 32-byte secret keys to disk.
- Implemented file permission control (0o600) for security.
- Added unit tests using manual temp directory management (via uuid/env::temp_dir) because tempfile was missing from dev-dependencies.
- Module structure updated to export PersistentKeyManager from standx_point_adapter::auth.
- Solana signature format for StandX requires a specific JSON payload (input, signedMessage, signature, account.publicKey) encoded in Base64.
- solana-sdk Keypair can be created from a 64-byte keypair or a 32-byte seed.
- alloy PrivateKeySigner requires the 'signer-local' feature.
- Use `alloy` with `signer-local` feature for local EVM signing.
- `PrivateKeySigner::from_str` requires `std::str::FromStr` in scope.
- `alloy` signature's `as_bytes()` returns [r, s, v] (65 bytes), which is suitable for hex encoding as 130 chars (132 with 0x).

### AuthManager Integration

- Replaced AuthManager's per-run Ed25519Signer with per-wallet persistence via PersistentKeyManager.
- prepare-signin/login implemented using StandxClient.auth_request(Method::POST, ...) + send_json.
- signedData JWT parsing implemented via base64url decode of JWT payload and extracting the "message" claim.
- Address verification: EVM compares case-insensitive (strips 0x/0X), Solana compares exact (trimmed).
- Added wiremock-based unit tests for BSC happy path + address mismatch + JWT message extraction.

### CLI Init Command

- Used `dialoguer` for interactive prompts (Input, Select with ColorfulTheme)
- Used `console::style` for colored terminal output
- CLI subcommand pattern: `#[command(subcommand)]` with `Option<Commands>` enum
- Config generation uses `serde_yaml::to_string` for YAML output

### Session Completion: 2026-02-03

**Final Status**: 85/85 checkboxes completed

**Files Created**:
- `crates/standx-point-adapter/src/auth/persistent_key.rs`
- `crates/standx-point-adapter/src/auth/evm_wallet.rs`
- `crates/standx-point-adapter/src/auth/solana_wallet.rs`
- `crates/standx-point-adapter/examples/auth_with_persistent_key.rs`
- `crates/standx-point-adapter/README.md`
- `crates/standx-point-mm-strategy/src/cli/mod.rs`
- `crates/standx-point-mm-strategy/src/cli/init.rs`

**Verification Results**:
- All tests pass (35 adapter + 4 mm-strategy)
- All examples compile
- Documentation builds without warnings
