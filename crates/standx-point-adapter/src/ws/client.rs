/*
[INPUT]:  WebSocket URL and JWT token for authentication
[OUTPUT]: Real-time market data and order updates via channels
[POS]:    WebSocket layer - real-time data stream handling
[UPDATE]: When adding new channels or changing connection logic
[UPDATE]: 2026-02-07 Add auth header for order stream and position subscriptions
*/

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tracing::{debug, info};
use uuid::Uuid;

const MARKET_STREAM_URL: &str = "wss://perps.standx.com/ws-stream/v1";
const ORDER_STREAM_URL: &str = "wss://perps.standx.com/ws-api/v1";
const MESSAGE_SAMPLE_LIMIT: usize = 3;
const SUBSCRIPTION_LOG_LIMIT: usize = 10;
const AUTH_LOG_LIMIT: usize = 5;
const OTHER_LOG_LIMIT: usize = 3;
const PARSE_FAIL_LOG_LIMIT: usize = 3;
const ERROR_RESPONSE_LOG_LIMIT: usize = 3;
const RAW_LOG_MAX_BYTES: usize = 1024;

static MESSAGE_SAMPLE_COUNT: AtomicUsize = AtomicUsize::new(0);
static SUBSCRIBE_LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static AUTH_LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static OTHER_LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static PARSE_FAIL_LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static ERROR_RESPONSE_LOG_COUNT: AtomicUsize = AtomicUsize::new(0);

/// WebSocket message types
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "channel")]
pub enum WebSocketMessage {
    #[serde(rename = "price")]
    Price {
        symbol: String,
        data: serde_json::Value,
    },
    #[serde(rename = "depth_book")]
    DepthBook {
        symbol: String,
        data: serde_json::Value,
    },
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
    stream_kind: Arc<Mutex<Option<&'static str>>>,
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
            stream_kind: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the message receiver
    pub fn take_receiver(&mut self) -> Option<mpsc::Receiver<WebSocketMessage>> {
        self.message_rx.take()
    }

    /// Connect to market data stream (public)
    pub async fn connect_market_stream(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.set_stream_kind("market").await;
        self.connect_stream(MARKET_STREAM_URL).await
    }

    /// Connect to order response stream (authenticated)
    pub async fn connect_order_stream(
        &self,
        token: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_stream_kind("order").await;
        let mut request = ORDER_STREAM_URL.to_string().into_client_request()?;
        let value = HeaderValue::from_str(&format!("Bearer {token}"))?;
        request.headers_mut().insert(AUTHORIZATION, value);
        self.connect_stream_with_request(request).await
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

    /// Authenticate for user-level channels on market stream
    pub async fn authenticate(
        &self,
        token: &str,
        streams: Option<&[&str]>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut auth = serde_json::json!({
            "token": token
        });

        if let Some(channels) = streams
            && let Some(auth_map) = auth.as_object_mut()
        {
            let stream_values: Vec<Value> = channels
                .iter()
                .map(|channel| serde_json::json!({ "channel": channel }))
                .collect();
            auth_map.insert("streams".to_string(), Value::Array(stream_values));
        }

        let msg = Value::Object({
            let mut root = serde_json::Map::new();
            root.insert("auth".to_string(), auth);
            root
        });

        self.send_message(msg).await
    }

    /// Subscribe to position updates (requires auth)
    pub async fn subscribe_positions(&self) -> Result<(), Box<dyn std::error::Error>> {
        let msg = serde_json::json!({
            "subscribe": {
                "channel": "position"
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

    /// Unsubscribe from position updates (requires auth)
    pub async fn unsubscribe_positions(&self) -> Result<(), Box<dyn std::error::Error>> {
        let msg = serde_json::json!({
            "unsubscribe": {
                "channel": "position"
            }
        });
        self.send_subscription(msg).await
    }

    async fn connect_stream(&self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_stream, _response) = connect_async(url).await?;
        self.connect_stream_with_socket(ws_stream).await
    }

    async fn connect_stream_with_request(
        &self,
        request: tokio_tungstenite::tungstenite::http::Request<()>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_stream, _response) = connect_async(request).await?;
        self.connect_stream_with_socket(ws_stream).await
    }

    async fn connect_stream_with_socket(
        &self,
        ws_stream: tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
                                if let Some(parsed) = Self::parse_message(message)
                                    && message_tx.send(parsed).await.is_err()
                                {
                                    break;
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
        self.send_message(message).await.map(|_| ())
    }

    async fn set_stream_kind(&self, kind: &'static str) {
        let mut guard = self.stream_kind.lock().await;
        *guard = Some(kind);
    }

    async fn send_message(
        &self,
        message: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sender = {
            let guard = self.outbound_tx.lock().await;
            guard.clone().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotConnected, "WebSocket not connected")
            })?
        };

        let stream_kind = *self.stream_kind.lock().await;

        let message = ensure_request_id(message, stream_kind);

        sender
            .send(WsMessage::Text(message.to_string().into()))
            .await
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "WebSocket send channel closed",
                )
            })?;

        log_outbound_message(&message, stream_kind);

        Ok(())
    }

    fn parse_message(message: WsMessage) -> Option<WebSocketMessage> {
        let text: String = match message {
            WsMessage::Text(text) => text.to_string(),
            WsMessage::Binary(bytes) => String::from_utf8(bytes.to_vec()).ok()?,
            _ => return Some(WebSocketMessage::Other),
        };

        match serde_json::from_str::<WebSocketMessage>(&text) {
            Ok(parsed) => {
                if matches!(parsed, WebSocketMessage::Other) {
                    log_other_message_once(&text);
                } else {
                    log_message_sample_once(&parsed);
                }
                Some(parsed)
            }
            Err(err) => {
                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                    if let Some(parsed) = infer_position_message(&value) {
                        log_message_sample_once(&parsed);
                        return Some(parsed);
                    }
                    if looks_like_error_response(&value) {
                        log_error_response_once(&value);
                        return Some(WebSocketMessage::Other);
                    }
                }

                log_parse_fail_once(&err, &text);
                Some(WebSocketMessage::Other)
            }
        }
    }
}

