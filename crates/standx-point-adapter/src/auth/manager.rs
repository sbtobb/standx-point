/*
[INPUT]:  Wallet signer and HTTP client
[OUTPUT]: Authenticated credentials (JWT token)
[POS]:    Auth layer - orchestrates complete authentication flow
[UPDATE]: When auth endpoints or flow steps change
*/

use std::fs;
use std::path::{Path, PathBuf};

use base64::{
    Engine as _,
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};
use reqwest::Method;
use serde::Deserialize;

use crate::http::{Result, StandxClient, StandxError};
use crate::types::Chain;

use super::{EvmWalletSigner, JwtManager, PersistentKeyManager, SolanaWalletSigner, WalletSigner};

const DEFAULT_EXPIRES_SECONDS: u64 = 7 * 24 * 60 * 60;

/// Data returned from prepare-signin endpoint
#[derive(Debug, Deserialize)]
pub struct SigninData {
    #[serde(rename = "signedData")]
    pub signed_data: String,
}

/// Response from login endpoint
#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub address: String,
    #[serde(rename = "alias")]
    pub alias: Option<String>,
    pub chain: String,
    #[serde(rename = "perpsAlpha")]
    pub perps_alpha: Option<bool>,
}

/// Manages the complete authentication flow
#[derive(Debug)]
pub struct AuthManager {
    client: StandxClient,
    key_manager: PersistentKeyManager,
    jwt_manager: JwtManager,
}

impl AuthManager {
    /// Create a new auth manager using the default key directory.
    ///
    /// Default: `./.standx-config/keys` relative to current working directory.
    pub fn new(client: StandxClient) -> Self {
        Self::new_with_key_dir(client, default_key_dir())
    }

    /// Create a new auth manager with an explicit key directory.
    pub fn new_with_key_dir(client: StandxClient, key_dir: impl AsRef<Path>) -> Self {
        Self {
            client,
            key_manager: PersistentKeyManager::new(key_dir),
            jwt_manager: JwtManager::new(),
        }
    }

