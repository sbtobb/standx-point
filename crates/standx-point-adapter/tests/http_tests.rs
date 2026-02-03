/*
[INPUT]:  Mock HTTP responses
[OUTPUT]: Test results for HTTP client
[POS]:    Integration tests - HTTP endpoints
[UPDATE]: When HTTP endpoints change
*/

mod common;

use common::{generate_test_keypair, mock_jwt_token, setup_mock_server};
use standx_point_adapter::{Chain, ClientConfig, Credentials, StandxClient, StandxError};
use tokio_test::assert_ok;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[test]
fn test_client_creation() {
    let _client = assert_ok!(StandxClient::new());
    let (public_key, secret_key) = generate_test_keypair();
    assert_eq!(public_key.len(), 32);
    assert_eq!(secret_key.len(), 32);
}

#[test]
fn test_client_with_config() {
    let config = ClientConfig::default();
    let _client = assert_ok!(StandxClient::with_config(config));
}

#[test]
fn test_client_credentials_roundtrip() {
    let mut client = assert_ok!(StandxClient::new());
    let credentials = Credentials {
        jwt_token: mock_jwt_token(),
        wallet_address: "0x1234567890abcdef".to_string(),
        chain: Chain::Bsc,
    };

    client.set_credentials(credentials.clone());
    let stored = client.credentials().expect("credentials should be set");

    assert_eq!(stored.jwt_token, credentials.jwt_token);
    assert_eq!(stored.wallet_address, credentials.wallet_address);
    assert_eq!(stored.chain, credentials.chain);
}

#[test]
fn test_error_retryable() {
    let timeout_err = StandxError::Timeout { duration: 30 };
    assert!(timeout_err.is_retryable());

    let auth_err = StandxError::TokenExpired;
    assert!(!auth_err.is_retryable());
}

#[tokio::test]
async fn test_wiremock_basic_get() {
    let server = setup_mock_server().await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok",
        })))
        .mount(&server)
        .await;

    let url = format!("{}/health", server.uri());
    let response = assert_ok!(reqwest::get(url).await);
    assert!(response.status().is_success());

    let body: serde_json::Value = assert_ok!(response.json().await);
    assert_eq!(body.get("status").and_then(|value| value.as_str()), Some("ok"));
}
