/*
[INPUT]:  Request parameters and Ed25519 signer
[OUTPUT]: Signed request headers (x-request-signature)
[POS]:    HTTP layer - request signing for authenticated endpoints
[UPDATE]: When changing signing algorithm or header format
*/

use crate::auth::Ed25519Signer;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Default request signature version.
pub const DEFAULT_SIGNATURE_VERSION: &str = "v1";

/// Header carrying the signature version.
pub const HEADER_REQUEST_VERSION: &str = "x-request-version";

/// Header carrying the request id used in signing.
pub const HEADER_REQUEST_ID: &str = "x-request-id";

/// Header carrying the request timestamp used in signing.
pub const HEADER_REQUEST_TIMESTAMP: &str = "x-request-timestamp";

/// Header carrying the base64-encoded Ed25519 signature.
pub const HEADER_REQUEST_SIGNATURE: &str = "x-request-signature";

/// Signed metadata for body-signature endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodySignature {
    pub version: String,
    pub request_id: String,
    pub timestamp: u64,
    pub signature: String,
}

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

    /// Get current unix timestamp in milliseconds.
    pub fn timestamp_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
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

    /// Create a [`BodySignature`] for the given request `payload`.
    ///
    /// This uses [`DEFAULT_SIGNATURE_VERSION`], a UUID v4 request id and the provided `timestamp`.
    pub fn sign_payload(&self, payload: &str, timestamp: u64) -> BodySignature {
        let request_id = self.request_id();
        let version = DEFAULT_SIGNATURE_VERSION;
        let signature = self.sign_request(version, &request_id, timestamp, payload);
        BodySignature {
            version: version.to_string(),
            request_id,
            timestamp,
            signature,
        }
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

    #[test]
    fn test_sign_request_is_deterministic_for_fixed_key() {
        let secret = [1u8; 32];
        let signer = Ed25519Signer::from_secret_key(&secret);
        let request_signer = RequestSigner::new(signer);

        let version = "v1";
        let request_id = "rid";
        let timestamp = 123;
        let payload = r#"{"a":1}"#;

        let got = request_signer.sign_request(version, request_id, timestamp, payload);

        // Compute expected signature from the spec message format.
        let message = format!("{version},{request_id},{timestamp},{payload}");
        let expected = {
            let signer = Ed25519Signer::from_secret_key(&secret);
            let sig = signer.sign(message.as_bytes());
            BASE64.encode(sig.to_bytes())
        };

        assert_eq!(got, expected);
    }
}
