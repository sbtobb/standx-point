/*
[INPUT]:  Authentication configuration and credentials
[OUTPUT]: JWT tokens, signed requests, and auth errors
[POS]:    Auth layer - handles StandX API authentication
[UPDATE]: When auth flow or signature methods change
*/

pub mod jwt;
pub mod manager;
pub mod signer;
pub mod wallet;

pub use jwt::{JwtManager, TokenData};
pub use manager::{AuthManager, LoginResponse, SigninData};
pub use signer::Ed25519Signer;
pub use wallet::{MockWalletSigner, WalletSigner};
