/*
[INPUT]:  Wallet credentials and StandX API endpoints
[OUTPUT]: Authenticated JWT token for API access
[POS]:    Examples - authentication flow demonstration
[UPDATE]: When auth flow changes
*/

use standx_point_adapter::*;

/// Example: Authentication flow
/// 
/// This example demonstrates the complete authentication flow:
/// 1. Create HTTP client
/// 2. Create auth manager with Ed25519 signer
/// 3. Prepare signin (get signedData from server)
/// 4. Sign message with wallet
/// 5. Login to get JWT token
#[tokio::main]
async fn main() {
    println!("=== StandX Authentication Example ===\n");
    
    // Step 1: Create HTTP client
    let client = match StandxClient::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create client: {}", e);
            return;
        }
    };
    println!("✓ HTTP client created");
    
    // Step 2: Create auth manager
    // This uses PersistentKeyManager to manage Ed25519 signers for different addresses
    let auth_manager = AuthManager::new(client);
    println!("✓ Auth manager created");

    // Show request ID (Ed25519 public key) for a test address
    let test_address = "0x0000000000000000000000000000000000000000";
    if let Ok(signer) = auth_manager.key_manager().get_or_create_signer(test_address) {
        println!("  Example Request ID for {}: {}", test_address, signer.public_key_base58());
    }
    
    // Step 3-5: Authenticate with wallet
    // In a real implementation, you would:
    // 1. Implement WalletSigner for your wallet (MetaMask, Phantom, etc.)
    // 2. Call auth_manager.authenticate(&wallet, 604800).await
    
    println!("\nNote: In production, implement WalletSigner for your wallet:");
    println!("  - For EVM: Use ethers-rs or similar to sign messages");
    println!("  - For Solana: Use solana-sdk to sign messages");
    println!("  - Then call: auth_manager.authenticate(&wallet, 604800).await");
    
    println!("\n✓ Authentication example complete");
}
