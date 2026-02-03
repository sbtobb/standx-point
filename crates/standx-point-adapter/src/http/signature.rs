/*
[INPUT]:  Request parameters and Ed25519 signer
[OUTPUT]: Signed request headers (x-request-signature)
[POS]:    HTTP layer - request signing for authenticated endpoints
[UPDATE]: When changing signing algorithm or header format
*/

use crate::auth::Ed25519Signer;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use uuid::Uuid;

/// Signs HTTP request bodies for authenticated endpoints
#[derive(Debug)]
pub struct RequestSigner {
    signer: Ed25519Signer,
}

impl RequestSigner {
    /// Create a new request signer with the given Ed25519 signer
    pub fn new(signer: Ed25519Signer) -> Self {
        Self { signer }
    }

    /// Generate a request id for signing headers
    pub fn request_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// Sign a request according to StandX API specification
    ///
    /// Format: "{version},{request_id},{timestamp},{payload}"
    /// Returns base64-encoded signature
    pub fn sign_request(
        &self,
        version: &str,
        request_id: &str,
        timestamp: u64,
        payload: &str,
    ) -> String {
        let message = format!("{version},{request_id},{timestamp},{payload}");
        let signature = self.signer.sign(message.as_bytes());
        BASE64.encode(signature.to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_is_uuid() {
        let signer = Ed25519Signer::generate();
        let request_signer = RequestSigner::new(signer);

        let request_id = request_signer.request_id();
        assert!(Uuid::parse_str(&request_id).is_ok());
    }

    #[test]
    fn test_sign_request() {
        let signer = Ed25519Signer::generate();
        let request_signer = RequestSigner::new(signer);

        let signature = request_signer.sign_request(
            "v1",
            "test-request-id",
            1_234_567_890,
            r#"{"symbol":"BTC-USD"}"#,
        );

        assert!(!signature.is_empty());
        let decoded = BASE64.decode(&signature).unwrap();
        assert_eq!(decoded.len(), 64);
    }
}
