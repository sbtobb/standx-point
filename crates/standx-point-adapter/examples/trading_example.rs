/*
[INPUT]:  Authenticated client and order parameters
[OUTPUT]: Order creation/cancellation confirmations
[POS]:    Examples - trading operations
[UPDATE]: When trading API changes
*/

use rust_decimal::Decimal;
use standx_point_adapter::*;
use std::str::FromStr;

/// Example: Trading operations (requires authentication + body signature)
///
/// Trading endpoints require:
/// 1. JWT authentication (Authorization header)
/// 2. Ed25519 body signature (x-request-signature header)
#[tokio::main]
async fn main() {
    println!("=== StandX Trading Example ===\n");

    // Create client
    let _client = match StandxClient::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create client: {}", e);
            return;
        }
    };
    println!("✓ HTTP client created");

    // In production:
    // 1. Authenticate to get JWT token
    // 2. Set credentials on client
    // 3. Use RequestSigner to sign request bodies

    println!("\nTrading requires:");
    println!("  1. JWT token (from authentication)");
    println!("  2. Ed25519 body signature for each request");

    // Example: Create a new order
    println!("\nExample order request:");
    let order_req = NewOrderRequest {
        symbol: "BTC-USD".to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        qty: Decimal::from_str("0.1").unwrap_or_default(),
        price: Some(Decimal::from_str("50000").unwrap_or_default()),
        time_in_force: TimeInForce::Gtc,
        reduce_only: false,
        cl_ord_id: None,
        margin_mode: None,
        leverage: None,
        tp_price: None,
        sl_price: None,
    };
    println!("  {:?}", order_req);

    // In production:
    // let response = client.new_order(order_req).await?;

    println!("\n✓ Trading example complete");
    println!("  Note: Full implementation requires HTTP client completion");
}
