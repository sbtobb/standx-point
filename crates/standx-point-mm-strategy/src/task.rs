/*
[INPUT]:  TaskConfig + StandxClient (per task), watch::Receiver<SymbolPrice>, CancellationToken
[OUTPUT]: Tokio tasks running lifecycle (startup -> run -> shutdown) with best-effort cleanup
[POS]:    Execution layer - per-task trading orchestration
[UPDATE]: When changing startup/shutdown guarantees or supervision semantics
*/

use crate::config::{CredentialsConfig, StrategyConfig, TaskConfig};
use crate::market_data::MarketDataHub;
use anyhow::{Context as _, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rust_decimal::Decimal;
use standx_point_adapter::{
    CancelOrderRequest, Chain, ClientConfig, Credentials, Ed25519Signer, NewOrderRequest,
    OrderType, Side, StandxClient, SymbolPrice, TimeInForce,
};
use std::sync::Once;
use std::time::Duration;
use tokio::sync::{Mutex, watch};
use tokio::task::JoinHandle;
use tokio::time::{Instant, Sleep};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

static PANIC_HOOK_ONCE: Once = Once::new();

fn ensure_panic_hook_installed() {
    PANIC_HOOK_ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            tracing::error!("panic in task: {info}");
            previous(info);
        }));
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Init,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

/// Task manager that coordinates multiple trading tasks.
#[derive(Debug)]
pub struct TaskManager {
    tasks: Vec<JoinHandle<Result<()>>>,

    #[cfg_attr(test, allow(dead_code))]
    market_data_hub: std::sync::Arc<Mutex<MarketDataHub>>,
    shutdown: CancellationToken,

    #[cfg(test)]
    test_price_txs: Vec<watch::Sender<SymbolPrice>>,
}

impl TaskManager {
    /// Create a new task manager.
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            market_data_hub: std::sync::Arc::new(Mutex::new(MarketDataHub::new())),
            shutdown: CancellationToken::new(),

