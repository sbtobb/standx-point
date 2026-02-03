/*
[INPUT]:  JWT tokens and expiration timestamps
[OUTPUT]: Token retrieval and expiration status
[POS]:    Auth layer - token lifecycle management
[UPDATE]: When adding token refresh or changing storage strategy
*/

use chrono::{DateTime, Duration, Utc};
use std::sync::{Arc, RwLock};

use crate::types::Chain;

/// Stored token data with metadata
#[derive(Debug, Clone)]
pub struct TokenData {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub wallet_address: String,
    pub chain: Chain,
}

/// Thread-safe JWT token manager
#[derive(Debug, Clone)]
pub struct JwtManager {
    data: Arc<RwLock<Option<TokenData>>>,
}

impl JwtManager {
    /// Create a new empty JWT manager
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(None)),
        }
    }

    /// Store a new token with expiration
    pub fn set_token(
        &self,
        token: String,
        expires_seconds: u64,
        wallet_address: String,
        chain: Chain,
    ) {
        let expires_at = Utc::now() + Duration::seconds(expires_seconds as i64);
        let token_data = TokenData {
            token,
            expires_at,
            wallet_address,
            chain,
        };

        let mut guard = self.data.write().unwrap();
        *guard = Some(token_data);
    }

    /// Get the current token if available
    pub fn get_token(&self) -> Option<String> {
        let guard = self.data.read().unwrap();
        guard.as_ref().map(|data| data.token.clone())
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let guard = self.data.read().unwrap();
        match guard.as_ref() {
            Some(data) => Utc::now() > data.expires_at,
            None => true,
        }
    }

    /// Get token data if available
    pub fn token_data(&self) -> Option<TokenData> {
        let guard = self.data.read().unwrap();
        guard.clone()
    }

    /// Clear the stored token
    pub fn clear(&self) {
        let mut guard = self.data.write().unwrap();
        *guard = None;
    }
}

impl Default for JwtManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_manager_is_empty() {
        let manager = JwtManager::new();
        assert!(manager.get_token().is_none());
        assert!(manager.is_expired());
    }

    #[test]
    fn test_set_and_get_token() {
        let manager = JwtManager::new();
        manager.set_token(
            "test_token".to_string(),
            3600,
            "0x123".to_string(),
            Chain::Bsc,
        );

        assert_eq!(manager.get_token(), Some("test_token".to_string()));
        assert!(!manager.is_expired());
    }

    #[test]
    fn test_clear_token() {
        let manager = JwtManager::new();
        manager.set_token(
            "test_token".to_string(),
            3600,
            "0x123".to_string(),
            Chain::Bsc,
        );

        manager.clear();
        assert!(manager.get_token().is_none());
        assert!(manager.is_expired());
    }
}
