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
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Match, Mock, Request, ResponseTemplate};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rust_decimal::Decimal;
use standx_point_adapter::http::signature::{
    HEADER_REQUEST_ID, HEADER_REQUEST_SIGNATURE, HEADER_REQUEST_TIMESTAMP, HEADER_REQUEST_VERSION,
};
use standx_point_adapter::{Ed25519Signer, NewOrderRequest, OrderStatus, OrderType, Side, TimeInForce};
use std::str;

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

#[derive(Clone)]
struct ValidBodySignatureMatcher {
    secret_key: [u8; 32],
}

impl Match for ValidBodySignatureMatcher {
    fn matches(&self, request: &Request) -> bool {
        let version = match request.headers.get(HEADER_REQUEST_VERSION) {
            Some(value) => match value.to_str() {
                Ok(s) => s,
                Err(_) => return false,
            },
            None => return false,
        };

        let request_id = match request.headers.get(HEADER_REQUEST_ID) {
            Some(value) => match value.to_str() {
                Ok(s) => s,
                Err(_) => return false,
            },
            None => return false,
        };

        let timestamp_str = match request.headers.get(HEADER_REQUEST_TIMESTAMP) {
            Some(value) => match value.to_str() {
                Ok(s) => s,
                Err(_) => return false,
            },
            None => return false,
        };
        let timestamp: u64 = match timestamp_str.parse() {
            Ok(v) => v,
            Err(_) => return false,
        };

        let signature = match request.headers.get(HEADER_REQUEST_SIGNATURE) {
            Some(value) => match value.to_str() {
                Ok(s) => s,
                Err(_) => return false,
            },
            None => return false,
        };

        let payload = match str::from_utf8(&request.body) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let message = format!("{version},{request_id},{timestamp},{payload}");
        let signer = Ed25519Signer::from_secret_key(&self.secret_key);
        let expected = {
            let sig = signer.sign(message.as_bytes());
            BASE64.encode(sig.to_bytes())
        };

        signature == expected
    }
}

#[tokio::test]
async fn test_http_user_endpoints_send_bearer_jwt() {
    let server = setup_mock_server().await;
    let base_url = server.uri();

    let jwt = mock_jwt_token();

    // query_orders
    Mock::given(method("GET"))
        .and(path("/api/query_orders"))
        .and(query_param("symbol", "BTC-USD"))
        .and(query_param("status", "filled"))
        .and(query_param("limit", "10"))
        .and(header("authorization", format!("Bearer {jwt}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "page_size": 1,
            "result": [],
            "total": 0,
        })))
        .mount(&server)
        .await;

    // query_open_orders
    Mock::given(method("GET"))
        .and(path("/api/query_open_orders"))
        .and(query_param("symbol", "BTC-USD"))
        .and(header("authorization", format!("Bearer {jwt}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "page_size": 1,
            "result": [],
            "total": 0,
        })))
        .mount(&server)
        .await;

    // query_positions
    Mock::given(method("GET"))
        .and(path("/api/query_positions"))
        .and(query_param("symbol", "BTC-USD"))
        .and(header("authorization", format!("Bearer {jwt}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    // query_balance
    Mock::given(method("GET"))
        .and(path("/api/query_balance"))
        .and(header("authorization", format!("Bearer {jwt}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "isolated_balance": "0",
            "isolated_upnl": "0",
            "cross_balance": "0",
            "cross_margin": "0",
            "cross_upnl": "0",
            "locked": "0",
            "cross_available": "0",
            "balance": "0",
            "upnl": "0",
            "equity": "0",
            "pnl_freeze": "0"
        })))
        .mount(&server)
        .await;

    let mut client = assert_ok!(StandxClient::with_config_and_base_urls(
        ClientConfig::default(),
        &base_url,
        &base_url
    ));
    client.set_credentials(Credentials {
        jwt_token: jwt.clone(),
        wallet_address: "0x1234567890abcdef".to_string(),
        chain: Chain::Bsc,
    });

    let orders = assert_ok!(client.query_orders(Some("BTC-USD"), Some(OrderStatus::Filled), Some(10)).await);
    assert_eq!(orders.result.len(), 0);

    let open_orders = assert_ok!(client.query_open_orders(Some("BTC-USD")).await);
    assert_eq!(open_orders.result.len(), 0);

    let positions = assert_ok!(client.query_positions(Some("BTC-USD")).await);
    assert!(positions.is_empty());

    let balance = assert_ok!(client.query_balance().await);
    assert_eq!(balance.balance, Decimal::ZERO);
}

#[tokio::test]
async fn test_query_open_orders_defaults_missing_total() {
    let server = setup_mock_server().await;
    let base_url = server.uri();

    let jwt = mock_jwt_token();

    Mock::given(method("GET"))
        .and(path("/api/query_open_orders"))
        .and(query_param("symbol", "BTC-USD"))
        .and(header("authorization", format!("Bearer {jwt}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "page_size": 1,
            "result": [],
        })))
        .mount(&server)
        .await;

    let mut client = assert_ok!(StandxClient::with_config_and_base_urls(
        ClientConfig::default(),
        &base_url,
        &base_url
    ));
    client.set_credentials(Credentials {
        jwt_token: jwt.clone(),
        wallet_address: "0x1234567890abcdef".to_string(),
        chain: Chain::Bsc,
    });

    let open_orders = assert_ok!(client.query_open_orders(Some("BTC-USD")).await);
    assert_eq!(open_orders.result.len(), 0);
    assert_eq!(open_orders.total, 0);
}

