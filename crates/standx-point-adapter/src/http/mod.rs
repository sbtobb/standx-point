/*
[INPUT]:  HTTP client configuration and API endpoints
[OUTPUT]: HTTP responses and typed API results
[POS]:    HTTP layer - REST API communication
[UPDATE]: When adding new endpoints or changing client behavior
*/

pub mod client;
pub mod error;
pub mod public;
pub mod signature;
pub mod trade;
pub mod user;

pub use error::{Result, StandxError};
pub use signature::RequestSigner;

pub use client::{ClientConfig, Credentials, StandxClient};
