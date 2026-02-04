/*
[INPUT]:  StandX market WebSocket stream + per-symbol subscriptions.
[OUTPUT]: Latest-per-symbol price snapshots via `watch` + connection state notifications.
[POS]:    Data layer - shared market data distribution (no trading logic).
[UPDATE]: When changing subscription channels, reconnection backoff, or shutdown semantics.
*/

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::Duration;

use rust_decimal::Decimal;
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use standx_point_adapter::{PriceData, StandxWebSocket, SymbolPrice, WebSocketMessage};

const DEFAULT_WS_URL: &str = "wss://perps.standx.com/ws-stream/v1";
const DEFAULT_MAX_RETRIES: u32 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    Disconnected { retry_count: u32 },
    Paused,
}

#[derive(Debug)]
enum HubCommand {
    TrackSymbol {
        symbol: String,
        price_tx: watch::Sender<SymbolPrice>,
    },
    Shutdown,
}

/// Market data hub that distributes price updates to all tasks.
///
/// This is intentionally data-only: it connects, subscribes, parses, and broadcasts.
#[derive(Debug)]
pub struct MarketDataHub {
    ws_url: String,
    symbols: Vec<String>,
    price_txs: HashMap<String, watch::Sender<SymbolPrice>>,
    connection_state: watch::Sender<ConnectionState>,
    shutdown: CancellationToken,
    cmd_tx: mpsc::UnboundedSender<HubCommand>,
    cmd_rx: Option<mpsc::UnboundedReceiver<HubCommand>>,
    worker_handle: Option<tokio::task::JoinHandle<()>>,
    auto_connect: bool,
}

impl MarketDataHub {
    /// Create a new market data hub.
    ///
    /// Note: this starts the internal worker lazily on first subscription.
    pub fn new() -> Self {
        Self::new_internal(true)
    }

    #[cfg(test)]
    fn new_for_test() -> Self {
        Self::new_internal(false)
    }

    fn new_internal(auto_connect: bool) -> Self {
        let (connection_state, _rx) =
            watch::channel(ConnectionState::Disconnected { retry_count: 0 });
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        Self {
            ws_url: DEFAULT_WS_URL.to_string(),
            symbols: Vec::new(),
            price_txs: HashMap::new(),
            connection_state,
            shutdown: CancellationToken::new(),
            cmd_tx,
            cmd_rx: Some(cmd_rx),
            worker_handle: None,
            auto_connect,
        }
    }

    /// Subscribe to connection state changes.
    pub fn subscribe_connection_state(&self) -> watch::Receiver<ConnectionState> {
        self.connection_state.subscribe()
    }

    /// Subscribe to price updates for a symbol.
    ///
    /// This returns a `watch::Receiver` that always contains the latest snapshot.
    pub fn subscribe_price(&mut self, symbol: &str) -> watch::Receiver<SymbolPrice> {
        if self.auto_connect {
            self.start_worker_if_needed();
        }

        if let Some(existing) = self.price_txs.get(symbol) {
            return existing.subscribe();
        }

        let initial = initial_symbol_price(symbol);
        let (tx, rx) = watch::channel(initial);
        self.price_txs.insert(symbol.to_string(), tx.clone());

        if !self.symbols.iter().any(|s| s == symbol) {
            self.symbols.push(symbol.to_string());
        }

        let _ = self.cmd_tx.send(HubCommand::TrackSymbol {
            symbol: symbol.to_string(),
            price_tx: tx,
        });

        rx
    }

    /// Trigger a graceful shutdown of the internal worker.
    pub fn shutdown(&self) {
        self.shutdown.cancel();
        let _ = self.cmd_tx.send(HubCommand::Shutdown);
    }

    fn start_worker_if_needed(&mut self) {
        if self.worker_handle.is_some() {
            return;
        }

        let Some(cmd_rx) = self.cmd_rx.take() else {
            return;
        };

        if tokio::runtime::Handle::try_current().is_err() {
            warn!("MarketDataHub created without Tokio runtime; worker not started");
            self.cmd_rx = Some(cmd_rx);
            return;
        }

        let ws_url = self.ws_url.clone();
        let connection_state = self.connection_state.clone();
        let shutdown = self.shutdown.clone();

        self.worker_handle = Some(tokio::spawn(async move {
            let worker = MarketDataHubWorker::new(ws_url, cmd_rx, connection_state, shutdown);
            worker.run().await;
        }));
    }
}

