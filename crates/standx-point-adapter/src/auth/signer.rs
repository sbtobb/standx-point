/*
[INPUT]:  Message bytes and optional secret key bytes
[OUTPUT]: Ed25519 signatures and base58-encoded public keys
[POS]:    Auth layer - cryptographic signing for request authentication
[UPDATE]: When changing signing algorithm or key format
*/

use bs58;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use rand::rngs::OsRng;

/// Ed25519 signer for request authentication
#[derive(Debug)]
pub struct Ed25519Signer {
    signing_key: SigningKey,
}

impl Ed25519Signer {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Create signer from existing secret key bytes (32 bytes)
    pub fn from_secret_key(bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        Self { signing_key }
    }

    /// Sign a message and return the signature
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    /// Get the public key in base58 encoding (for requestId)
    pub fn public_key_base58(&self) -> String {
        let verifying_key = self.signing_key.verifying_key();
        bs58::encode(verifying_key.as_bytes()).into_string()
    }

    /// Get the raw public key bytes
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Get the raw secret key bytes
    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Verify a signature against a message
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.signing_key
            .verifying_key()
            .verify(message, signature)
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let signer = Ed25519Signer::generate();
        assert_eq!(signer.public_key_bytes().len(), 32);
    }

    #[test]
    fn test_sign_and_verify() {
        let signer = Ed25519Signer::generate();
        let message = b"test message";
        let signature = signer.sign(message);
        assert!(signer.verify(message, &signature));
    }

    #[test]
    fn test_base58_encoding() {
        let signer = Ed25519Signer::generate();
        let base58_key = signer.public_key_base58();
        assert!(!base58_key.is_empty());
        let decoded = bs58::decode(&base58_key).into_vec().unwrap();
        assert_eq!(decoded.len(), 32);
    }
}
