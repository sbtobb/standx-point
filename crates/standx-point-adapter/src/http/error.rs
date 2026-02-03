/*
[INPUT]:  Error sources (HTTP, API, serialization, auth, WebSocket)
[OUTPUT]: Structured error types with context and retry hints
[POS]:    Error handling layer - unified error types for entire crate
[UPDATE]: When adding new error sources or improving error messages
*/

use reqwest::StatusCode;
use thiserror::Error;

/// Main error type for StandX adapter
#[derive(Error, Debug)]
pub enum StandxError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// API returned an error response
    #[error("API error (code {code}): {message}")]
    Api { code: i32, message: String },

    /// Authentication failed
    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    /// JWT token is expired
    #[error("JWT token expired, please re-authenticate")]
    TokenExpired,

    /// Request signature is invalid
    #[error("Invalid request signature")]
    InvalidSignature,

    /// Serialization/deserialization failed
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// URL parsing failed
    #[error("Invalid URL: {0}")]
    UrlParse(#[from] url::ParseError),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Invalid response from server
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded, retry after {retry_after}s")]
    RateLimit { retry_after: u64 },

    /// Connection timeout
    #[error("Connection timeout after {duration}s")]
    Timeout { duration: u64 },
}

impl StandxError {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            StandxError::Http(_)
                | StandxError::RateLimit { .. }
                | StandxError::Timeout { .. }
                | StandxError::WebSocket(_)
                | StandxError::InvalidResponse(_)
        )
    }

    /// Get retry delay in seconds (if retryable)
    pub fn retry_delay(&self) -> Option<u64> {
        match self {
            StandxError::RateLimit { retry_after } => Some(*retry_after),
            StandxError::Timeout { .. } => Some(1),
            _ => None,
        }
    }

    /// Check if error indicates authentication failure
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            StandxError::Authentication { .. }
                | StandxError::TokenExpired
                | StandxError::InvalidSignature
        )
    }

    /// Create an API error from status code and message
    pub fn api_error(status: StatusCode, message: impl Into<String>) -> Self {
        StandxError::Api {
            code: status.as_u16() as i32,
            message: message.into(),
        }
    }
}

/// Result type alias for StandX operations
pub type Result<T> = std::result::Result<T, StandxError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retryable() {
        let timeout_err = StandxError::Timeout { duration: 30 };
        assert!(timeout_err.is_retryable());
        assert_eq!(timeout_err.retry_delay(), Some(1));

        let auth_err = StandxError::TokenExpired;
        assert!(!auth_err.is_retryable());
    }

    #[test]
    fn test_error_is_auth_error() {
        assert!(StandxError::TokenExpired.is_auth_error());
        assert!(StandxError::InvalidSignature.is_auth_error());
        assert!(!StandxError::Timeout { duration: 30 }.is_auth_error());
    }

    #[test]
    fn test_api_error_creation() {
        let err = StandxError::api_error(StatusCode::BAD_REQUEST, "Invalid symbol");
        match err {
            StandxError::Api { code, message } => {
                assert_eq!(code, 400);
                assert_eq!(message, "Invalid symbol");
            }
            _ => panic!("Expected Api error variant"),
        }
    }
}
