# Decisions - StandX Auth CLI Optimization

## Session: 2026-02-03T14:24:00Z

- Added StandxError::Internal variant to handle unexpected failures like serialization errors in signing.
- Used solana-sdk for Solana wallet implementation instead of raw ed25519-dalek to stay consistent with Solana ecosystem patterns and ensure correct public key derivation.
- Implemented `EvmWalletSigner` using `alloy` crate for EVM compatibility.
- Used `StandxError::Config` for private key parsing errors and `StandxError::Internal` for signing errors.
- Support both 0x-prefixed and non-prefixed hex private keys in constructor.

- Kept `AuthManager::new(client)` for compatibility and added `AuthManager::new_with_key_dir(client, key_dir)` for explicit key storage control.
- Changed `AuthManager::authenticate` receiver to `&self` (JwtManager is internally synchronized).
- Implemented JWT message extraction without adding a jsonwebtoken dependency (base64url decode + JSON parse).
- Derived `Debug` for PersistentKeyManager to keep `AuthManager: Debug`.
