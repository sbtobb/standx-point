/*
[INPUT]:  WebSocket URL and JWT token for authentication
[OUTPUT]: Real-time market data and order updates via channels
[POS]:    WebSocket layer - real-time data stream handling
[UPDATE]: When adding new channels or changing connection logic
*/

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;

const MARKET_STREAM_URL: &str = "wss://perps.standx.com/ws-stream/v1";
const ORDER_STREAM_URL: &str = "wss://perps.standx.com/ws-api/v1";

/// WebSocket message types
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "channel")]
pub enum WebSocketMessage {
    #[serde(rename = "price")]
    Price { symbol: String, data: serde_json::Value },
    #[serde(rename = "depth_book")]
    DepthBook { symbol: String, data: serde_json::Value },
    #[serde(rename = "order")]
    Order { data: serde_json::Value },
    #[serde(rename = "position")]
    Position { data: serde_json::Value },
    #[serde(rename = "balance")]
    Balance { data: serde_json::Value },
    #[serde(other)]
    Other,
}

/// WebSocket client for StandX API
#[derive(Debug)]
#[allow(dead_code)]
pub struct StandxWebSocket {
    message_tx: mpsc::Sender<WebSocketMessage>,
    message_rx: Option<mpsc::Receiver<WebSocketMessage>>,
    outbound_tx: Arc<Mutex<Option<mpsc::Sender<WsMessage>>>>,
}

#[allow(dead_code)]
impl StandxWebSocket {
    /// Create a new WebSocket client
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            message_tx: tx,
            message_rx: Some(rx),
            outbound_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the message receiver
    pub fn take_receiver(&mut self) -> Option<mpsc::Receiver<WebSocketMessage>> {
        self.message_rx.take()
    }

    /// Connect to market data stream (public)
    pub async fn connect_market_stream(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.connect_stream(MARKET_STREAM_URL).await
    }

    /// Connect to order response stream (authenticated)
    pub async fn connect_order_stream(
        &self,
        _token: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.connect_stream(ORDER_STREAM_URL).await
    }

    /// Subscribe to price updates for a symbol
    pub async fn subscribe_price(&self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        let msg = serde_json::json!({
            "subscribe": {
                "channel": "price",
                "symbol": symbol
            }
        });
        self.send_subscription(msg).await
    }

    /// Subscribe to depth book updates
    pub async fn subscribe_depth(&self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        let msg = serde_json::json!({
            "subscribe": {
                "channel": "depth_book",
                "symbol": symbol
            }
        });
        self.send_subscription(msg).await
    }

    /// Subscribe to order updates (requires auth)
    pub async fn subscribe_orders(&self) -> Result<(), Box<dyn std::error::Error>> {
        let msg = serde_json::json!({
            "subscribe": {
                "channel": "order"
            }
        });
        self.send_subscription(msg).await
    }
    
    /// Unsubscribe from price updates for a symbol
    pub async fn unsubscribe_price(&self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        let msg = serde_json::json!({
            "unsubscribe": {
                "channel": "price",
                "symbol": symbol
            }
        });
        self.send_subscription(msg).await
    }
    
    /// Unsubscribe from depth book updates
    pub async fn unsubscribe_depth(&self, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        let msg = serde_json::json!({
            "unsubscribe": {
                "channel": "depth_book",
                "symbol": symbol
            }
        });
        self.send_subscription(msg).await
    }
    
    /// Unsubscribe from order updates (requires auth)
    pub async fn unsubscribe_orders(&self) -> Result<(), Box<dyn std::error::Error>> {
        let msg = serde_json::json!({
            "unsubscribe": {
                "channel": "order"
            }
        });
        self.send_subscription(msg).await
    }

    async fn connect_stream(&self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_stream, _response) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        let (outbound_tx, mut outbound_rx) = mpsc::channel(100);
        let outbound_state = self.outbound_tx.clone();

        {
            let mut guard = outbound_state.lock().await;
            if guard.is_some() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "WebSocket already connected",
                )));
            }
            *guard = Some(outbound_tx);
        }

        let message_tx = self.message_tx.clone();
        let outbound_state_for_task = outbound_state.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    outbound = outbound_rx.recv() => {
                        match outbound {
                            Some(message) => {
                                if write.send(message).await.is_err() {
                                    break;
                                }
                            }
                            None => {
                                let _ = write.send(WsMessage::Close(None)).await;
                                break;
                            }
                        }
                    }
                    incoming = read.next() => {
                        match incoming {
                            Some(Ok(WsMessage::Close(_))) => {
                                let _ = write.send(WsMessage::Close(None)).await;
                                break;
                            }
                            Some(Ok(WsMessage::Ping(_))) | Some(Ok(WsMessage::Pong(_))) => {}
                            Some(Ok(message)) => {
                                if let Some(parsed) = Self::parse_message(message) {
                                    if message_tx.send(parsed).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            Some(Err(_)) | None => {
                                break;
                            }
                        }
                    }
                }
            }

            let mut guard = outbound_state_for_task.lock().await;
            *guard = None;
        });

        Ok(())
    }

    async fn send_subscription(
        &self,
        message: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sender = {
            let guard = self.outbound_tx.lock().await;
            guard.clone().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotConnected, "WebSocket not connected")
            })?
        };

        sender
            .send(WsMessage::Text(message.to_string().into()))
            .await
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "WebSocket send channel closed",
                )
            })?;

        Ok(())
    }

    fn parse_message(message: WsMessage) -> Option<WebSocketMessage> {
        let text: String = match message {
            WsMessage::Text(text) => text.to_string(),
            WsMessage::Binary(bytes) => String::from_utf8(bytes.to_vec()).ok()?,
            _ => return Some(WebSocketMessage::Other),
        };

        Some(
            serde_json::from_str::<WebSocketMessage>(&text).unwrap_or(WebSocketMessage::Other),
        )
    }
}

impl Default for StandxWebSocket {
    fn default() -> Self {
        Self::new()
    }
}
