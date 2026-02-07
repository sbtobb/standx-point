/*
[INPUT]:  Message to sign and wallet private key
[OUTPUT]: Signature string for authentication
[POS]:    Auth layer - wallet integration abstraction
[UPDATE]: When adding new wallet types or changing signature format
*/

use async_trait::async_trait;

use crate::http::Result;
use crate::types::Chain;

/// Trait for wallet signing operations
///
/// Implement this trait for your wallet type (EVM, Solana, etc.)
/// The trait is async to support hardware wallets and external signers.
#[async_trait]
pub trait WalletSigner: Send + Sync {
    /// Get the blockchain chain type
    fn chain(&self) -> Chain;

    /// Get the wallet address
    fn address(&self) -> &str;

    /// Sign a message and return the signature
    ///
    /// For EVM: Returns hex-encoded signature (0x...)
    /// For Solana: Returns base64-encoded signature
    async fn sign_message(&self, message: &str) -> Result<String>;
}

/// Mock wallet signer for testing
#[derive(Debug, Clone)]
pub struct MockWalletSigner {
    chain: Chain,
    address: String,
    signature: String,
}

impl MockWalletSigner {
    /// Create a new mock signer with predetermined signature
    pub fn new(chain: Chain, address: &str, signature: &str) -> Self {
        Self {
            chain,
            address: address.to_string(),
            signature: signature.to_string(),
        }
    }
}

#[async_trait]
impl WalletSigner for MockWalletSigner {
    fn chain(&self) -> Chain {
        self.chain
    }

    fn address(&self) -> &str {
        &self.address
    }

    async fn sign_message(&self, _message: &str) -> Result<String> {
        Ok(self.signature.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_signer() {
        let signer = MockWalletSigner::new(
            Chain::Bsc,
            "0x1234567890abcdef",
            "0xmock_signature",
        );

        assert_eq!(signer.chain(), Chain::Bsc);
        assert_eq!(signer.address(), "0x1234567890abcdef");

        let signature = signer.sign_message("test message").await.unwrap();
        assert_eq!(signature, "0xmock_signature");
    }
}
