use standx_point_adapter::ws::{StandxWebSocket, WebSocketMessage};
use standx_point_adapter::types::SymbolPrice;
use tokio::sync::{mpsc, watch};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use std::time::Duration;
use rust_decimal::Decimal;
use std::str::FromStr;

pub struct PriceService {
    websocket: Arc<tokio::sync::Mutex<StandxWebSocket>>,
    price_watches: HashMap<String, watch::Sender<SymbolPrice>>,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Option<mpsc::Receiver<()>>,
    websocket_handle: Option<JoinHandle<()>>,
}

pub struct PriceSubscription {
    pub symbol: String,
    pub receiver: watch::Receiver<SymbolPrice>,
}

impl PriceService {
    pub fn new(websocket: StandxWebSocket) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        Self {
            websocket: Arc::new(tokio::sync::Mutex::new(websocket)),
            price_watches: HashMap::new(),
            shutdown_tx,
            shutdown_rx: Some(shutdown_rx),
            websocket_handle: None,
        }
    }
    
    pub async fn start(&mut self) -> anyhow::Result<()> {
        self.websocket.lock().await.connect_market_stream().await.map_err(|e| anyhow::anyhow!("{}", e))?;
        
        let mut shutdown_rx = self.shutdown_rx.take().expect("Shutdown receiver should be available");
        let websocket = Arc::clone(&self.websocket);
        
        let price_watches = Arc::new(tokio::sync::RwLock::new(self.price_watches.clone()));
        
        let handle = tokio::spawn(async move {
            let mut backoff = Duration::from_millis(100);
            const MAX_BACKOFF: Duration = Duration::from_secs(30);
            
            loop {
                match Self::websocket_handler(Arc::clone(&websocket), Arc::clone(&price_watches)).await {
                    Ok(_) => {
                        tracing::info!("WebSocket handler completed successfully");
                        break;
                    }
                    Err(e) => {
                        tracing::error!("WebSocket handler failed: {e}, reconnecting in {backoff:?}");
                        
                        tokio::select! {
                            _ = shutdown_rx.recv() => {
                                tracing::info!("Shutdown signal received, stopping reconnection attempts");
                                break;
                            }
                            _ = tokio::time::sleep(backoff) => {
                                backoff = std::cmp::min(backoff * 2, MAX_BACKOFF);
                            }
                        }
                    }
                }
            }
        });
        
        self.websocket_handle = Some(handle);
        Ok(())
    }
    
    async fn websocket_handler(
        websocket: Arc<tokio::sync::Mutex<StandxWebSocket>>,
        price_watches: Arc<tokio::sync::RwLock<HashMap<String, watch::Sender<SymbolPrice>>>>,
    ) -> anyhow::Result<()> {
        let mut message_rx = match websocket.lock().await.take_receiver() {
            Some(rx) => rx,
            None => return Err(anyhow::anyhow!("WebSocket receiver already taken")),
        };
        
        while let Some(msg) = message_rx.recv().await {
            match msg {
                WebSocketMessage::Price { data, .. } => {
                    if let Ok(price_data) = serde_json::from_value::<PriceData>(data) {
                        let price = SymbolPrice {
                            base: price_data.base,
                            index_price: Decimal::from_str(&price_data.index_price)
                                .unwrap_or_else(|_| Decimal::ZERO),
                            last_price: Decimal::from_str(&price_data.last_price).ok(),
                            mark_price: Decimal::from_str(&price_data.mark_price)
                                .unwrap_or_else(|_| Decimal::ZERO),
                            mid_price: Decimal::from_str(&price_data.mid_price).ok(),
                            quote: price_data.quote,
                            spread_ask: price_data.spread.get(0).and_then(|s| Decimal::from_str(s).ok()),
                            spread_bid: price_data.spread.get(1).and_then(|s| Decimal::from_str(s).ok()),
                            symbol: price_data.symbol.clone(),
                            time: price_data.time,
                        };
                        
                        let watches = price_watches.read().await;
                        if let Some(sender) = watches.get(&price_data.symbol) {
                            let _ = sender.send(price);
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    pub async fn subscribe(&mut self, symbol: String) -> anyhow::Result<PriceSubscription> {
        if let Some(sender) = self.price_watches.get(&symbol) {
            return Ok(PriceSubscription {
                symbol: symbol.clone(),
                receiver: sender.subscribe(),
            });
        }
        
        if self.price_watches.len() >= 10 {
            return Err(anyhow::anyhow!("Maximum 10 symbol subscriptions allowed"));
        }
        
        let initial_price = SymbolPrice {
            base: String::new(),
            index_price: Decimal::ZERO,
            last_price: None,
            mark_price: Decimal::ZERO,
            mid_price: None,
            quote: String::new(),
            spread_ask: None,
            spread_bid: None,
            symbol: symbol.clone(),
            time: String::new(),
        };
        let (sender, receiver) = watch::channel(initial_price);
        
        self.websocket.lock().await.subscribe_price(&symbol).await.map_err(|e| anyhow::anyhow!("{}", e))?;
        
        self.price_watches.insert(symbol.clone(), sender);
        
        Ok(PriceSubscription {
            symbol,
            receiver,
        })
    }
    
    pub async fn unsubscribe(&mut self, symbol: &str) -> anyhow::Result<()> {
        self.websocket.lock().await.unsubscribe_price(symbol).await.map_err(|e| anyhow::anyhow!("{}", e))?;
        self.price_watches.remove(symbol);
        Ok(())
    }
    
    pub fn get_current_price(&self, symbol: &str) -> Option<SymbolPrice> {
        self.price_watches.get(symbol).map(|sender| {
            sender.borrow().clone()
        })
    }
    
    pub async fn shutdown(mut self) -> anyhow::Result<()> {
        let _ = self.shutdown_tx.send(()).await;
        
        if let Some(handle) = self.websocket_handle.take() {
            let _ = handle.await;
        }
        
        self.price_watches.clear();
        
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct PriceData {
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