impl Default for StandxWebSocket {
    fn default() -> Self {
        Self::new()
    }
}

fn log_subscription_sent_with_stream(message: &serde_json::Value, stream: Option<&str>) {
    let count = SUBSCRIBE_LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    if count >= SUBSCRIPTION_LOG_LIMIT {
        return;
    }

    if let Some((action, channel, symbol)) = describe_subscription(message) {
        if let Some(symbol) = symbol {
            info!(
                sample_index = count + 1,
                sample_limit = SUBSCRIPTION_LOG_LIMIT,
                stream,
                action,
                channel,
                symbol,
                "ws subscription sent"
            );
        } else {
            info!(
                sample_index = count + 1,
                sample_limit = SUBSCRIPTION_LOG_LIMIT,
                stream,
                action,
                channel,
                "ws subscription sent"
            );
        }
        return;
    }

    let preview = truncate_for_log(&message.to_string(), RAW_LOG_MAX_BYTES);
    info!(
        sample_index = count + 1,
        sample_limit = SUBSCRIPTION_LOG_LIMIT,
        stream,
        message = %preview,
        "ws subscription sent"
    );
}

fn log_auth_sent(message: &serde_json::Value, stream: Option<&str>) {
    let count = AUTH_LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    if count >= AUTH_LOG_LIMIT {
        return;
    }

    let channels = describe_auth_streams(message);
    if channels.is_empty() {
        info!(
            sample_index = count + 1,
            sample_limit = AUTH_LOG_LIMIT,
            stream,
            action = "auth",
            "ws auth sent"
        );
        return;
    }

    let channel_list = channels.join(",");
    info!(
        sample_index = count + 1,
        sample_limit = AUTH_LOG_LIMIT,
        stream,
        action = "auth",
        channels = %channel_list,
        "ws auth sent"
    );
}

fn log_outbound_message(message: &serde_json::Value, stream: Option<&str>) {
    if message.get("auth").is_some() {
        log_auth_sent(message, stream);
        return;
    }

    log_subscription_sent_with_stream(message, stream);
}

fn describe_subscription(
    message: &serde_json::Value,
) -> Option<(&'static str, &str, Option<&str>)> {
    let (action, payload) = if let Some(payload) = message.get("subscribe") {
        ("subscribe", payload)
    } else if let Some(payload) = message.get("unsubscribe") {
        ("unsubscribe", payload)
    } else {
        return None;
    };

    let channel = payload.get("channel")?.as_str()?;
    let symbol = payload.get("symbol").and_then(|value| value.as_str());
    Some((action, channel, symbol))
}

fn describe_auth_streams(message: &serde_json::Value) -> Vec<String> {
    let Some(auth) = message.get("auth") else {
        return Vec::new();
    };
    let Some(streams) = auth.get("streams") else {
        return Vec::new();
    };
    let Some(array) = streams.as_array() else {
        return Vec::new();
    };

    array
        .iter()
        .filter_map(|entry| entry.get("channel").and_then(|value| value.as_str()))
        .map(str::to_string)
        .collect()
}

fn ensure_request_id(message: serde_json::Value, stream_kind: Option<&str>) -> serde_json::Value {
    let mut value = message;
    let Some(object) = value.as_object_mut() else {
        return value;
    };

    if stream_kind != Some("order") {
        return value;
    }

    if !object.contains_key("request_id") {
        object.insert(
            "request_id".to_string(),
            serde_json::Value::String(Uuid::new_v4().to_string()),
        );
    }

    value
}