#[tokio::test]
async fn test_http_trading_endpoints_send_body_signature_headers() {
    let server = setup_mock_server().await;
    let base_url = server.uri();

    let jwt = mock_jwt_token();
    let secret_key = [7u8; 32];
    let signer = Ed25519Signer::from_secret_key(&secret_key);

    let signature_matcher = ValidBodySignatureMatcher { secret_key };

    Mock::given(method("POST"))
        .and(path("/api/new_order"))
        .and(header("authorization", format!("Bearer {jwt}")))
        .and(signature_matcher.clone())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0,
            "message": "ok",
            "request_id": "req-1"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/cancel_order"))
        .and(header("authorization", format!("Bearer {jwt}")))
        .and(signature_matcher.clone())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0,
            "message": "ok",
            "request_id": "req-2"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/change_leverage"))
        .and(header("authorization", format!("Bearer {jwt}")))
        .and(signature_matcher)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": 0,
            "message": "ok",
            "request_id": "req-3"
        })))
        .mount(&server)
        .await;

    let mut client = assert_ok!(StandxClient::with_config_and_base_urls(
        ClientConfig::default(),
        &base_url,
        &base_url
    ));
    client.set_credentials_and_signer(
        Credentials {
            jwt_token: jwt,
            wallet_address: "0x1234567890abcdef".to_string(),
            chain: Chain::Bsc,
        },
        signer,
    );

    let order_req = NewOrderRequest {
        symbol: "BTC-USD".to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        qty: Decimal::from(1),
        time_in_force: TimeInForce::Gtc,
        reduce_only: false,
        price: Some(Decimal::from(10)),
        cl_ord_id: None,
        margin_mode: None,
        leverage: None,
        tp_price: None,
        sl_price: None,
    };

    let new_order = assert_ok!(client.new_order(order_req).await);
    assert_eq!(new_order.code, 0);

    let cancel = assert_ok!(client.cancel_order(standx_point_adapter::CancelOrderRequest {
        order_id: Some(1),
        cl_ord_id: None,
    }).await);
    assert_eq!(cancel.code, 0);

    let change = assert_ok!(client.change_leverage("BTC-USD", 10).await);
    assert_eq!(change.code, 0);
}