            #[cfg(test)]
            test_price_txs: Vec::new(),
        }
    }

    pub fn with_market_data_hub(market_data_hub: std::sync::Arc<Mutex<MarketDataHub>>) -> Self {
        Self {
            tasks: Vec::new(),
            market_data_hub,
            shutdown: CancellationToken::new(),

            #[cfg(test)]
            test_price_txs: Vec::new(),
        }
    }

    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown.clone()
    }

    /// Spawn tasks from configuration using the default StandxClient builder.
    pub async fn spawn_from_config(&mut self, config: StrategyConfig) -> Result<()> {
        self.spawn_from_config_with_client_builder(config, |task_config| {
            Task::build_client(task_config)
        })
        .await
    }

    /// Spawn tasks from configuration using a custom StandxClient builder.
    ///
    /// This is primarily intended for tests where callers inject wiremock base URLs.
    pub async fn spawn_from_config_with_client_builder<F>(
        &mut self,
        config: StrategyConfig,
        build_client: F,
    ) -> Result<()>
    where
        F: Fn(&TaskConfig) -> Result<StandxClient>,
    {
        ensure_panic_hook_installed();

        for task_config in config.tasks {
            let client = build_client(&task_config)
                .with_context(|| format!("build StandxClient for task_id={}", task_config.id))?;

            let price_rx = self.subscribe_price(&task_config.symbol).await;
            let shutdown = self.shutdown.child_token();

            let task = Task::new_with_client(task_config, client, price_rx, shutdown);
            self.tasks.push(task.spawn());
        }

        Ok(())
    }

    /// Request graceful shutdown and wait for all tasks to exit.
    ///
    /// Guarantees a bounded shutdown time (30s) and aborts remaining tasks on timeout.
    pub async fn shutdown_and_wait(&mut self) -> Result<()> {
        self.shutdown.cancel();
        self.join_all_with_deadline(SHUTDOWN_TIMEOUT).await
    }

    async fn join_all_with_deadline(&mut self, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;

        // Drain handles so we can abort remaining ones on timeout.
        let mut handles = std::mem::take(&mut self.tasks);

        while let Some(mut handle) = handles.pop() {
            let sleep = sleep_until_deadline(deadline);

            tokio::select! {
                res = &mut handle => {
                    match res {
                        Ok(Ok(())) => {}
                        Ok(Err(err)) => {
                            self.shutdown.cancel();
                            abort_all(handles);
                            return Err(err).context("task returned error");
                        }
                        Err(join_err) => {
                            self.shutdown.cancel();
                            abort_all(handles);
                            if join_err.is_panic() {
                                return Err(anyhow!("task panicked: {join_err}"));
                            }
                            return Err(anyhow!("task join error: {join_err}"));
                        }
                    }
                }
                _ = sleep => {
                    handle.abort();
                    abort_all(handles);
                    return Err(anyhow!("shutdown timed out after {timeout:?}"));
                }
            }
        }

        Ok(())
    }

    async fn subscribe_price(&mut self, symbol: &str) -> watch::Receiver<SymbolPrice> {
        #[cfg(test)]
        {
            let initial = dummy_symbol_price(symbol);
            let (tx, rx) = watch::channel(initial);
            self.test_price_txs.push(tx);
            return rx;
        }

        #[cfg(not(test))]
        {
            let mut hub = self.market_data_hub.lock().await;
            hub.subscribe_price(symbol)
        }
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual trading task.
#[derive(Debug)]
pub struct Task {
    id: Uuid,
    config: TaskConfig,
    client: StandxClient,
    price_rx: watch::Receiver<SymbolPrice>,
    state: TaskState,
    shutdown: CancellationToken,
}

impl Task {
    /// Create a new trading task.
    ///
    /// This is a placeholder constructor. Real tasks should be created from config.
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(dummy_symbol_price("DUMMY"));
        drop(tx);
        let client = StandxClient::new().expect("StandxClient::new should succeed");

        Self {
            id: Uuid::new_v4(),
            config: dummy_task_config(),
            client,
            price_rx: rx,
            state: TaskState::Init,
            shutdown: CancellationToken::new(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn config(&self) -> &TaskConfig {
        &self.config
    }

    pub fn spawn(self) -> JoinHandle<Result<()>> {
        tokio::spawn(async move { self.run().await })
    }

    pub fn new_with_client(
        config: TaskConfig,
        client: StandxClient,
        price_rx: watch::Receiver<SymbolPrice>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            config,
            client,
            price_rx,
            state: TaskState::Init,
            shutdown,
        }
    }

    pub fn try_from_config(
        config: TaskConfig,
        price_rx: watch::Receiver<SymbolPrice>,
        shutdown: CancellationToken,
    ) -> Result<Self> {
        let client = Self::build_client(&config)?;
        Ok(Self::new_with_client(config, client, price_rx, shutdown))
    }

    pub fn build_client(config: &TaskConfig) -> Result<StandxClient> {
        Self::build_client_with_config_and_base_urls(
            config,
            ClientConfig::default(),
            "https://api.standx.com",
            "https://perps.standx.com",
        )
    }

    pub fn build_client_with_config_and_base_urls(
        config: &TaskConfig,
        client_config: ClientConfig,
        auth_base_url: &str,
        trading_base_url: &str,
    ) -> Result<StandxClient> {
        let mut client =
            StandxClient::with_config_and_base_urls(client_config, auth_base_url, trading_base_url)
                .map_err(|err| anyhow!("create StandxClient failed: {err}"))?;

        let secret_key = decode_ed25519_secret_key_base64(&config.credentials.signing_key)
            .context("decode signing_key (base64) failed")?;
        let signer = Ed25519Signer::from_secret_key(&secret_key);

        // NOTE: wallet_address/chain are not required for trading endpoints today.
        // Keep placeholders until StrategyConfig carries these fields.
        client.set_credentials_and_signer(
            Credentials {
                jwt_token: config.credentials.jwt_token.clone(),
                wallet_address: "unknown".to_string(),
                chain: Chain::Bsc,
            },
            signer,
        );

        Ok(client)
    }

    async fn run(mut self) -> Result<()> {
        self.state = TaskState::Starting;
        tracing::info!(
            task_uuid = %self.id,
            task_id = %self.config.id,
            symbol = %self.config.symbol,
            "task starting"
        );

        if let Err(err) = self.startup_sequence().await {
            self.state = TaskState::Failed;
            return Err(err).context("startup sequence failed");
        }

        self.state = TaskState::Running;
        tracing::info!(
            task_uuid = %self.id,
            task_id = %self.config.id,
            symbol = %self.config.symbol,
            "task running"
        );

        loop {
            tokio::select! {
                _ = self.shutdown.cancelled() => {
                    self.state = TaskState::Stopping;
                    tracing::info!(
                        task_uuid = %self.id,
                        task_id = %self.config.id,
                        symbol = %self.config.symbol,
                        "task stopping"
                    );

                    let shutdown_res = self.shutdown_sequence().await;
                    self.state = match shutdown_res {
                        Ok(()) => TaskState::Stopped,
                        Err(_) => TaskState::Failed,
                    };

                    return shutdown_res;
                }
                changed = self.price_rx.changed() => {
                    if changed.is_err() {
                        // Sender dropped (e.g. hub stopped). Keep waiting for shutdown signal.
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        continue;
                    }

                    let _price = self.price_rx.borrow().clone();
                    // Strategy logic is intentionally not implemented here.
                }
            }
        }
    }

    async fn startup_sequence(&mut self) -> Result<()> {
        // Startup sequence: query -> cancel -> trade.
        self.cancel_open_orders().await
    }

    async fn shutdown_sequence(&mut self) -> Result<()> {
        // Shutdown sequence: cancel open orders -> close positions.
        // This is best-effort and should remain minimal.
        self.cancel_open_orders().await?;
        self.close_positions().await?;
        Ok(())
    }

    async fn cancel_open_orders(&self) -> Result<()> {
        let symbol = self.config.symbol.as_str();
        let orders = self
            .client
            .query_open_orders(Some(symbol))
            .await
            .context("query_open_orders failed")?;

        let mut first_error: Option<anyhow::Error> = None;

        for order in orders.result {
            let req = CancelOrderRequest {
                order_id: Some(order.id),
                cl_ord_id: None,
            };

            if let Err(err) = self.client.cancel_order(req).await {
                tracing::warn!(
                    task_uuid = %self.id,
                    task_id = %self.config.id,
                    symbol = %symbol,
                    order_id = order.id,
                    "cancel_order failed: {err}"
                );

                if first_error.is_none() {
                    first_error = Some(anyhow!(err));
                }
            }
        }

        if let Some(err) = first_error {
            return Err(err).context("one or more cancels failed");
        }

        Ok(())
    }

    async fn close_positions(&self) -> Result<()> {
        let symbol = self.config.symbol.as_str();
        let positions = self
            .client
            .query_positions(Some(symbol))
            .await
            .context("query_positions failed")?;

        let mut first_error: Option<anyhow::Error> = None;

        for position in positions {
            if position.qty.is_zero() {
                continue;
            }

            let (side, qty) = if position.qty.is_sign_positive() {
                (Side::Sell, position.qty)
            } else {
                (Side::Buy, position.qty.abs())
            };

            let req = NewOrderRequest {
                symbol: position.symbol.clone(),
                side,
                order_type: OrderType::Market,
                qty,
                time_in_force: TimeInForce::Ioc,
                reduce_only: true,
                price: None,
                cl_ord_id: None,
                margin_mode: None,
                leverage: None,
                tp_price: None,
                sl_price: None,
            };

            match self.client.new_order(req).await {
                Ok(resp) if resp.code == 0 => {}
                Ok(resp) => {
                    let err = anyhow!(
                        "new_order returned code={} message={}",
                        resp.code,
                        resp.message
                    );
                    tracing::warn!(
                        task_uuid = %self.id,
                        task_id = %self.config.id,
                        symbol = %symbol,
                        "close position failed: {err}"
                    );
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        task_uuid = %self.id,
                        task_id = %self.config.id,
                        symbol = %symbol,
                        "close position HTTP failed: {err}"
                    );
                    if first_error.is_none() {
                        first_error = Some(anyhow!(err));
                    }
                }
            }
        }

        if let Some(err) = first_error {
            return Err(err).context("one or more position closes failed");
        }

        Ok(())
    }
}

impl Default for Task {
    fn default() -> Self {
        Self::new()
    }
}

fn decode_ed25519_secret_key_base64(encoded: &str) -> Result<[u8; 32]> {
    let decoded = BASE64
        .decode(encoded.trim())
        .map_err(|err| anyhow!("base64 decode failed: {err}"))?;

    match decoded.len() {
        32 => {
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&decoded);
            Ok(bytes)
        }
        64 => {
            // Common representation: 32-byte seed + 32-byte public key.
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&decoded[..32]);
            Ok(bytes)
        }
        other => Err(anyhow!(
            "unexpected signing key length after base64 decode: {other} (expected 32 or 64)"
        )),
    }
}

