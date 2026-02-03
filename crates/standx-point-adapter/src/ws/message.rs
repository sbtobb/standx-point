/*
[INPUT]:  Raw WebSocket message bytes
[OUTPUT]: Parsed WebSocketMessage structs
[POS]:    WebSocket layer - message parsing and validation
[UPDATE]: When adding new message types or changing format
*/

use serde::{Deserialize, Serialize};

/// Market price data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PriceData {
    pub base: String,
    pub index_price: String,
    pub last_price: String,
    pub mark_price: String,
    pub mid_price: String,
    pub quote: String,
    pub spread: Vec<String>,
    pub symbol: String,
    pub time: String,
}

/// Depth book data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DepthBookData {
    pub asks: Vec<Vec<String>>,
    pub bids: Vec<Vec<String>>,
    pub symbol: String,
}

/// Order update data
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrderUpdateData {
    pub id: i64,
    pub symbol: String,
    pub side: String,
    pub status: String,
    pub qty: String,
    pub fill_qty: String,
    pub price: String,
    pub order_type: String,
}
