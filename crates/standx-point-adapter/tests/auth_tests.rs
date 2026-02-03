/*
[INPUT]:  Mock authentication responses
[OUTPUT]: Test results for auth flow
[POS]:    Integration tests - authentication
[UPDATE]: When auth endpoints or flow changes
*/

mod common;

use common::{generate_test_keypair, mock_jwt_token, setup_mock_server};
use standx_point_adapter::{AuthManager, Chain, MockWalletSigner, StandxClient, WalletSigner};
use tokio_test::assert_ok;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn test_auth_manager_creation() {
    let client = assert_ok!(StandxClient::new());
    let auth_manager = AuthManager::new(client);
    let (public_key, secret_key) = generate_test_keypair();

    assert!(auth_manager.jwt_manager().is_expired());
    assert!(!auth_manager.signer().public_key_base58().is_empty());
    assert_eq!(public_key.len(), 32);
    assert_eq!(secret_key.len(), 32);
}

#[tokio::test]
async fn test_mock_wallet_signer() {
    let wallet = MockWalletSigner::new(Chain::Bsc, "0x1234567890abcdef", "0xmock_signature");

    assert_eq!(wallet.chain(), Chain::Bsc);
    assert_eq!(wallet.address(), "0x1234567890abcdef");

    let signature = assert_ok!(wallet.sign_message("test").await);
    assert_eq!(signature, "0xmock_signature");
}

#[tokio::test]
async fn test_prepare_signin_wiremock_scaffold() {
    let server = setup_mock_server().await;
    let client = assert_ok!(StandxClient::new());
    let auth_manager = AuthManager::new(client);

    let expected_signed_data = mock_jwt_token();
    let response_body = serde_json::json!({
        "signedData": expected_signed_data.clone(),
    });
    Mock::given(method("POST"))
        .and(path("/v1/offchain/prepare-signin"))
        .and(query_param("chain", "bsc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&server)
        .await;

    let request_id = auth_manager.signer().public_key_base58();
    let body = serde_json::json!({
        "address": "0x1234567890abcdef",
        "requestId": request_id,
    });

    let url = format!("{}/v1/offchain/prepare-signin?chain=bsc", server.uri());
    let response = assert_ok!(reqwest::Client::new().post(url).json(&body).send().await);
    assert!(response.status().is_success());

    let payload: serde_json::Value = assert_ok!(response.json().await);
    assert_eq!(
        payload.get("signedData").and_then(|value| value.as_str()),
        Some(expected_signed_data.as_str())
    );
}
