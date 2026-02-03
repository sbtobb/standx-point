/*
[INPUT]:  HTTP configuration (base URLs, timeouts, credentials)
[OUTPUT]: Configured reqwest client ready for API calls
[POS]:    HTTP layer - core client implementation
[UPDATE]: When adding connection options or changing client behavior
*/

use crate::types::Chain;
use reqwest::{Client, Method, RequestBuilder, Url};
use std::time::Duration;

/// Base URLs for StandX API
const AUTH_BASE_URL: &str = "https://api.standx.com";
const TRADING_BASE_URL: &str = "https://perps.standx.com";

/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub timeout: Duration,
    pub connect_timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
        }
    }
}

/// Credentials for authenticated requests
#[derive(Debug, Clone)]
pub struct Credentials {
    pub jwt_token: String,
    pub wallet_address: String,
    pub chain: Chain,
}

/// Main HTTP client for StandX API
#[derive(Debug)]
#[allow(dead_code)]
pub struct StandxClient {
    http_client: Client,
    auth_base_url: Url,
    trading_base_url: Url,
    credentials: Option<Credentials>,
}

#[allow(dead_code)]
impl StandxClient {
    /// Create a new client with default configuration
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_config(ClientConfig::default())
    }

    /// Create a new client with custom configuration
    pub fn with_config(config: ClientConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let http_client = Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .build()?;

        Ok(Self {
            http_client,
            auth_base_url: Url::parse(AUTH_BASE_URL)?,
            trading_base_url: Url::parse(TRADING_BASE_URL)?,
            credentials: None,
        })
    }

    /// Set credentials for authenticated requests
    pub fn set_credentials(&mut self, credentials: Credentials) {
        self.credentials = Some(credentials);
    }

    /// Get credentials if set
    pub fn credentials(&self) -> Option<&Credentials> {
        self.credentials.as_ref()
    }

    /// Build full URL for auth endpoints
    fn auth_url(&self, endpoint: &str) -> Result<Url, url::ParseError> {
        self.auth_base_url.join(endpoint)
    }

    /// Build full URL for trading endpoints
    fn trading_url(&self, endpoint: &str) -> Result<Url, url::ParseError> {
        self.trading_base_url.join(endpoint)
    }

    /// Build request builder for auth endpoints
    pub(crate) fn auth_request(
        &self,
        method: Method,
        endpoint: &str,
    ) -> Result<RequestBuilder, url::ParseError> {
        let url = self.auth_url(endpoint)?;
        Ok(self.http_client.request(method, url))
    }

    /// Build request builder for trading endpoints
    pub(crate) fn trading_request(
        &self,
        method: Method,
        endpoint: &str,
    ) -> Result<RequestBuilder, url::ParseError> {
        let url = self.trading_url(endpoint)?;
        Ok(self.http_client.request(method, url))
    }
}