impl Default for MarketDataHub {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MarketDataHub {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[derive(Debug)]
struct MarketDataHubWorker {
    ws_url: String,
    tracked_symbols: HashSet<String>,
    price_txs: HashMap<String, watch::Sender<SymbolPrice>>,
    cmd_rx: mpsc::UnboundedReceiver<HubCommand>,
    connection_state: watch::Sender<ConnectionState>,
    shutdown: CancellationToken,
    max_retries: u32,
}

impl MarketDataHubWorker {
    fn new(
        ws_url: String,
        cmd_rx: mpsc::UnboundedReceiver<HubCommand>,
        connection_state: watch::Sender<ConnectionState>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            ws_url,
            tracked_symbols: HashSet::new(),
            price_txs: HashMap::new(),
            cmd_rx,
            connection_state,
            shutdown,
            max_retries: DEFAULT_MAX_RETRIES,
        }
    }

    async fn run(mut self) {
        let mut retry_count: u32 = 0;

        'run: loop {
            if self.shutdown.is_cancelled() {
                let _ = self
                    .connection_state
                    .send(ConnectionState::Disconnected { retry_count });
                break 'run;
            }

            if self.tracked_symbols.is_empty() {
                tokio::select! {
                    _ = self.shutdown.cancelled() => {
                        let _ = self.connection_state.send(ConnectionState::Disconnected { retry_count });
                        break 'run;
                    }
                    cmd = self.cmd_rx.recv() => {
                        match cmd {
                            Some(HubCommand::TrackSymbol { symbol, price_tx }) => {
                                self.track_symbol(symbol, price_tx);
                            }
                            Some(HubCommand::Shutdown) | None => {
                                let _ = self.connection_state.send(ConnectionState::Disconnected { retry_count });
                                break 'run;
                            }
                        }
                    }
                }

                continue;
            }

            let _ = self.connection_state.send(ConnectionState::Paused);

            match self.connect_once().await {
                Ok((ws, mut rx)) => {
                    retry_count = 0;

                    let _ = self.connection_state.send(ConnectionState::Connected);
                    info!("Market data hub connected");

                    match self.stream_loop(&ws, &mut rx).await {
                        StreamExit::Shutdown => {
                            drop(rx);
                            drop(ws);
                            let _ = self
                                .connection_state
                                .send(ConnectionState::Disconnected { retry_count });
                            break 'run;
                        }
                        StreamExit::Disconnected => {
                            drop(rx);
                            drop(ws);
                            retry_count = 0;
                            let _ = self.connection_state.send(ConnectionState::Paused);
                            continue 'run;
                        }
                    }
                }
                Err(err_msg) => {
                    retry_count = retry_count.saturating_add(1);

                    let _ = self
                        .connection_state
                        .send(ConnectionState::Disconnected { retry_count });

                    if retry_count >= self.max_retries {
                        warn!(retry_count, max_retries = self.max_retries, error = %err_msg, "Market data hub gave up reconnecting");
                        break 'run;
                    }

                    let backoff = backoff_duration(retry_count);
                    let _ = self.connection_state.send(ConnectionState::Paused);
                    warn!(retry_count, ?backoff, error = %err_msg, "Market data hub connect failed; retrying with backoff");

                    tokio::select! {
                        _ = self.shutdown.cancelled() => {
                            let _ = self.connection_state.send(ConnectionState::Disconnected { retry_count });
                            break 'run;
                        }
                        _ = tokio::time::sleep(backoff) => {}
                        cmd = self.cmd_rx.recv() => {
                            match cmd {
                                Some(HubCommand::TrackSymbol { symbol, price_tx }) => {
                                    self.track_symbol(symbol, price_tx);
                                }
                                Some(HubCommand::Shutdown) | None => {
                                    let _ = self.connection_state.send(ConnectionState::Disconnected { retry_count });
                                    break 'run;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    async fn connect_once(
        &self,
    ) -> Result<(StandxWebSocket, mpsc::Receiver<WebSocketMessage>), String> {
        let mut ws = StandxWebSocket::new();

        info!(ws_url = %self.ws_url, "Connecting to StandX market WebSocket");
        ws.connect_market_stream()
            .await
            .map_err(|err| err.to_string())?;
        self.subscribe_tracked_symbols(&ws).await?;

        let rx = ws
            .take_receiver()
            .ok_or_else(|| "StandxWebSocket receiver already taken".to_string())?;

        Ok((ws, rx))
    }

    async fn stream_loop(
        &mut self,
        ws: &StandxWebSocket,
        rx: &mut mpsc::Receiver<WebSocketMessage>,
    ) -> StreamExit {
        loop {
            tokio::select! {
                _ = self.shutdown.cancelled() => {
                    debug!("Market data hub shutdown requested");
                    return StreamExit::Shutdown;
                }
                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(HubCommand::TrackSymbol { symbol, price_tx }) => {
                            self.track_symbol(symbol.clone(), price_tx);
                            if let Err(err) = self.subscribe_symbol(ws, &symbol).await {
                                warn!(%symbol, error = %err, "Failed to subscribe symbol while connected");
                                return StreamExit::Disconnected;
                            }
                        }
                        Some(HubCommand::Shutdown) | None => {
                            return StreamExit::Shutdown;
                        }
                    }
                }
                msg = rx.recv() => {
                    match msg {
                        Some(message) => {
                            self.handle_ws_message(message);
                        }
                        None => {
                            warn!("Market WebSocket stream ended");
                            return StreamExit::Disconnected;
                        }
                    }
                }
            }
        }
    }

    fn track_symbol(&mut self, symbol: String, price_tx: watch::Sender<SymbolPrice>) {
        self.tracked_symbols.insert(symbol.clone());
        self.price_txs.insert(symbol, price_tx);
    }

    async fn subscribe_tracked_symbols(&self, ws: &StandxWebSocket) -> Result<(), String> {
        for symbol in &self.tracked_symbols {
            self.subscribe_symbol(ws, symbol).await?;
        }
        Ok(())
    }

    async fn subscribe_symbol(&self, ws: &StandxWebSocket, symbol: &str) -> Result<(), String> {
        ws.subscribe_price(symbol)
            .await
            .map_err(|err| err.to_string())?;
        ws.subscribe_depth(symbol)
            .await
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    fn handle_ws_message(&self, message: WebSocketMessage) {
        match message {
            WebSocketMessage::Price { symbol, data } => {
                match serde_json::from_value::<PriceData>(data) {
                    Ok(price_data) => {
                        let Some(price) = symbol_price_from_price_data(price_data) else {
                            debug!(%symbol, "Failed to parse decimals from price payload");
                            return;
                        };

                        if let Some(tx) = self.price_txs.get(&symbol) {
                            let _ = tx.send(price);
                        } else {
                            debug!(%symbol, "Received price for untracked symbol");
                        }
                    }
                    Err(err) => {
                        debug!(%symbol, error = %err, "Failed to deserialize price payload");
                    }
                }
            }
            WebSocketMessage::DepthBook { .. } => {}
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamExit {
    Disconnected,
    Shutdown,
}

fn backoff_duration(retry_count: u32) -> Duration {
    let exp = retry_count.saturating_sub(1).min(63);
    let secs = 1u64 << exp;
    Duration::from_secs(secs.min(30))
}

fn initial_symbol_price(symbol: &str) -> SymbolPrice {
    SymbolPrice {
        base: String::new(),
        index_price: Decimal::ZERO,
        last_price: None,
        mark_price: Decimal::ZERO,
        mid_price: None,
        quote: String::new(),
        spread_ask: None,
        spread_bid: None,
        symbol: symbol.to_string(),
        time: String::new(),
    }
}

fn symbol_price_from_price_data(data: PriceData) -> Option<SymbolPrice> {
    let parse_decimal_str = |s: &str| {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        Decimal::from_str(s).ok()
    };

    let index_price = parse_decimal_str(&data.index_price)?;
    let mark_price = parse_decimal_str(&data.mark_price)?;

    let last_price = parse_decimal_str(&data.last_price);
    let mid_price = parse_decimal_str(&data.mid_price);

    let spread_bid = data.spread.get(0).and_then(|s| parse_decimal_str(s));
    let spread_ask = data.spread.get(1).and_then(|s| parse_decimal_str(s));

    Some(SymbolPrice {
        base: data.base,
        index_price,
        last_price,
        mark_price,
        mid_price,
        quote: data.quote,
        spread_ask,
        spread_bid,
        symbol: data.symbol,
        time: data.time,
    })
}

// Note: parsing helpers are implemented inline in `handle_ws_message` to avoid taking a direct
// dependency on `serde_json` from this crate.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_data_backoff_clamps_at_30s() {
        assert_eq!(backoff_duration(1), Duration::from_secs(1));
        assert_eq!(backoff_duration(2), Duration::from_secs(2));
        assert_eq!(backoff_duration(3), Duration::from_secs(4));
        assert_eq!(backoff_duration(4), Duration::from_secs(8));
        assert_eq!(backoff_duration(5), Duration::from_secs(16));
        assert_eq!(backoff_duration(6), Duration::from_secs(30));
        assert_eq!(backoff_duration(10), Duration::from_secs(30));
    }

    #[tokio::test]
    async fn market_data_watch_broadcasts_latest_price() {
        let mut hub = MarketDataHub::new_for_test();

        let mut rx = hub.subscribe_price("BTCUSDT");
        let tx = hub
            .price_txs
            .get("BTCUSDT")
            .expect("price sender exists")
            .clone();

        let next = SymbolPrice {
            base: "BTC".to_string(),
            index_price: Decimal::from_str("100").unwrap(),
            last_price: Some(Decimal::from_str("101").unwrap()),
            mark_price: Decimal::from_str("100.5").unwrap(),
            mid_price: Some(Decimal::from_str("100.7").unwrap()),
            quote: "USDT".to_string(),
            spread_ask: Some(Decimal::from_str("100.8").unwrap()),
            spread_bid: Some(Decimal::from_str("100.6").unwrap()),
            symbol: "BTCUSDT".to_string(),
            time: "2026-02-03T00:00:00Z".to_string(),
        };

        let _ = tx.send(next.clone());
        rx.changed().await.unwrap();
        assert_eq!(&*rx.borrow(), &next);
    }

    #[tokio::test]
    async fn market_data_subscribe_price_reuses_sender() {
        let mut hub = MarketDataHub::new_for_test();

        let mut rx1 = hub.subscribe_price("BTCUSDT");
        let mut rx2 = hub.subscribe_price("BTCUSDT");

        let tx = hub
            .price_txs
            .get("BTCUSDT")
            .expect("price sender exists")
            .clone();

        let next = SymbolPrice {
            base: "BTC".to_string(),
            index_price: Decimal::from_str("100").unwrap(),
            last_price: Some(Decimal::from_str("101").unwrap()),
            mark_price: Decimal::from_str("100.5").unwrap(),
            mid_price: Some(Decimal::from_str("100.7").unwrap()),
            quote: "USDT".to_string(),
            spread_ask: Some(Decimal::from_str("100.8").unwrap()),
            spread_bid: Some(Decimal::from_str("100.6").unwrap()),
            symbol: "BTCUSDT".to_string(),
            time: "2026-02-03T00:00:00Z".to_string(),
        };

        let _ = tx.send(next.clone());

        rx1.changed().await.unwrap();
        rx2.changed().await.unwrap();

        assert_eq!(&*rx1.borrow(), &next);
        assert_eq!(&*rx2.borrow(), &next);
    }

    #[tokio::test]
    async fn market_data_connection_state_broadcasts_updates() {
        let hub = MarketDataHub::new_for_test();

        let mut rx = hub.subscribe_connection_state();
        assert_eq!(
            &*rx.borrow(),
            &ConnectionState::Disconnected { retry_count: 0 }
        );

        hub.connection_state.send(ConnectionState::Paused).unwrap();
        rx.changed().await.unwrap();
        assert_eq!(&*rx.borrow(), &ConnectionState::Paused);
    }
}
