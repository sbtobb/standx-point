/*
[INPUT]:  Symbol identifier (e.g., "BTC-USD")
[OUTPUT]: Market data (price, depth, symbol info)
[POS]:    Examples - public market data queries
[UPDATE]: When adding new market data endpoints
*/

use standx_point_adapter::*;

/// Example: Query market data (no authentication required)
///
/// These endpoints are public and don't require JWT authentication.
#[tokio::main]
async fn main() {
    println!("=== StandX Market Data Example ===\n");

    // Create client (no auth needed for public endpoints)
    let client = match StandxClient::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create client: {}", e);
            return;
        }
    };
    println!("✓ HTTP client created (no auth required for public endpoints)\n");

    let symbol = "BTC-USD";

    // Query symbol information
    println!("Querying symbol info for {}...", symbol);
    match client.query_symbol_info(symbol).await {
        Ok(info) => println!("✓ Symbol info: {:?}", info),
        Err(e) => println!("✗ Error: {} (HTTP not yet fully implemented)", e),
    }

    // Query current price
    println!("\nQuerying price for {}...", symbol);
    match client.query_symbol_price(symbol).await {
        Ok(price) => println!("✓ Price: {:?}", price),
        Err(e) => println!("✗ Error: {} (HTTP not yet fully implemented)", e),
    }

    // Query order book depth
    println!("\nQuerying depth book for {}...", symbol);
    match client.query_depth_book(symbol).await {
        Ok(depth) => println!("✓ Depth: {:?}", depth),
        Err(e) => println!("✗ Error: {} (HTTP not yet fully implemented)", e),
    }

    println!("\n✓ Market data example complete");
}
