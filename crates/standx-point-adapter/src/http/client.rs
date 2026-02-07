/*
[INPUT]:  HTTP configuration (base URLs, timeouts, credentials, request signer)
[OUTPUT]: Configured reqwest client ready for API calls
[POS]:    HTTP layer - core client implementation
[UPDATE]: When adding connection options or changing client behavior
*/

use super::error::{Result as HttpResult, StandxError};
use super::signature::{
    BodySignature, RequestSigner, DEFAULT_SIGNATURE_VERSION, HEADER_REQUEST_ID,
    HEADER_REQUEST_SIGNATURE, HEADER_REQUEST_TIMESTAMP, HEADER_REQUEST_VERSION,
};
use crate::auth::Ed25519Signer;
use crate::types::Chain;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Method, RequestBuilder, Url};
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;
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
    request_signer: Option<RequestSigner>,
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
            request_signer: None,
        })
    }

    /// Create a new client with custom base URLs (useful for tests).
    pub fn with_config_and_base_urls(
        config: ClientConfig,
        auth_base_url: &str,
        trading_base_url: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let http_client = Client::builder()
            .timeout(config.timeout)
            .connect_timeout(config.connect_timeout)
            .build()?;

        Ok(Self {
            http_client,
            auth_base_url: Url::parse(auth_base_url)?,
            trading_base_url: Url::parse(trading_base_url)?,
            credentials: None,
            request_signer: None,
        })
    }

    /// Set credentials for authenticated requests
    pub fn set_credentials(&mut self, credentials: Credentials) {
        self.credentials = Some(credentials);
    }

    /// Set Ed25519 request signer for body-signature endpoints.
    pub fn set_request_signer(&mut self, signer: Ed25519Signer) {
        self.request_signer = Some(RequestSigner::new(signer));
    }

    /// Set credentials and request signer in one call.
    pub fn set_credentials_and_signer(&mut self, credentials: Credentials, signer: Ed25519Signer) {
        self.credentials = Some(credentials);
        self.request_signer = Some(RequestSigner::new(signer));
    }

    /// Get credentials if set
    pub fn credentials(&self) -> Option<&Credentials> {
        self.credentials.as_ref()
    }

    /// Get request signer if set
    pub fn request_signer(&self) -> Option<&RequestSigner> {
        self.request_signer.as_ref()
    }

    pub(crate) fn require_credentials(&self) -> HttpResult<&Credentials> {
        self.credentials
            .as_ref()
            .ok_or_else(|| StandxError::Config("credentials not set".to_string()))
    }

    pub(crate) fn require_request_signer(&self) -> HttpResult<&RequestSigner> {
        self.request_signer
            .as_ref()
            .ok_or_else(|| StandxError::Config("request signer not set".to_string()))
    }

    pub(crate) fn trading_request_with_jwt(
        &self,
        method: Method,
        endpoint: &str,
    ) -> HttpResult<RequestBuilder> {
        let credentials = self.require_credentials()?;
        let builder = self.trading_request(method, endpoint)?;
        Ok(builder.header(AUTHORIZATION, format!("Bearer {}", credentials.jwt_token)))
    }

    pub(crate) fn trading_post_with_jwt_and_signature(
        &self,
        endpoint: &str,
        payload: &str,
        timestamp: u64,
    ) -> HttpResult<(RequestBuilder, BodySignature)> {
        let signer = self.require_request_signer()?;
        let signature = signer.sign_payload(payload, timestamp);

        let builder = self
            .trading_request_with_jwt(Method::POST, endpoint)?
            .header(CONTENT_TYPE, "application/json")
            .header(HEADER_REQUEST_VERSION, DEFAULT_SIGNATURE_VERSION)
            .header(HEADER_REQUEST_ID, signature.request_id.clone())
            .header(HEADER_REQUEST_TIMESTAMP, signature.timestamp.to_string())
            .header(HEADER_REQUEST_SIGNATURE, signature.signature.clone());

        Ok((builder, signature))
    }

    pub(crate) async fn send_json<T: DeserializeOwned>(&self, builder: RequestBuilder) -> HttpResult<T> {
        const MAX_RETRIES: usize = 3;
        let mut retries = 0;
        
        loop {
            let result = async {
                let response = builder.try_clone().ok_or_else(|| StandxError::Internal("Builder cannot be cloned".to_string()))?.send().await?;
                let status = response.status();
                let body = response.text().await?;
                
                if status.is_success() {
                    return Ok(serde_json::from_str::<T>(&body)?);
                }
                
                if status == reqwest::StatusCode::UNAUTHORIZED {
                    return Err(StandxError::TokenExpired);
                }
                
                let message = match serde_json::from_str::<JsonValue>(&body) {
                    Ok(JsonValue::Object(map)) => map
                        .get("message")
                        .and_then(|value| value.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| body.clone()),
                    _ => body.clone(),
                };
                
                if status == reqwest::StatusCode::FORBIDDEN
                    && message.to_ascii_lowercase().contains("signature")
                {
                    return Err(StandxError::InvalidSignature);
                }
                
                Err(StandxError::api_error(status, message))
            }.await;
            
            match result {
                Ok(v) => return Ok(v),
                Err(e) => {
                    retries += 1;
                    if retries > MAX_RETRIES {
                        return Err(e);
                    }
                    // Wait for a short time before retrying
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
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
