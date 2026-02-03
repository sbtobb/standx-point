/*
[INPUT]:  Authentication configuration and credentials
[OUTPUT]: JWT tokens, signed requests, and auth errors
[POS]:    Auth layer - handles StandX API authentication
[UPDATE]: When auth flow or signature methods change
*/

pub mod evm_wallet;
pub mod jwt;
pub mod manager;
pub mod persistent_key;
pub mod signer;
pub mod solana_wallet;
pub mod wallet;

pub use evm_wallet::EvmWalletSigner;
pub use jwt::{JwtManager, TokenData};
pub use manager::{AuthManager, LoginResponse, SigninData};
pub use persistent_key::PersistentKeyManager;
pub use signer::Ed25519Signer;
pub use solana_wallet::SolanaWalletSigner;
pub use wallet::{MockWalletSigner, WalletSigner};
