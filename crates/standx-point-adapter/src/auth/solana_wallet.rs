/*
[INPUT]:  Solana private key (base58) and message to sign
[OUTPUT]: Base64-encoded JSON signature for Solana authentication
[POS]:    Auth layer - Solana wallet implementation
[UPDATE]: When Solana signature format or SDK version changes
*/

use async_trait::async_trait;
use base64::{prelude::BASE64_STANDARD, Engine};
use bs58;
use serde::Serialize;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::signer::keypair::keypair_from_seed;

use crate::http::{Result, StandxError};
use crate::types::Chain;
use crate::auth::wallet::WalletSigner;

/// Solana wallet signer implementation
pub struct SolanaWalletSigner {
    keypair: Keypair,
    address: String,
}

#[derive(Serialize)]
struct SolanaSignatureOutput {
    #[serde(rename = "signedMessage")]
    signed_message: Vec<u8>,
    signature: Vec<u8>,
    account: SolanaAccount,
}

#[derive(Serialize)]
struct SolanaAccount {
    #[serde(rename = "publicKey")]
    public_key: Vec<u8>,
}

#[derive(Serialize)]
struct SolanaSignaturePayload {
    input: String,
    output: SolanaSignatureOutput,
}

impl SolanaWalletSigner {
    /// Create a new Solana wallet signer from a base58-encoded private key
    /// Supports 64-byte keypair or 32-byte seed
    pub fn new(private_key_base58: &str) -> Result<Self> {
        let bytes = bs58::decode(private_key_base58)
            .into_vec()
            .map_err(|e| StandxError::Config(format!("Invalid base58 private key: {}", e)))?;

        let keypair = if bytes.len() == 64 {
            Keypair::try_from(bytes.as_slice())
                .map_err(|e| StandxError::Config(format!("Invalid keypair bytes: {}", e)))?
        } else if bytes.len() == 32 {
            keypair_from_seed(&bytes)
                .map_err(|e| StandxError::Config(format!("Invalid seed bytes: {}", e)))?
        } else {
            return Err(StandxError::Config(format!(
                "Invalid private key length: expected 32 or 64 bytes, got {}",
                bytes.len()
            )));
        };

        let address = keypair.pubkey().to_string();

        Ok(Self { keypair, address })
    }
}

#[async_trait]
impl WalletSigner for SolanaWalletSigner {
    fn chain(&self) -> Chain {
        Chain::Solana
    }

    fn address(&self) -> &str {
        &self.address
    }

    async fn sign_message(&self, message: &str) -> Result<String> {
        let message_bytes = message.as_bytes();
        let signature = self.keypair.sign_message(message_bytes);
        
        let payload = SolanaSignaturePayload {
            input: message.to_string(),
            output: SolanaSignatureOutput {
                signed_message: message_bytes.to_vec(),
                signature: signature.as_ref().to_vec(),
                account: SolanaAccount {
                    public_key: self.keypair.pubkey().to_bytes().to_vec(),
                },
            },
        };

        let json = serde_json::to_string(&payload)
            .map_err(StandxError::Serialization)?;

        Ok(BASE64_STANDARD.encode(json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_solana_signer_from_seed() {
        // A dummy 32-byte seed in base58 (all zeros)
        let seed = "11111111111111111111111111111111";
        let signer = SolanaWalletSigner::new(seed).unwrap();
        
        assert_eq!(signer.chain(), Chain::Solana);

        let message = "hello world";
        let signature_b64 = signer.sign_message(message).await.unwrap();
        
        let decoded_json = BASE64_STANDARD.decode(signature_b64).unwrap();
        let decoded_json_str = String::from_utf8(decoded_json).unwrap();
        
        let payload: serde_json::Value = serde_json::from_str(&decoded_json_str).unwrap();
        assert_eq!(payload["input"], "hello world");
        assert_eq!(payload["output"]["signedMessage"], serde_json::json!(message.as_bytes()));
        assert!(payload["output"]["signature"].is_array());
        assert_eq!(payload["output"]["account"]["publicKey"], serde_json::json!(signer.keypair.pubkey().to_bytes()));
    }

    #[tokio::test]
    async fn test_solana_signer_invalid_key() {
        let result = SolanaWalletSigner::new("invalid_base58_!@#");
        assert!(result.is_err());
        
        let result = SolanaWalletSigner::new("bs58tooShort");
        assert!(result.is_err());
    }
}