fn log_message_sample_once(message: &WebSocketMessage) {
    let count = MESSAGE_SAMPLE_COUNT.fetch_add(1, Ordering::Relaxed);
    if count >= MESSAGE_SAMPLE_LIMIT {
        return;
    }

    match message {
        WebSocketMessage::Price { symbol, .. } => {
            info!(
                sample_index = count + 1,
                sample_limit = MESSAGE_SAMPLE_LIMIT,
                channel = "price",
                symbol,
                "ws message sample"
            );
        }
        WebSocketMessage::DepthBook { symbol, .. } => {
            info!(
                sample_index = count + 1,
                sample_limit = MESSAGE_SAMPLE_LIMIT,
                channel = "depth_book",
                symbol,
                "ws message sample"
            );
        }
        WebSocketMessage::Order { .. } => {
            info!(
                sample_index = count + 1,
                sample_limit = MESSAGE_SAMPLE_LIMIT,
                channel = "order",
                "ws message sample"
            );
        }
        WebSocketMessage::Position { .. } => {
            info!(
                sample_index = count + 1,
                sample_limit = MESSAGE_SAMPLE_LIMIT,
                channel = "position",
                "ws message sample"
            );
        }
        WebSocketMessage::Balance { .. } => {
            info!(
                sample_index = count + 1,
                sample_limit = MESSAGE_SAMPLE_LIMIT,
                channel = "balance",
                "ws message sample"
            );
        }
        WebSocketMessage::Other => {
            info!(
                sample_index = count + 1,
                sample_limit = MESSAGE_SAMPLE_LIMIT,
                channel = "other",
                "ws message sample"
            );
        }
    }
}

fn log_other_message_once(raw: &str) {
    let count = OTHER_LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    if count < OTHER_LOG_LIMIT {
        info!(
            sample_index = count + 1,
            sample_limit = OTHER_LOG_LIMIT,
            bytes = raw.len(),
            "ws message channel unrecognized"
        );
        let preview = truncate_for_log(raw, RAW_LOG_MAX_BYTES);
        debug!(
            sample_index = count + 1,
            sample_limit = OTHER_LOG_LIMIT,
            bytes = raw.len(),
            message = %preview,
            "ws message channel unrecognized"
        );
    }
}

fn log_parse_fail_once(err: &serde_json::Error, raw: &str) {
    let count = PARSE_FAIL_LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    if count < PARSE_FAIL_LOG_LIMIT {
        let preview = truncate_for_log(raw, RAW_LOG_MAX_BYTES);
        info!(
            sample_index = count + 1,
            sample_limit = PARSE_FAIL_LOG_LIMIT,
            error = %err,
            bytes = raw.len(),
            raw_preview = %preview,
            "ws message parse failed"
        );
        debug!(
            sample_index = count + 1,
            sample_limit = PARSE_FAIL_LOG_LIMIT,
            error = %err,
            bytes = raw.len(),
            message = %preview,
            "ws message parse failed"
        );
    }
}

fn log_error_response_once(value: &Value) {
    let count = ERROR_RESPONSE_LOG_COUNT.fetch_add(1, Ordering::Relaxed);
    if count >= ERROR_RESPONSE_LOG_LIMIT {
        return;
    }

    let code = value
        .get("code")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let message = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let preview = truncate_for_log(&value.to_string(), RAW_LOG_MAX_BYTES);
    info!(
        sample_index = count + 1,
        sample_limit = ERROR_RESPONSE_LOG_LIMIT,
        code,
        message,
        raw_preview = %preview,
        "ws error response"
    );
}

fn truncate_for_log(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_string();
    }
    let mut out = String::with_capacity(max_len + 3);
    out.push_str(&value[..max_len]);
    out.push_str("...");
    out
}

fn infer_position_message(value: &Value) -> Option<WebSocketMessage> {
    if !looks_like_position(value) {
        return None;
    }

    let data = value.get("data").cloned().unwrap_or_else(|| value.clone());
    Some(WebSocketMessage::Position { data })
}

fn looks_like_error_response(value: &Value) -> bool {
    value.get("code").is_some() && value.get("message").is_some()
}

fn looks_like_position(value: &Value) -> bool {
    if let Some(channel) = value.get("channel").and_then(|entry| entry.as_str()) {
        return channel == "position";
    }

    for key in ["topic", "type", "event", "action"] {
        if let Some(value) = value.get(key).and_then(|entry| entry.as_str())
            && value.contains("position")
        {
            return true;
        }
    }

    if let Some(data) = value.get("data") {
        if data.get("symbol").is_some() && data.get("qty").is_some() {
            return true;
        }
        if data.get("positions").is_some() {
            return true;
        }
    }

    value.get("symbol").is_some() && value.get("qty").is_some()
}
