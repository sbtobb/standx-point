/*
[INPUT]:  None (Demonstration of persistent key storage)
[OUTPUT]: Console output of stored accounts
[POS]:    Examples - persistent authentication demonstration
[UPDATE]: When PersistentKeyManager or AuthManager API changes
*/

//! Example: Authentication with persistent key storage
//!
//! Demonstrates the simplified auth flow using PersistentKeyManager.

use standx_point_adapter::auth::AuthManager;
use standx_point_adapter::http::StandxClient;

// Test key - DO NOT USE IN PRODUCTION
#[allow(dead_code)]
const TEST_EVM_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Initialize client and AuthManager with a specific key directory
    // This directory will store Ed25519 session keys for each wallet address
    let client = StandxClient::new()?;
    let auth = AuthManager::new_with_key_dir(client, "./.standx-config/keys");

    println!("=== StandX Persistent Auth Example ===");

    // Step 2: List accounts that already have stored session keys
    let accounts = auth.list_stored_accounts();
    println!("Stored accounts: {:?}", accounts);

    if accounts.is_empty() {
        println!("No stored accounts found in ./.standx-config/keys");
    } else {
        for addr in &accounts {
            println!("  - Found session key for: {}", addr);
        }
    }

    // Step 3: Demonstrate how authentication would be performed
    // In real usage, you would call:
    /*
    let response = auth.authenticate_with_wallet(
        "0x...",  // wallet address
        TEST_EVM_KEY,
        standx_point_adapter::types::Chain::Bsc
    ).await?;
    */

    println!("\nExample complete - see comments for real usage");
    Ok(())
}