    /// Get the JWT manager
    pub fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }

    /// Get the key manager used for per-wallet Ed25519 persistence.
    pub fn key_manager(&self) -> &PersistentKeyManager {
        &self.key_manager
    }

    /// List all wallet addresses that have stored Ed25519 keys.
    pub fn list_stored_accounts(&self) -> Vec<String> {
        self.key_manager.list_stored_accounts()
    }

    /// Step 1: Prepare signin - get signature data from server
    ///
    /// POST /v1/offchain/prepare-signin?chain={chain}
    pub async fn prepare_signin(&self, chain: Chain, address: &str) -> Result<SigninData> {
        let signer = self
            .key_manager
            .get_or_create_signer(address)
            .map_err(|e| {
                StandxError::Config(format!(
                    "Failed to load or create ed25519 signer for {address}: {e}"
                ))
            })?;
        let request_id = signer.public_key_base58();

        let body = serde_json::json!({
            "address": address,
            "requestId": request_id,
        });

        let endpoint = format!(
            "/v1/offchain/prepare-signin?chain={}",
            chain_query_value(chain)
        );

        let builder = self.client.auth_request(Method::POST, &endpoint)?;
        let builder = builder.json(&body);
        self.client.send_json(builder).await
    }

    /// Step 2: Login - submit signature to get JWT
    ///
    /// POST /v1/offchain/login?chain={chain}
    pub async fn login(
        &self,
        chain: Chain,
        signature: &str,
        signed_data: &str,
        expires_seconds: u64,
    ) -> Result<LoginResponse> {
        let body = serde_json::json!({
            "signature": signature,
            "signedData": signed_data,
            "expiresSeconds": expires_seconds,
        });

        let endpoint = format!("/v1/offchain/login?chain={}", chain_query_value(chain));

        let builder = self.client.auth_request(Method::POST, &endpoint)?;
        let builder = builder.json(&body);
        self.client.send_json(builder).await
    }

    /// Authenticate using wallet private key + chain, while verifying the address.
    pub async fn authenticate_with_wallet(
        &self,
        wallet_address: &str,
        private_key: &str,
        chain: Chain,
    ) -> Result<LoginResponse> {
        match chain {
            Chain::Bsc => {
                let wallet = EvmWalletSigner::new(private_key)?;
                verify_wallet_address(chain, wallet_address, wallet.address())?;
                self.authenticate(&wallet, DEFAULT_EXPIRES_SECONDS).await
            }
            Chain::Solana => {
                let wallet = SolanaWalletSigner::new(private_key)?;
                verify_wallet_address(chain, wallet_address, wallet.address())?;
                self.authenticate(&wallet, DEFAULT_EXPIRES_SECONDS).await
            }
        }
    }

    /// Complete authentication flow
    ///
    /// 1. Prepare signin
    /// 2. Parse signedData to get message
    /// 3. Sign message with wallet
    /// 4. Login to get JWT
    /// 5. Store JWT in manager
    pub async fn authenticate(
        &self,
        wallet: &dyn WalletSigner,
        expires_seconds: u64,
    ) -> Result<LoginResponse> {
        let chain = wallet.chain();
        let address = wallet.address().to_string();

        // Step 1: Prepare signin
        let signin_data = self.prepare_signin(chain, &address).await?;

        // Step 2: Parse signedData JWT to extract message
        let message = extract_message_from_signed_data(&signin_data.signed_data)?;

        // Step 3: Sign message with wallet
        let signature = wallet.sign_message(&message).await?;

        // Step 4: Login
        let login_response = self
            .login(chain, &signature, &signin_data.signed_data, expires_seconds)
            .await?;

        // Step 5: Store JWT
        self.jwt_manager.set_token(
            login_response.token.clone(),
            expires_seconds,
            login_response.address.clone(),
            chain,
        );

        Ok(login_response)
    }
}

fn default_key_dir() -> PathBuf {
    let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config_dir = base_dir.join(".standx-config");
    let new_dir = config_dir.join("keys");
    migrate_legacy_key_dir(&base_dir, &new_dir);
    new_dir
}

fn migrate_legacy_key_dir(base_dir: &Path, new_dir: &Path) {
    if new_dir.exists() {
        return;
    }

    let legacy_dir = base_dir.join(".standx-keys");
    if !legacy_dir.exists() {
        return;
    }

    if fs::create_dir_all(new_dir).is_err() {
        return;
    }

    if fs::rename(&legacy_dir, new_dir).is_ok() {
        return;
    }

    if let Ok(entries) = fs::read_dir(&legacy_dir) {
        for entry in entries.flatten() {
            let from_path = entry.path();
            if let Some(name) = from_path.file_name() {
                let to_path = new_dir.join(name);
                let _ = fs::copy(&from_path, &to_path);
            }
        }
    }
}

fn chain_query_value(chain: Chain) -> &'static str {
    match chain {
        Chain::Bsc => "bsc",
        Chain::Solana => "solana",
    }
}

fn normalize_evm_address(address: &str) -> String {
    let address = address.trim();
    address
        .strip_prefix("0x")
        .or_else(|| address.strip_prefix("0X"))
        .unwrap_or(address)
        .to_ascii_lowercase()
}

fn verify_wallet_address(chain: Chain, expected: &str, derived: &str) -> Result<()> {
    let matches = match chain {
        Chain::Bsc => normalize_evm_address(expected) == normalize_evm_address(derived),
        Chain::Solana => expected.trim() == derived.trim(),
    };

    if matches {
        Ok(())
    } else {
        Err(StandxError::Config(format!(
            "Wallet address mismatch: provided {expected}, derived {derived}"
        )))
    }
}