fn dummy_task_config() -> TaskConfig {
    TaskConfig {
        id: "dummy".to_string(),
        symbol: "DUMMY".to_string(),
        credentials: CredentialsConfig {
            jwt_token: "".to_string(),
            signing_key: "".to_string(),
        },
        risk: crate::config::RiskConfig {
            level: "conservative".to_string(),
            max_position_usd: "0".to_string(),
            price_jump_threshold_bps: 0,
        },
        sizing: crate::config::SizingConfig {
            base_qty: "0".to_string(),
            tiers: 1,
        },
    }
}

fn dummy_symbol_price(symbol: &str) -> SymbolPrice {
    SymbolPrice {
        base: "DUMMY".to_string(),
        index_price: Decimal::ZERO,
        last_price: None,
        mark_price: Decimal::ZERO,
        mid_price: None,
        quote: "DUMMY".to_string(),
        spread_ask: None,
        spread_bid: None,
        symbol: symbol.to_string(),
        time: "0".to_string(),
    }
}

fn sleep_until_deadline(deadline: Instant) -> Sleep {
    tokio::time::sleep_until(deadline)
}

fn abort_all(handles: Vec<JoinHandle<Result<()>>>) {
    for handle in handles {
        handle.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;
    use standx_point_adapter::RequestSigner;
    use standx_point_adapter::http::signature::{
        HEADER_REQUEST_ID, HEADER_REQUEST_SIGNATURE, HEADER_REQUEST_TIMESTAMP,
        HEADER_REQUEST_VERSION,
    };
    use std::str;
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

    #[derive(Clone)]
    struct ValidBodySignatureMatcher {
        secret_key: [u8; 32],
    }

    impl Match for ValidBodySignatureMatcher {
        fn matches(&self, request: &Request) -> bool {
            let version = match request.headers.get(HEADER_REQUEST_VERSION) {
                Some(value) => match value.to_str() {
                    Ok(s) => s,
                    Err(_) => return false,
                },
                None => return false,
            };

            let request_id = match request.headers.get(HEADER_REQUEST_ID) {
                Some(value) => match value.to_str() {
                    Ok(s) => s,
                    Err(_) => return false,
                },
                None => return false,
            };

            let timestamp_str = match request.headers.get(HEADER_REQUEST_TIMESTAMP) {
                Some(value) => match value.to_str() {
                    Ok(s) => s,
                    Err(_) => return false,
                },
                None => return false,
            };

            let timestamp: u64 = match timestamp_str.parse() {
                Ok(v) => v,
                Err(_) => return false,
            };

            let signature = match request.headers.get(HEADER_REQUEST_SIGNATURE) {
                Some(value) => match value.to_str() {
                    Ok(s) => s,
                    Err(_) => return false,
                },
                None => return false,
            };

            let payload = match str::from_utf8(&request.body) {
                Ok(s) => s,
                Err(_) => return false,
            };

            let signer = Ed25519Signer::from_secret_key(&self.secret_key);
            let request_signer = RequestSigner::new(signer);
            let expected = request_signer.sign_request(version, request_id, timestamp, payload);

            signature == expected
        }
    }

    fn test_task_config(symbol: &str, jwt: &str, signing_key_base64: &str) -> TaskConfig {
        TaskConfig {
            id: "task-1".to_string(),
            symbol: symbol.to_string(),
            credentials: CredentialsConfig {
                jwt_token: jwt.to_string(),
                signing_key: signing_key_base64.to_string(),
            },
            risk: crate::config::RiskConfig {
                level: "conservative".to_string(),
                max_position_usd: "0".to_string(),
                price_jump_threshold_bps: 0,
            },
            sizing: crate::config::SizingConfig {
                base_qty: "0".to_string(),
                tiers: 1,
            },
        }
    }

    fn test_order_json(order_id: i64, symbol: &str) -> serde_json::Value {
        json!({
            "avail_locked": "0",
            "cl_ord_id": format!("cl-{order_id}"),
            "closed_block": 0,
            "created_at": "0",
            "created_block": 0,
            "fill_avg_price": "0",
            "fill_qty": "0",
            "id": order_id,
            "leverage": "1",
            "liq_id": 0,
            "margin": "0",
            "order_type": "limit",
            "position_id": 0,
            "price": "100",
            "qty": "1",
            "reduce_only": false,
            "remark": "",
            "side": "buy",
            "source": "test",
            "status": "open",
            "symbol": symbol,
            "time_in_force": "gtc",
            "updated_at": "0",
            "user": "user",
        })
    }

    fn test_position_json(position_id: i64, symbol: &str, qty: &str) -> serde_json::Value {
        json!({
            "bankruptcy_price": "0",
            "created_at": "0",
            "entry_price": "0",
            "entry_value": "0",
            "holding_margin": "0",
            "id": position_id,
            "initial_margin": "0",
            "leverage": "1",
            "liq_price": "0",
            "maint_margin": "0",
            "margin_asset": "USD",
            "margin_mode": "cross",
            "mark_price": "0",
            "mmr": "0",
            "position_value": "0",
            "qty": qty,
            "realized_pnl": "0",
            "status": "open",
            "symbol": symbol,
            "time": "0",
            "updated_at": "0",
            "upnl": "0",
            "user": "user",
        })
    }

    #[tokio::test]
    async fn task_startup_cancels_open_orders() {
        let server = MockServer::start().await;
        let base_url = server.uri();

        let jwt = "jwt-token";
        let secret_key = [7u8; 32];
        let signing_key_base64 = BASE64.encode(secret_key);
        let symbol = "BTC-USD";

        Mock::given(method("GET"))
            .and(path("/api/query_open_orders"))
            .and(query_param("symbol", symbol))
            .and(header("authorization", format!("Bearer {jwt}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "page_size": 2,
                "result": [
                    test_order_json(1, symbol),
                    test_order_json(2, symbol),
                ],
                "total": 2,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let signature_matcher = ValidBodySignatureMatcher { secret_key };

        Mock::given(method("POST"))
            .and(path("/api/cancel_order"))
            .and(header("authorization", format!("Bearer {jwt}")))
            .and(signature_matcher)
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "request_id": "req-cancel",
            })))
            .expect(2)
            .mount(&server)
            .await;

        let task_config = test_task_config(symbol, jwt, &signing_key_base64);
        let client = Task::build_client_with_config_and_base_urls(
            &task_config,
            ClientConfig::default(),
            &base_url,
            &base_url,
        )
        .unwrap();

        let (_tx, rx) = watch::channel(dummy_symbol_price(symbol));
        let shutdown = CancellationToken::new();
        let mut task = Task::new_with_client(task_config, client, rx, shutdown);

        task.startup_sequence().await.unwrap();
    }

    #[tokio::test]
    async fn task_shutdown_cancels_orders_and_closes_positions() {
        let server = MockServer::start().await;
        let base_url = server.uri();

        let jwt = "jwt-token";
        let secret_key = [9u8; 32];
        let signing_key_base64 = BASE64.encode(secret_key);
        let symbol = "BTC-USD";

        Mock::given(method("GET"))
            .and(path("/api/query_open_orders"))
            .and(query_param("symbol", symbol))
            .and(header("authorization", format!("Bearer {jwt}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "page_size": 1,
                "result": [test_order_json(10, symbol)],
                "total": 1,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let signature_matcher = ValidBodySignatureMatcher { secret_key };

        Mock::given(method("POST"))
            .and(path("/api/cancel_order"))
            .and(header("authorization", format!("Bearer {jwt}")))
            .and(signature_matcher.clone())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "request_id": "req-cancel",
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/query_positions"))
            .and(query_param("symbol", symbol))
            .and(header("authorization", format!("Bearer {jwt}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                test_position_json(1, symbol, "1.5"),
                test_position_json(2, symbol, "-2"),
            ])))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/new_order"))
            .and(header("authorization", format!("Bearer {jwt}")))
            .and(signature_matcher.clone())
            .and(body_json(json!({
                "symbol": symbol,
                "side": "sell",
                "order_type": "market",
                "qty": "1.5",
                "time_in_force": "ioc",
                "reduce_only": true,
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "request_id": "req-close-1",
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/new_order"))
            .and(header("authorization", format!("Bearer {jwt}")))
            .and(signature_matcher)
            .and(body_json(json!({
                "symbol": symbol,
                "side": "buy",
                "order_type": "market",
                "qty": "2",
                "time_in_force": "ioc",
                "reduce_only": true,
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "code": 0,
                "message": "ok",
                "request_id": "req-close-2",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let task_config = test_task_config(symbol, jwt, &signing_key_base64);
        let client = Task::build_client_with_config_and_base_urls(
            &task_config,
            ClientConfig::default(),
            &base_url,
            &base_url,
        )
        .unwrap();

        let (_tx, rx) = watch::channel(dummy_symbol_price(symbol));
        let shutdown = CancellationToken::new();
        let mut task = Task::new_with_client(task_config, client, rx, shutdown);

        task.shutdown_sequence().await.unwrap();
    }

    #[tokio::test]
    async fn task_manager_spawns_and_shutdowns_tasks() {
        let server = MockServer::start().await;
        let base_url = server.uri();

        let jwt = "jwt-token";
        let secret_key = [1u8; 32];
        let signing_key_base64 = BASE64.encode(secret_key);
        let symbol = "BTC-USD";

        // One call during startup + one call during shutdown.
        Mock::given(method("GET"))
            .and(path("/api/query_open_orders"))
            .and(query_param("symbol", symbol))
            .and(header("authorization", format!("Bearer {jwt}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "page_size": 0,
                "result": [],
                "total": 0,
            })))
            .expect(2)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/query_positions"))
            .and(query_param("symbol", symbol))
            .and(header("authorization", format!("Bearer {jwt}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .expect(1)
            .mount(&server)
            .await;

        let task_config = test_task_config(symbol, jwt, &signing_key_base64);
        let strategy_config = StrategyConfig {
            tasks: vec![task_config.clone()],
        };

        let mut manager = TaskManager::new();
        manager
            .spawn_from_config_with_client_builder(strategy_config, |cfg| {
                Task::build_client_with_config_and_base_urls(
                    cfg,
                    ClientConfig::default(),
                    &base_url,
                    &base_url,
                )
            })
            .await
            .unwrap();

        manager.shutdown_and_wait().await.unwrap();
    }
}
