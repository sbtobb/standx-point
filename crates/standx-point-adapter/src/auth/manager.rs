/*
[INPUT]:  Wallet signer and HTTP client
[OUTPUT]: Authenticated credentials (JWT token)
[POS]:    Auth layer - orchestrates complete authentication flow
[UPDATE]: When auth endpoints or flow steps change
*/

use serde::Deserialize;

use crate::http::{Result, StandxClient};
use crate::types::Chain;

use super::{Ed25519Signer, JwtManager, WalletSigner};

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
    signer: Ed25519Signer,
    jwt_manager: JwtManager,
}

impl AuthManager {
    /// Create a new auth manager with generated Ed25519 keypair
    pub fn new(client: StandxClient) -> Self {
        Self {
            client,
            signer: Ed25519Signer::generate(),
            jwt_manager: JwtManager::new(),
        }
    }

    /// Get the Ed25519 signer (for request signing)
    pub fn signer(&self) -> &Ed25519Signer {
        &self.signer
    }

    /// Get the JWT manager
    pub fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }

    /// Step 1: Prepare signin - get signature data from server
    ///
    /// POST /v1/offchain/prepare-signin?chain={chain}
    pub async fn prepare_signin(&self, chain: Chain, address: &str) -> Result<SigninData> {
        let request_id = self.signer.public_key_base58();

        let body = serde_json::json!({
            "address": address,
            "requestId": request_id,
        });

        let chain_str = match chain {
            Chain::Bsc => "bsc",
            Chain::Solana => "solana",
        };

        let endpoint = format!("/v1/offchain/prepare-signin?chain={}", chain_str);
        let _ = (&self.client, body, endpoint);

        // For now, return a placeholder - actual HTTP call will be implemented later
        // This is the structure, implementation will use client.post()
        todo!("Implement HTTP call to prepare-signin endpoint")
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

        let chain_str = match chain {
            Chain::Bsc => "bsc",
            Chain::Solana => "solana",
        };

        let endpoint = format!("/v1/offchain/login?chain={}", chain_str);
        let _ = (&self.client, body, endpoint);

        // For now, return a placeholder - actual HTTP call will be implemented later
        todo!("Implement HTTP call to login endpoint")
    }

    /// Complete authentication flow
    ///
    /// 1. Prepare signin
    /// 2. Parse signedData to get message
    /// 3. Sign message with wallet
    /// 4. Login to get JWT
    /// 5. Store JWT in manager
    pub async fn authenticate(
        &mut self,
        wallet: &dyn WalletSigner,
        expires_seconds: u64,
    ) -> Result<LoginResponse> {
        let chain = wallet.chain();
        let address = wallet.address().to_string();

        // Step 1: Prepare signin
        let signin_data = self.prepare_signin(chain, &address).await?;

        // Step 2: Parse signedData JWT to extract message
        // For now, we'll use the signed_data as the message
        // In reality, this needs JWT parsing to extract the "message" field
        let message = &signin_data.signed_data;

        // Step 3: Sign message with wallet
        let signature = wallet.sign_message(message).await?;

        // Step 4: Login
        let login_response =
            self.login(chain, &signature, &signin_data.signed_data, expires_seconds)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::MockWalletSigner;

    // Note: These tests will fail with "not yet implemented" until HTTP client is fully implemented
    // They serve as placeholders for the test structure
    #[tokio::test]
    #[ignore = "Requires HTTP client implementation"]
    async fn test_authenticate_flow() {
        let client = StandxClient::new().unwrap();
        let _auth_manager = AuthManager::new(client);

        let _wallet = MockWalletSigner::new(
            Chain::Bsc,
            "0x1234567890abcdef",
            "0xmock_signature",
        );

        // This will panic with "not yet implemented" until HTTP is ready
        // let result = auth_manager.authenticate(&wallet, 604800).await;
        // assert!(result.is_ok());
    }
}