fn extract_message_from_signed_data(signed_data: &str) -> Result<String> {
    let signed_data = signed_data.trim();
    let payload_b64 = signed_data
        .split('.')
        .nth(1)
        .ok_or_else(|| StandxError::InvalidResponse("signedData is not a valid JWT".to_string()))?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| URL_SAFE.decode(payload_b64))
        .map_err(|e| {
            StandxError::InvalidResponse(format!("Invalid signedData JWT payload base64: {e}"))
        })?;

    let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)?;
    let message = payload
        .get("message")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            StandxError::InvalidResponse("signedData JWT missing 'message' claim".to_string())
        })?;

    Ok(message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use uuid::Uuid;
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn temp_dir() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("standx-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn make_test_jwt(message: &str) -> String {
        let header = serde_json::json!({"alg": "none", "typ": "JWT"});
        let payload = serde_json::json!({"message": message});

        let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
        let payload_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap());

        format!("{header_b64}.{payload_b64}.signature")
    }

    #[test]
    fn test_auth_manager_extract_message_from_signed_data() {
        let jwt = make_test_jwt("hello");
        let message = extract_message_from_signed_data(&jwt).unwrap();
        assert_eq!(message, "hello");
    }

    #[tokio::test]
    async fn test_auth_manager_address_mismatch_bsc() {
        let client = StandxClient::new().unwrap();
        let dir = temp_dir();
        let auth_manager = AuthManager::new_with_key_dir(client, &dir);

        let pk = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let bad_address = "0x0000000000000000000000000000000000000000";

        let err = auth_manager
            .authenticate_with_wallet(bad_address, pk, Chain::Bsc)
            .await
            .unwrap_err();

        match err {
            StandxError::Config(msg) => {
                assert!(msg.to_ascii_lowercase().contains("address mismatch"));
            }
            other => panic!("unexpected error: {other:?}"),
        }

        fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn test_auth_manager_authenticate_with_wallet_happy_path_bsc() {
        let server = MockServer::start().await;

        let client = StandxClient::with_config_and_base_urls(
            crate::http::ClientConfig::default(),
            &server.uri(),
            &server.uri(),
        )
        .unwrap();
        let dir = temp_dir();
        let auth_manager = AuthManager::new_with_key_dir(client, &dir);

        let pk = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let wallet = EvmWalletSigner::new(pk).unwrap();
        let derived_address = wallet.address().to_string();

        // Accept both checksummed and lowercase input; the request will still use derived address.
        let provided_address = derived_address.to_ascii_lowercase();

        // Ensure requestId is deterministic within this test.
        let request_id = auth_manager
            .key_manager()
            .get_or_create_signer(&derived_address)
            .unwrap()
            .public_key_base58();

        let message = "hello";
        let signed_data = make_test_jwt(message);
        let expected_signature = wallet.sign_message(message).await.unwrap();

        Mock::given(method("POST"))
            .and(path("/v1/offchain/prepare-signin"))
            .and(query_param("chain", "bsc"))
            .and(body_json(serde_json::json!({
                "address": derived_address,
                "requestId": request_id,
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "signedData": signed_data.clone(),
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/offchain/login"))
            .and(query_param("chain", "bsc"))
            .and(body_json(serde_json::json!({
                "signature": expected_signature,
                "signedData": signed_data,
                "expiresSeconds": DEFAULT_EXPIRES_SECONDS,
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token": "jwt-token",
                "address": wallet.address(),
                "chain": "bsc",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let login = auth_manager
            .authenticate_with_wallet(&provided_address, pk, Chain::Bsc)
            .await
            .unwrap();

        assert_eq!(login.token, "jwt-token");
        assert_eq!(
            auth_manager.jwt_manager().get_token(),
            Some("jwt-token".to_string())
        );
        assert!(
            auth_manager
                .list_stored_accounts()
                .contains(&wallet.address().to_string())
        );

        fs::remove_dir_all(dir).unwrap();
    }
}
