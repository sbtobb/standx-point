/*
[INPUT]:  EVM private key (hex string)
[OUTPUT]: Signed messages and wallet address for EVM chains
[POS]:    Auth layer - EVM wallet implementation
[UPDATE]: When signing logic or EVM address formatting changes
*/


use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use alloy::signers::Signer;
use async_trait::async_trait;

use crate::auth::WalletSigner;
use crate::http::Result;
use crate::types::Chain;

/// Signer for EVM-compatible wallets (e.g., BSC)
pub struct EvmWalletSigner {
    signer: PrivateKeySigner,
    address: String,
}

impl EvmWalletSigner {
    /// Create a new EVM wallet signer from a hex-encoded private key
    ///
    /// Supports both "0x"-prefixed and non-prefixed hex strings.
    pub fn new(private_key_hex: &str) -> Result<Self> {
        let private_key_hex = private_key_hex.strip_prefix("0x").unwrap_or(private_key_hex);
        let signer = PrivateKeySigner::from_str(private_key_hex)
            .map_err(|e| crate::http::StandxError::Config(format!("Invalid EVM private key: {}", e)))?;
        
        let address = signer.address().to_checksum(None);
        
        Ok(Self { signer, address })
    }
}

#[async_trait]
impl WalletSigner for EvmWalletSigner {
    fn chain(&self) -> Chain {
        Chain::Bsc
    }

    fn address(&self) -> &str {
        &self.address
    }

    async fn sign_message(&self, message: &str) -> Result<String> {
        let signature = self.signer.sign_message(message.as_bytes()).await
            .map_err(|e| crate::http::StandxError::Internal(format!("Failed to sign EVM message: {}", e)))?;
        
        // alloy's Signature as_bytes() returns [r, s, v]
        Ok(format!("0x{}", hex::encode(signature.as_bytes())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_evm_wallet_signer() {
        // A well-known test private key
        let pk = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let signer = EvmWalletSigner::new(pk).unwrap();
        
        assert_eq!(signer.chain(), Chain::Bsc);
        // address for above pk
        assert_eq!(signer.address(), "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        
        let message = "hello";
        let signature = signer.sign_message(message).await.unwrap();
        
        assert!(signature.starts_with("0x"));
        assert_eq!(signature.len(), 132); // 0x + 65 bytes * 2 = 132
    }

    #[test]
    fn test_evm_wallet_signer_no_prefix() {
        let pk = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let signer = EvmWalletSigner::new(pk).unwrap();
        assert_eq!(signer.address(), "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
    }
}
