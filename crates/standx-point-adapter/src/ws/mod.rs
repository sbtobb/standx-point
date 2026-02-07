/*
[INPUT]:  WebSocket configuration and subscription channels
[OUTPUT]: Real-time market data and order updates
[POS]:    WebSocket layer - real-time data streams
[UPDATE]: When adding new channels or changing connection logic
*/

pub mod client;
pub mod message;

pub use client::{StandxWebSocket, WebSocketMessage};
pub use message::{DepthBookData, OrderUpdateData, PriceData};
