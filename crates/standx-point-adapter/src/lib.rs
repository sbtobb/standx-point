/*
[INPUT]:  Crate modules and public type definitions
[OUTPUT]: Public StandX adapter crate surface
[POS]:    Crate root - module wiring
[UPDATE]: When public modules or exports change
*/

pub mod auth;
pub mod http;
pub mod types;
pub mod ws;

// Re-export commonly used types from auth
pub use auth::{
    AuthManager,
    Ed25519Signer,
    JwtManager,
    MockWalletSigner,
    TokenData,
    WalletSigner,
};

// Re-export commonly used types from http
pub use http::{
    ClientConfig,
    Credentials,
    RequestSigner,
    Result,
    StandxClient,
    StandxError,
};

// Re-export all types
pub use types::*;

// Re-export commonly used types from ws
pub use ws::{
    DepthBookData,
    OrderUpdateData,
    PriceData,
    StandxWebSocket,
    WebSocketMessage,
};
