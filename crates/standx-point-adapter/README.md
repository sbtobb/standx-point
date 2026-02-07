# StandX Point Adapter

Rust adapter for StandX Point API.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
standx-point-adapter = { path = "../standx-point-adapter" }
```

## Simplified Authentication

### Automatic Key Management

The `PersistentKeyManager` automatically handles ed25519 session key storage:

```rust
use standx_point_adapter::auth::AuthManager;

// Keys stored in ./.standx-config/keys/{wallet_address}_ed25519.key
let auth = AuthManager::new_with_key_dir(client, "./.standx-config/keys");

// List stored accounts
let accounts = auth.list_stored_accounts();
```

### One-Line Authentication

```rust
let response = auth.authenticate_with_wallet(
    "0x1234...",  // wallet address
    "0xprivate_key_hex",  // private key
    Chain::Bsc
).await?;
```

### Wallet Signers

- `EvmWalletSigner` - For BSC (EVM) chains
- `SolanaWalletSigner` - For Solana chains

### Security Notes

- Ed25519 keys are stored with 0o600 permissions
- Never commit `.standx-config/keys/` to version control
- Add `.standx-config/` to your `.gitignore`
