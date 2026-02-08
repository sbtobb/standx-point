/*
[INPUT]:  Test configuration and mock server requirements
[OUTPUT]: Shared test utilities, fixtures, and mock helpers
[POS]:    Test infrastructure - shared across all test modules
[UPDATE]: When adding new test patterns or fixtures
*/

//! Common test utilities for standx-point-adapter tests

use wiremock::MockServer;

/// Setup a mock HTTP server for testing
pub async fn setup_mock_server() -> MockServer {
    MockServer::start().await
}

/// Generate a deterministic Ed25519 keypair for testing
#[allow(dead_code)]
pub fn generate_test_keypair() -> ([u8; 32], [u8; 32]) {
    let seed = [1u8; 32];
    (seed, seed)
}

/// Mock JWT token for testing
pub fn mock_jwt_token() -> String {
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test.signature".to_string()
}
