/*
[INPUT]:  TaskConfig + StandxClient (per task), watch::Receiver<SymbolPrice>, CancellationToken
[OUTPUT]: Tokio tasks running lifecycle (startup -> run -> shutdown) with best-effort cleanup
[POS]:    Execution layer - per-task trading orchestration
[UPDATE]: When changing startup/shutdown guarantees or supervision semantics
[UPDATE]: 2026-02-05 Add per-task stop capability keyed by task_id
[UPDATE]: 2026-02-06 Delegate quoting to MarketMakingStrategy run loop
[UPDATE]: 2026-02-06 Log account snapshot and open orders on startup
[UPDATE]: 2026-02-07 Cache symbol info in .standx-config for startup use
[UPDATE]: 2026-02-07 Move symbol cache to TaskManager scope
[UPDATE]: 2026-02-07 Wire price tick constraints to strategy
[UPDATE]: 2026-02-07 Guard positions with fee-aware limit exits
*/

use crate::config::{AccountConfig, StrategyConfig, TaskConfig};
use crate::market_data::MarketDataHub;
use crate::order_state::OrderTracker;
use crate::strategy::{MarketMakingStrategy, RiskLevel, StrategyMode};
use anyhow::{Context as _, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use standx_point_adapter::{
    Balance, CancelOrderRequest, ClientConfig, Credentials, Ed25519Signer, NewOrderRequest,
    Order, OrderStatus, OrderType, PaginatedOrders, Position, Side, StandxClient, StandxError,
    SymbolPrice, SymbolInfo, TimeInForce, WebSocketMessage, StandxWebSocket,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Once};
use std::time::Duration;
use tokio::fs;
use tokio::sync::{Mutex, watch};
use tokio::task::JoinHandle;
use tokio::time::{Instant, Sleep};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);
const POSITION_GUARD_COOLDOWN: Duration = Duration::from_secs(5);
const POSITION_GUARD_RETRY_DELAY: Duration = Duration::from_secs(1);
const BPS_DENOMINATOR: i64 = 10_000;
const DEFAULT_EXIT_BPS_CONSERVATIVE: i64 = 8;
const DEFAULT_EXIT_BPS_MODERATE: i64 = 5;
const DEFAULT_EXIT_BPS_AGGRESSIVE: i64 = 3;
const DEFAULT_GUARD_BPS_CONSERVATIVE: i64 = 30;
const DEFAULT_GUARD_BPS_MODERATE: i64 = 50;
const DEFAULT_GUARD_BPS_AGGRESSIVE: i64 = 80;
const DEFAULT_FEE_BPS: i64 = 2;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskRuntimeStatus {
    Running,
    Finished,
}

#[derive(Debug)]
struct ManagedTask {
    shutdown: CancellationToken,
    handle: JoinHandle<Result<()>>,
}

#[derive(Debug, Clone)]
struct StartupSnapshot {
    positions: Vec<Position>,
    symbol_info: Option<SymbolInfo>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct SymbolCache {
    symbols: HashMap<String, SymbolInfo>,
}

/// Task manager that coordinates multiple trading tasks.
#[derive(Debug)]
pub struct TaskManager {
    tasks: HashMap<String, ManagedTask>,

    #[cfg_attr(test, allow(dead_code))]
    market_data_hub: std::sync::Arc<Mutex<MarketDataHub>>,
    symbol_cache: std::sync::Arc<Mutex<SymbolCache>>,
    shutdown: CancellationToken,

    #[cfg(test)]
    test_price_txs: Vec<watch::Sender<SymbolPrice>>,
}

impl TaskManager {
    /// Create a new task manager.
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            market_data_hub: std::sync::Arc::new(Mutex::new(MarketDataHub::new())),
            symbol_cache: std::sync::Arc::new(Mutex::new(SymbolCache::default())),
            shutdown: CancellationToken::new(),

            #[cfg(test)]
            test_price_txs: Vec::new(),
        }
    }

    pub fn with_market_data_hub(market_data_hub: std::sync::Arc<Mutex<MarketDataHub>>) -> Self {
        Self {
            tasks: HashMap::new(),
            market_data_hub,
            symbol_cache: std::sync::Arc::new(Mutex::new(SymbolCache::default())),
            shutdown: CancellationToken::new(),

            #[cfg(test)]
            test_price_txs: Vec::new(),
        }
    }

    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown.clone()
    }

    pub fn runtime_status(&self, task_id: &str) -> Option<TaskRuntimeStatus> {
        self.tasks.get(task_id).map(|task| {
            if task.handle.is_finished() {
                TaskRuntimeStatus::Finished
            } else {
                TaskRuntimeStatus::Running
            }
        })
    }

    pub fn runtime_status_snapshot(&self) -> HashMap<String, TaskRuntimeStatus> {
        self.tasks
            .iter()
            .map(|(task_id, task)| {
                let status = if task.handle.is_finished() {
                    TaskRuntimeStatus::Finished
                } else {
                    TaskRuntimeStatus::Running
                };
                (task_id.clone(), status)
            })
            .collect()
    }

    /// Spawn tasks from configuration using the default StandxClient builder.
    pub async fn spawn_from_config(&mut self, config: StrategyConfig) -> Result<()> {
        self.spawn_from_config_with_client_builder(config, |task_config, account| {
            Task::build_client(task_config, account)
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
        F: Fn(&TaskConfig, &AccountConfig) -> Result<StandxClient>,
    {
        ensure_panic_hook_installed();

        let accounts_by_id: HashMap<String, AccountConfig> = config
            .accounts
            .into_iter()
            .map(|account| (account.id.clone(), account))
            .collect();

        self.load_symbol_cache_from_disk().await;

        for task_config in config.tasks {
            if self.tasks.contains_key(&task_config.id) {
                return Err(anyhow!(
                    "duplicate task_id in StrategyConfig: {}",
                    task_config.id
                ));
            }

            let account = accounts_by_id.get(&task_config.account_id).ok_or_else(|| {
                anyhow!("account_id not found for task_id={}", task_config.id)
            })?;

            let client = build_client(&task_config, account)
                .with_context(|| format!("build StandxClient for task_id={}", task_config.id))?;

            let price_rx = self.subscribe_price(&task_config.symbol).await;
            let shutdown = self.shutdown.child_token();
            let task_id = task_config.id.clone();

            let task = Task::new_with_client(
                task_config,
                client,
                account.jwt_token.clone(),
                price_rx,
                shutdown.clone(),
                self.symbol_cache.clone(),
            );
            let handle = task.spawn();
            self.tasks.insert(
                task_id,
                ManagedTask {
                    shutdown,
                    handle,
                },
            );
        }

        Ok(())
    }

    pub async fn stop_task(&mut self, task_id: &str) -> Result<()> {
        let Some(task) = self.tasks.remove(task_id) else {
            return Err(anyhow!("task_id not found: {task_id}"));
        };

        task.shutdown.cancel();

        let mut handle = task.handle;
        let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
        let sleep = sleep_until_deadline(deadline);

        tokio::select! {
            res = &mut handle => {
                match res {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(err)) => Err(err).with_context(|| format!("task_id={task_id} returned error")),
                    Err(join_err) => {
                        if join_err.is_panic() {
                            return Err(anyhow!("task panicked task_id={task_id}: {join_err}"));
                        }
                        Err(anyhow!("task join error task_id={task_id}: {join_err}"))
                    }
                }
            }
            _ = sleep => {
                handle.abort();
                Err(anyhow!("stop_task timed out after {SHUTDOWN_TIMEOUT:?} task_id={task_id}"))
            }
        }
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
        let mut tasks: Vec<(String, ManagedTask)> =
            std::mem::take(&mut self.tasks).into_iter().collect();

        while let Some((task_id, task)) = tasks.pop() {
            let mut handle = task.handle;
            let sleep = sleep_until_deadline(deadline);

            tokio::select! {
                res = &mut handle => {
                    match res {
                        Ok(Ok(())) => {}
                        Ok(Err(err)) => {
                            self.shutdown.cancel();
                            abort_all(tasks);
                            return Err(err).with_context(|| format!("task returned error task_id={task_id}"));
                        }
                        Err(join_err) => {
                            self.shutdown.cancel();
                            abort_all(tasks);
                            if join_err.is_panic() {
                                return Err(anyhow!("task panicked task_id={task_id}: {join_err}"));
                            }
                            return Err(anyhow!("task join error task_id={task_id}: {join_err}"));
                        }
                    }
                }
                _ = sleep => {
                    handle.abort();
                    abort_all(tasks);
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

    async fn load_symbol_cache_from_disk(&self) {
        if let Some(cache) = load_symbol_cache().await {
            let mut guard = self.symbol_cache.lock().await;
            *guard = cache;
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
    account_jwt: String,
    price_rx: watch::Receiver<SymbolPrice>,
    state: TaskState,
    shutdown: CancellationToken,
    symbol_cache: std::sync::Arc<Mutex<SymbolCache>>,
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
            account_jwt: String::new(),
            price_rx: rx,
            state: TaskState::Init,
            shutdown: CancellationToken::new(),
            symbol_cache: std::sync::Arc::new(Mutex::new(SymbolCache::default())),
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

    fn new_with_client(
        config: TaskConfig,
        client: StandxClient,
        account_jwt: String,
        price_rx: watch::Receiver<SymbolPrice>,
        shutdown: CancellationToken,
        symbol_cache: std::sync::Arc<Mutex<SymbolCache>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            config,
            client,
            account_jwt,
            price_rx,
            state: TaskState::Init,
            shutdown,
            symbol_cache,
        }
    }

    pub fn try_from_config(
        config: TaskConfig,
        account: &AccountConfig,
        price_rx: watch::Receiver<SymbolPrice>,
        shutdown: CancellationToken,
    ) -> Result<Self> {
        let client = Self::build_client(&config, account)?;
        Ok(Self::new_with_client(
            config,
            client,
            account.jwt_token.clone(),
            price_rx,
            shutdown,
            std::sync::Arc::new(Mutex::new(SymbolCache::default())),
        ))
    }

    pub fn build_client(config: &TaskConfig, account: &AccountConfig) -> Result<StandxClient> {
        Self::build_client_with_config_and_base_urls(
            config,
            account,
            ClientConfig::default(),
            "https://api.standx.com",
            "https://perps.standx.com",
        )
    }

    pub fn build_client_with_config_and_base_urls(
        _config: &TaskConfig,
        account: &AccountConfig,
        client_config: ClientConfig,
        auth_base_url: &str,
        trading_base_url: &str,
    ) -> Result<StandxClient> {
        let mut client =
            StandxClient::with_config_and_base_urls(client_config, auth_base_url, trading_base_url)
                .map_err(|err| anyhow!("create StandxClient failed: {err}"))?;

        let secret_key = decode_ed25519_secret_key_base64(&account.signing_key)
            .context("decode signing_key (base64) failed")?;
        let signer = Ed25519Signer::from_secret_key(&secret_key);

        // NOTE: wallet_address/chain are not required for trading endpoints today.
        // Keep placeholders until StrategyConfig carries these fields.
        client.set_credentials_and_signer(
            Credentials {
                jwt_token: account.jwt_token.clone(),
                wallet_address: "unknown".to_string(),
                chain: account.chain,
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

        let snapshot = match self.startup_sequence().await {
            Ok(snapshot) => snapshot,
            Err(err) => {
                self.state = TaskState::Failed;
                tracing::error!(
                    task_uuid = %self.id,
                    task_id = %self.config.id,
                    symbol = %self.config.symbol,
                    error = %err,
                    "startup sequence failed"
                );
                return Err(err).context("startup sequence failed");
            }
        };

        let risk_level = self
            .config
            .risk
            .level
            .parse::<RiskLevel>()
            .map_err(|_| anyhow!("invalid risk level: {}", self.config.risk.level))?;
        let budget_usd = Decimal::from_str(&self.config.risk.budget_usd)
            .with_context(|| format!("parse risk.budget_usd task_id={}", self.config.id))?;
        let tier_count = MarketMakingStrategy::tier_count_for_risk(risk_level);
        let initial_position_qty = snapshot
            .positions
            .iter()
            .fold(Decimal::ZERO, |acc, position| acc + position.qty);
        let order_tracker = Arc::new(Mutex::new(OrderTracker::new()));
        let mode = StrategyMode::aggressive_for_risk(risk_level);
        let mut strategy = MarketMakingStrategy::new_with_params(
            self.config.symbol.clone(),
            budget_usd,
            risk_level,
            self.price_rx.clone(),
            order_tracker,
            mode,
            tier_count,
            initial_position_qty,
        );

        if let Some(info) = snapshot.symbol_info.as_ref() {
            strategy.set_symbol_constraints(
                Some(info.price_tick_decimals),
                Some(info.qty_tick_decimals),
                Some(info.min_order_qty),
                Some(info.max_order_qty),
            );
            tracing::info!(
                task_uuid = %self.id,
                task_id = %self.config.id,
                symbol = %self.config.symbol,
                qty_tick_decimals = info.qty_tick_decimals,
                min_order_qty = %info.min_order_qty,
                max_order_qty = %info.max_order_qty,
                "symbol constraints loaded"
            );
        } else {
            tracing::warn!(
                task_uuid = %self.id,
                task_id = %self.config.id,
                symbol = %self.config.symbol,
                "symbol constraints unavailable; quantities will not be tick-aligned"
            );
        }

        self.state = TaskState::Running;
        tracing::info!(
            task_uuid = %self.id,
            task_id = %self.config.id,
            symbol = %self.config.symbol,
            "task running"
        );

        let guard_shutdown = self.shutdown.child_token();
        let client = &self.client;
        let id = self.id;
        let task_id = &self.config.id;
        let account_jwt = &self.account_jwt;
        let symbol = &self.config.symbol;
        let price_rx = self.price_rx.clone();
        let symbol_cache = self.symbol_cache.clone();
        let guard_future = Self::position_guard_ws_loop(
            client,
            id,
            task_id,
            account_jwt,
            symbol,
            price_rx,
            symbol_cache,
            risk_level,
            guard_shutdown.clone(),
        );
        let strategy_future = strategy.run(&self.client, self.shutdown.clone());
        tokio::pin!(guard_future);
        tokio::pin!(strategy_future);

        let strategy_result = tokio::select! {
            res = &mut strategy_future => {
                guard_shutdown.cancel();
                if let Err(err) = guard_future.await {
                    tracing::warn!(
                        task_uuid = %self.id,
                        task_id = %self.config.id,
                        symbol = %self.config.symbol,
                        "position guard exited with error: {err}"
                    );
                }
                res
            }
            res = &mut guard_future => {
                if let Err(err) = res {
                    tracing::warn!(
                        task_uuid = %self.id,
                        task_id = %self.config.id,
                        symbol = %self.config.symbol,
                        "position guard exited with error: {err}"
                    );
                }
                strategy_future.await
            }
        };
        if let Err(err) = &strategy_result {
            tracing::error!(
                task_uuid = %self.id,
                task_id = %self.config.id,
                symbol = %self.config.symbol,
                "strategy run failed: {err}"
            );
        }

        self.state = TaskState::Stopping;
        tracing::info!(
            task_uuid = %self.id,
            task_id = %self.config.id,
            symbol = %self.config.symbol,
            "task stopping"
        );

        let shutdown_res = self.shutdown_sequence().await;
        self.state = if strategy_result.is_ok() && shutdown_res.is_ok() {
            TaskState::Stopped
        } else {
            TaskState::Failed
        };

        match (strategy_result, shutdown_res) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(err), Ok(())) => Err(err).context("strategy run failed"),
            (Ok(()), Err(err)) => Err(err),
            (Err(err), Err(shutdown_err)) => Err(err).context(format!(
                "strategy run failed; shutdown error: {shutdown_err}"
            )),
        }
    }

    async fn startup_sequence(&mut self) -> Result<StartupSnapshot> {
        // Startup sequence: snapshot -> query -> cancel -> trade.
        let snapshot = self.log_startup_snapshot().await?;
        let orders = self.query_all_open_orders().await?;
        self.log_open_orders(&orders);
        self.cancel_orders(&orders).await?;
        Ok(snapshot)
    }

    async fn shutdown_sequence(&self) -> Result<()> {
        // Shutdown sequence: cancel open orders -> close positions.
        // This is best-effort and should remain minimal.
        self.cancel_open_orders().await?;
        self.close_positions().await?;
        Ok(())
    }

    async fn cancel_open_orders(&self) -> Result<()> {
        let orders = self.query_all_open_orders().await?;
        self.cancel_orders(&orders).await
    }

    async fn query_all_open_orders(&self) -> Result<PaginatedOrders> {
        let symbol = self.config.symbol.as_str();
        let open_orders = match self.query_open_orders().await {
            Ok(orders) => orders,
            Err(err) => {
                tracing::warn!(
                    task_uuid = %self.id,
                    task_id = %self.config.id,
                    symbol = %symbol,
                    "query_open_orders failed; falling back to query_orders: {err}"
                );

                let fallback = self
                    .client
                    .query_orders(Some(symbol), Some(OrderStatus::Open), None)
                    .await;

                return match fallback {
                    Ok(orders) => Ok(orders),
                    Err(fallback_err) => Err(anyhow!(err)).context(format!(
                        "query_open_orders failed; query_orders fallback failed: {fallback_err}"
                    )),
                };
            }
        };

        if open_orders.total > open_orders.result.len() as u32 {
            let limit = open_orders.total;
            match self
                .client
                .query_orders(Some(symbol), Some(OrderStatus::Open), Some(limit))
                .await
            {
                Ok(expanded) => return Ok(expanded),
                Err(err) => {
                    tracing::warn!(
                        task_uuid = %self.id,
                        task_id = %self.config.id,
                        symbol = %symbol,
                        total = open_orders.total,
                        page_size = open_orders.page_size,
                        "query_orders failed while expanding open orders: {err}"
                    );
                }
            }
        }

        Ok(open_orders)
    }

    async fn query_open_orders(&self) -> Result<PaginatedOrders> {
        let symbol = self.config.symbol.as_str();
        match self.client.query_open_orders(Some(symbol)).await {
            Ok(orders) => Ok(orders),
            Err(StandxError::Api { code: 404, message }) => {
                tracing::warn!(
                    task_uuid = %self.id,
                    task_id = %self.config.id,
                    symbol = %symbol,
                    "query_open_orders returned 404; treating as no open orders: {message}"
                );
                Ok(PaginatedOrders {
                    page_size: 0,
                    result: Vec::new(),
                    total: 0,
                })
            }
            Err(err) => Err(anyhow!(err)).context("query_open_orders failed"),
        }
    }

    async fn cancel_orders(&self, orders: &PaginatedOrders) -> Result<()> {
        let symbol = self.config.symbol.as_str();

        let mut first_error: Option<anyhow::Error> = None;

        for order in &orders.result {
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

    async fn log_startup_snapshot(&self) -> Result<StartupSnapshot> {
        let task_id = self.config.id.as_str();
        let symbol = self.config.symbol.as_str();

        match self.client.query_balance().await {
            Ok(balance) => {
                self.log_balance(task_id, symbol, &balance);
            }
            Err(StandxError::Api { code: 404, message }) => {
                return Err(anyhow!(
                    "account balance not found; please activate/fund your StandX account: {message}"
                ));
            }
            Err(err) => {
                tracing::warn!(
                    task_uuid = %self.id,
                    task_id = %task_id,
                    symbol = %symbol,
                    "query_balance failed during startup snapshot: {err}"
                );
            }
        };

        let positions = match self.client.query_positions(Some(symbol)).await {
            Ok(positions) => {
                self.log_positions(task_id, symbol, &positions);
                positions
            }
            Err(err) => {
                tracing::warn!(
                    task_uuid = %self.id,
                    task_id = %task_id,
                    symbol = %symbol,
                    "query_positions failed during startup snapshot: {err}"
                );
                Vec::new()
            }
        };

        let cached_symbol = {
            let cache = self.symbol_cache.lock().await;
            cache.symbols.get(symbol).cloned()
        };

        let symbol_info = match self.client.query_symbol_info(symbol).await {
            Ok(infos) => {
                let selected = select_symbol_info(infos, symbol).or_else(|| cached_symbol.clone());
                if let Some(info) = selected.as_ref() {
                    let updated_snapshot = {
                        let mut cache = self.symbol_cache.lock().await;
                        cache.symbols.insert(info.symbol.clone(), info.clone());
                        cache.clone()
                    };
                    if let Err(err) = save_symbol_cache(&updated_snapshot).await {
                        tracing::warn!(
                            task_uuid = %self.id,
                            task_id = %task_id,
                            symbol = %symbol,
                            "save_symbol_cache failed: {err}"
                        );
                    }
                }
                selected
            }
            Err(err) => {
                tracing::warn!(
                    task_uuid = %self.id,
                    task_id = %task_id,
                    symbol = %symbol,
                    "query_symbol_info failed during startup snapshot: {err}"
                );
                cached_symbol
            }
        };

        Ok(StartupSnapshot {
            positions,
            symbol_info,
        })
    }

    fn log_balance(&self, task_id: &str, symbol: &str, balance: &Balance) {
        tracing::info!(
            task_uuid = %self.id,
            task_id = %task_id,
            symbol = %symbol,
            equity = %balance.equity,
            balance = %balance.balance,
            cross_available = %balance.cross_available,
            upnl = %balance.upnl,
            locked = %balance.locked,
            "startup account balance"
        );
    }

    fn log_positions(&self, task_id: &str, symbol: &str, positions: &[Position]) {
        if positions.is_empty() {
            tracing::info!(
                task_uuid = %self.id,
                task_id = %task_id,
                symbol = %symbol,
                "startup positions: none"
            );
            return;
        }

        tracing::info!(
            task_uuid = %self.id,
            task_id = %task_id,
            symbol = %symbol,
            position_count = positions.len(),
            "startup positions"
        );

        for position in positions {
            tracing::info!(
                task_uuid = %self.id,
                task_id = %task_id,
                symbol = %position.symbol,
                qty = %position.qty,
                entry_price = %position.entry_price,
                mark_price = %position.mark_price,
                leverage = %position.leverage,
                upnl = %position.upnl,
                "startup position detail"
            );
        }
    }

    fn log_open_orders(&self, orders: &PaginatedOrders) {
        let task_id = self.config.id.as_str();
        let symbol = self.config.symbol.as_str();

        if orders.result.is_empty() {
            tracing::info!(
                task_uuid = %self.id,
                task_id = %task_id,
                symbol = %symbol,
                "startup open orders: none"
            );
            return;
        }

        tracing::info!(
            task_uuid = %self.id,
            task_id = %task_id,
            symbol = %symbol,
            total = orders.total,
            "startup open orders"
        );

        for order in &orders.result {
            self.log_open_order_detail(task_id, order);
        }
    }

    fn log_open_order_detail(&self, task_id: &str, order: &Order) {
        tracing::info!(
            task_uuid = %self.id,
            task_id = %task_id,
            symbol = %order.symbol,
            order_id = order.id,
            side = ?order.side,
            qty = %order.qty,
            price = ?order.price,
            order_type = ?order.order_type,
            status = ?order.status,
            time_in_force = ?order.time_in_force,
            "startup open order detail"
        );
    }

    async fn close_positions(&self) -> Result<()> {
        let symbol = self.config.symbol.as_str();
        let positions = self
            .client
            .query_positions(Some(symbol))
            .await
            .context("query_positions failed")?;

        self.close_positions_with_snapshot(positions).await
    }

    async fn close_positions_with_snapshot(&self, positions: Vec<Position>) -> Result<()> {
        let mut first_error: Option<anyhow::Error> = None;

        for position in positions {
            if position.qty.is_zero() {
                continue;
            }
            if let Err(err) = Self::close_position_qty(&self.client, self.id, &self.config.id, &position.symbol, position.qty).await {
                if first_error.is_none() {
                    first_error = Some(err);
                }
            }
        }

        if let Some(err) = first_error {
            return Err(err).context("one or more position closes failed");
        }

        Ok(())
    }

    async fn close_position_qty(client: &StandxClient, task_uuid: Uuid, task_id: &str, symbol: &str, qty: Decimal) -> Result<()> {
        if qty.is_zero() {
            return Ok(());
        }

        let (side, qty) = if qty.is_sign_positive() {
            (Side::Sell, qty)
        } else {
            (Side::Buy, qty.abs())
        };

        let req = NewOrderRequest {
            symbol: symbol.to_string(),
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

        match client.new_order(req).await {
            Ok(resp) if resp.code == 0 => Ok(()),
            Ok(resp) => {
                let err = anyhow!(
                    "new_order returned code={} message={}",
                    resp.code,
                    resp.message
                );
                tracing::warn!(
                    task_uuid = %task_uuid,
                    task_id = %task_id,
                    symbol = %symbol,
                    "close position failed: {err}"
                );
                Err(err)
            }
            Err(err) => {
                tracing::warn!(
                    task_uuid = %task_uuid,
                    task_id = %task_id,
                    symbol = %symbol,
                    "close position HTTP failed: {err}"
                );
                Err(anyhow!(err))
            }
        }
    }

    async fn place_guard_order(
        client: &StandxClient,
        task_uuid: Uuid,
        task_id: &str,
        symbol: &str,
        side: Side,
        qty: Decimal,
        price: Decimal,
    ) -> Option<GuardOrder> {
        if qty <= Decimal::ZERO || price <= Decimal::ZERO {
            return None;
        }

        let side_label = match side {
            Side::Buy => "buy",
            Side::Sell => "sell",
        };
        let cl_ord_id = format!("pg:{}:{}:{}", symbol, side_label, Uuid::new_v4());
        let req = NewOrderRequest {
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Limit,
            qty,
            time_in_force: TimeInForce::PostOnly,
            reduce_only: true,
            price: Some(price),
            cl_ord_id: Some(cl_ord_id.clone()),
            margin_mode: None,
            leverage: None,
            tp_price: None,
            sl_price: None,
        };

        match client.new_order(req.clone()).await {
            Ok(resp) if resp.code == 0 => {
                tracing::info!(
                    task_uuid = %task_uuid,
                    task_id = %task_id,
                    symbol = %symbol,
                    side = %side_label,
                    %price,
                    %qty,
                    request_id = %resp.request_id,
                    "position guard placed reduce-only limit"
                );
                let mut found = false;
                match client.query_open_orders(Some(symbol)).await {
                    Ok(orders) => {
                        found = orders
                            .result
                            .iter()
                            .any(|order| order.cl_ord_id == cl_ord_id);
                        tracing::info!(
                            task_uuid = %task_uuid,
                            task_id = %task_id,
                            symbol = %symbol,
                            side = %side_label,
                            cl_ord_id = %cl_ord_id,
                            open_orders = orders.result.len(),
                            found,
                            "position guard open orders check"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(
                            task_uuid = %task_uuid,
                            task_id = %task_id,
                            symbol = %symbol,
                            side = %side_label,
                            cl_ord_id = %cl_ord_id,
                            "position guard query_open_orders failed: {err}"
                        );
                    }
                }
                if !found {
                    tracing::warn!(
                        task_uuid = %task_uuid,
                        task_id = %task_id,
                        symbol = %symbol,
                        side = %side_label,
                        cl_ord_id = %cl_ord_id,
                        "position guard not visible in open orders; retrying after delay"
                    );
                    tokio::time::sleep(POSITION_GUARD_RETRY_DELAY).await;
                    match client.query_open_orders(Some(symbol)).await {
                        Ok(orders) => {
                            found = orders
                                .result
                                .iter()
                                .any(|order| order.cl_ord_id == cl_ord_id);
                            tracing::info!(
                                task_uuid = %task_uuid,
                                task_id = %task_id,
                                symbol = %symbol,
                                side = %side_label,
                                cl_ord_id = %cl_ord_id,
                                open_orders = orders.result.len(),
                                found,
                                "position guard open orders recheck"
                            );
                        }
                        Err(err) => {
                            tracing::warn!(
                                task_uuid = %task_uuid,
                                task_id = %task_id,
                                symbol = %symbol,
                                side = %side_label,
                                cl_ord_id = %cl_ord_id,
                                "position guard query_open_orders retry failed: {err}"
                            );
                        }
                    }
                    if !found {
                        match client.new_order(req).await {
                            Ok(resp) if resp.code == 0 => {
                                tracing::info!(
                                    task_uuid = %task_uuid,
                                    task_id = %task_id,
                                    symbol = %symbol,
                                    side = %side_label,
                                    %price,
                                    %qty,
                                    request_id = %resp.request_id,
                                    "position guard retry accepted"
                                );
                            }
                            Ok(resp) => {
                                tracing::warn!(
                                    task_uuid = %task_uuid,
                                    task_id = %task_id,
                                    symbol = %symbol,
                                    side = %side_label,
                                    %price,
                                    %qty,
                                    code = resp.code,
                                    message = %resp.message,
                                    "position guard retry returned non-zero code"
                                );
                            }
                            Err(err) => {
                                tracing::warn!(
                                    task_uuid = %task_uuid,
                                    task_id = %task_id,
                                    symbol = %symbol,
                                    side = %side_label,
                                    %price,
                                    %qty,
                                    "position guard retry http failed: {err}"
                                );
                            }
                        }
                    }
                }
                Some(GuardOrder {
                    cl_ord_id,
                    price,
                    qty,
                    side,
                })
            }
            Ok(resp) => {
                tracing::warn!(
                    task_uuid = %task_uuid,
                    task_id = %task_id,
                    symbol = %symbol,
                    side = %side_label,
                    %price,
                    %qty,
                    code = resp.code,
                    message = %resp.message,
                    "position guard new_order returned non-zero code"
                );
                None
            }
            Err(err) => {
                tracing::warn!(
                    task_uuid = %task_uuid,
                    task_id = %task_id,
                    symbol = %symbol,
                    side = %side_label,
                    %price,
                    %qty,
                    "position guard new_order http failed: {err}"
                );
                None
            }
        }
    }

    async fn cancel_guard_order(
        client: &StandxClient,
        task_uuid: Uuid,
        task_id: &str,
        cl_ord_id: &str,
    ) {
        let req = CancelOrderRequest {
            order_id: None,
            cl_ord_id: Some(cl_ord_id.to_string()),
        };

        match client.cancel_order(req).await {
            Ok(resp) if resp.code == 0 => {
                tracing::info!(
                    task_uuid = %task_uuid,
                    task_id = %task_id,
                    cl_ord_id = %cl_ord_id,
                    "position guard cancel_order ok"
                );
            }
            Ok(resp) => {
                tracing::warn!(
                    task_uuid = %task_uuid,
                    task_id = %task_id,
                    cl_ord_id = %cl_ord_id,
                    code = resp.code,
                    message = %resp.message,
                    "position guard cancel_order returned non-zero code"
                );
            }
            Err(err) => {
                tracing::warn!(
                    task_uuid = %task_uuid,
                    task_id = %task_id,
                    cl_ord_id = %cl_ord_id,
                    "position guard cancel_order http failed: {err}"
                );
            }
        }
    }

    async fn position_guard_ws_loop(
        client: &StandxClient,
        task_uuid: Uuid,
        task_id: &str,
        account_jwt: &str,
        task_symbol: &str,
        mut price_rx: watch::Receiver<SymbolPrice>,
        symbol_cache: Arc<Mutex<SymbolCache>>,
        risk_level: RiskLevel,
        shutdown: CancellationToken,
    ) -> Result<()> {
        if account_jwt.trim().is_empty() {
            tracing::warn!(
                task_uuid = %task_uuid,
                task_id = %task_id,
                "position guard disabled: missing account jwt"
            );
            return Ok(());
        }

        let mut ws = StandxWebSocket::new();
        if let Err(err) = ws.connect_market_stream().await {
            tracing::warn!(
                task_uuid = %task_uuid,
                task_id = %task_id,
                "position guard ws connect failed: {err}"
            );
            return Ok(());
        }

        let streams = ["position"];
        if let Err(err) = ws.authenticate(account_jwt, Some(&streams)).await {
            tracing::warn!(
                task_uuid = %task_uuid,
                task_id = %task_id,
                "position guard ws auth failed: {err}"
            );
            return Ok(());
        }

        if let Err(err) = ws.subscribe_positions().await {
            tracing::warn!(
                task_uuid = %task_uuid,
                task_id = %task_id,
                "position guard ws subscribe failed: {err}"
            );
            return Ok(());
        }

        let mut rx = ws
            .take_receiver()
            .ok_or_else(|| anyhow!("position guard ws receiver already taken"))?;

        let mut guard_state = PositionGuardState::default();

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    if let Some(order) = guard_state.guard_order.take() {
                        Self::cancel_guard_order(client, task_uuid, task_id, &order.cl_ord_id).await;
                    }
                    return Ok(());
                }
                msg = rx.recv() => {
                    let Some(message) = msg else {
                        return Ok(());
                    };

                    let WebSocketMessage::Position { data } = message else {
                        continue;
                    };

                    let updates = parse_ws_positions(&data);
                    if updates.is_empty() {
                        continue;
                    }

                    for update in updates {
                        if update.symbol != task_symbol {
                            continue;
                        }

                        if update.qty.is_zero() {
                            guard_state.position_qty = Decimal::ZERO;
                            if let Some(order) = guard_state.guard_order.take() {
                                Self::cancel_guard_order(client, task_uuid, task_id, &order.cl_ord_id).await;
                            }
                            continue;
                        }

                        guard_state.position_qty = update.qty;
                        let mark_price = price_rx.borrow().mark_price;
                        let symbol_info = {
                            let cache = symbol_cache.lock().await;
                            cache.symbols.get(task_symbol).cloned()
                        };
                        let policy = exit_guard_policy_for_risk(risk_level, symbol_info.as_ref());

                        if let Some(last_close) = guard_state.last_force_close {
                            if last_close.elapsed() < POSITION_GUARD_COOLDOWN {
                                continue;
                            }
                        }

                        let Some((side, price)) = exit_price_for_position(
                            mark_price,
                            update.qty,
                            policy,
                            symbol_info.as_ref(),
                        ) else {
                            tracing::warn!(
                                task_uuid = %task_uuid,
                                task_id = %task_id,
                                symbol = %update.symbol,
                                qty = %update.qty,
                                "position guard skipped: invalid mark price or exit price"
                            );
                            continue;
                        };

                        let qty = update.qty.abs();
                        if let Some(existing) = guard_state.guard_order.as_ref() {
                            if existing.side == side && existing.price == price && existing.qty == qty {
                                continue;
                            }
                        }

                        if let Some(order) = guard_state.guard_order.take() {
                            Self::cancel_guard_order(client, task_uuid, task_id, &order.cl_ord_id).await;
                        }

                        if let Some(order) = Self::place_guard_order(
                            client,
                            task_uuid,
                            task_id,
                            task_symbol,
                            side,
                            qty,
                            price,
                        ).await {
                            guard_state.guard_order = Some(order);
                        }
                    }
                }
                changed = price_rx.changed() => {
                    if changed.is_err() {
                        continue;
                    }

                    if guard_state.position_qty.is_zero() {
                        continue;
                    }

                    let mark_price = price_rx.borrow().mark_price;
                    let symbol_info = {
                        let cache = symbol_cache.lock().await;
                        cache.symbols.get(task_symbol).cloned()
                    };
                    let policy = exit_guard_policy_for_risk(risk_level, symbol_info.as_ref());

                    if let Some(last_close) = guard_state.last_force_close {
                        if last_close.elapsed() < POSITION_GUARD_COOLDOWN {
                            continue;
                        }
                    }

                    if guard_state.guard_order.is_none() {
                        let Some((side, price)) = exit_price_for_position(
                            mark_price,
                            guard_state.position_qty,
                            policy,
                            symbol_info.as_ref(),
                        ) else {
                            continue;
                        };

                        let qty = guard_state.position_qty.abs();
                        if let Some(order) = Self::place_guard_order(
                            client,
                            task_uuid,
                            task_id,
                            task_symbol,
                            side,
                            qty,
                            price,
                        ).await {
                            guard_state.guard_order = Some(order);
                        }
                        continue;
                    }

                    let Some(order) = guard_state.guard_order.as_ref() else {
                        continue;
                    };

                    if guard_exceeds_deviation_bps(mark_price, order.price, order.side, policy) {
                        tracing::warn!(
                            task_uuid = %task_uuid,
                            task_id = %task_id,
                            symbol = %task_symbol,
                            mark_price = %mark_price,
                            order_price = %order.price,
                            guard_bps = %policy.guard_bps,
                            "position guard deviation exceeded; forcing market close"
                        );

                        let order = guard_state.guard_order.take();
                        if let Some(order) = order {
                            Self::cancel_guard_order(client, task_uuid, task_id, &order.cl_ord_id).await;
                        }

                        guard_state.last_force_close = Some(Instant::now());

                        if let Err(err) = Self::close_position_qty(
                            client,
                            task_uuid,
                            task_id,
                            task_symbol,
                            guard_state.position_qty,
                        ).await {
                            tracing::warn!(
                                task_uuid = %task_uuid,
                                task_id = %task_id,
                                symbol = %task_symbol,
                                "position guard market close failed: {err}"
                            );
                        }
                    }
                }
            }
        }
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
        account_id: "account-1".to_string(),
        risk: crate::config::RiskConfig {
            level: "low".to_string(),
            budget_usd: "0".to_string(),
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

fn abort_all(tasks: Vec<(String, ManagedTask)>) {
    for (_task_id, task) in tasks {
        task.handle.abort();
    }
}

fn select_symbol_info(infos: Vec<SymbolInfo>, symbol: &str) -> Option<SymbolInfo> {
    if infos.is_empty() {
        return None;
    }

    infos
        .iter()
        .find(|info| info.symbol == symbol)
        .cloned()
        .or_else(|| infos.first().cloned())
}

#[derive(Debug, Clone)]
struct WsPositionUpdate {
    symbol: String,
    qty: Decimal,
}

#[derive(Debug, Clone, Copy)]
struct ExitGuardPolicy {
    exit_bps: Decimal,
    guard_bps: Decimal,
    fee_bps: Decimal,
}

#[derive(Debug, Clone)]
struct GuardOrder {
    cl_ord_id: String,
    price: Decimal,
    qty: Decimal,
    side: Side,
}

#[derive(Debug, Default)]
struct PositionGuardState {
    position_qty: Decimal,
    guard_order: Option<GuardOrder>,
    last_force_close: Option<Instant>,
}

fn parse_ws_positions(data: &serde_json::Value) -> Vec<WsPositionUpdate> {
    if let Some(inner) = data.get("data") {
        return parse_ws_positions(inner);
    }

    if let Some(positions) = data.get("positions") {
        return parse_ws_positions(positions);
    }

    match data {
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(parse_ws_position_entry)
            .collect(),
        serde_json::Value::Object(_) => parse_ws_position_entry(data).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn parse_ws_position_entry(data: &serde_json::Value) -> Option<WsPositionUpdate> {
    let symbol = data.get("symbol")?.as_str()?.to_string();
    let qty_value = data.get("qty")?;
    let qty = parse_decimal_value(qty_value)?;
    Some(WsPositionUpdate { symbol, qty })
}

fn parse_decimal_value(value: &serde_json::Value) -> Option<Decimal> {
    if let Some(raw) = value.as_str() {
        if raw.trim().is_empty() {
            return None;
        }
        return Decimal::from_str(raw).ok();
    }

    if value.is_number() {
        return Decimal::from_str(&value.to_string()).ok();
    }

    None
}

fn exit_guard_policy_for_risk(level: RiskLevel, symbol_info: Option<&SymbolInfo>) -> ExitGuardPolicy {
    let (exit_bps, guard_bps) = match level {
        RiskLevel::Low => (
            Decimal::from(DEFAULT_EXIT_BPS_CONSERVATIVE),
            Decimal::from(DEFAULT_GUARD_BPS_CONSERVATIVE),
        ),
        RiskLevel::Medium => (
            Decimal::from(DEFAULT_EXIT_BPS_MODERATE),
            Decimal::from(DEFAULT_GUARD_BPS_MODERATE),
        ),
        RiskLevel::High | RiskLevel::XHigh => (
            Decimal::from(DEFAULT_EXIT_BPS_AGGRESSIVE),
            Decimal::from(DEFAULT_GUARD_BPS_AGGRESSIVE),
        ),
    };

    let fee_bps = fee_bps_from_symbol_info(symbol_info);

    ExitGuardPolicy {
        exit_bps,
        guard_bps,
        fee_bps,
    }
}

fn fee_bps_from_symbol_info(symbol_info: Option<&SymbolInfo>) -> Decimal {
    let maker_fee_bps = symbol_info
        .map(|info| info.maker_fee * Decimal::from(BPS_DENOMINATOR))
        .unwrap_or_else(|| Decimal::from(DEFAULT_FEE_BPS));

    maker_fee_bps * Decimal::from(2)
}

fn exit_price_for_position(
    mark_price: Decimal,
    position_qty: Decimal,
    policy: ExitGuardPolicy,
    symbol_info: Option<&SymbolInfo>,
) -> Option<(Side, Decimal)> {
    if mark_price <= Decimal::ZERO || position_qty.is_zero() {
        return None;
    }

    let side = if position_qty.is_sign_positive() {
        Side::Sell
    } else {
        Side::Buy
    };

    let total_bps = policy.exit_bps + policy.fee_bps;
    if total_bps <= Decimal::ZERO {
        return None;
    }

    let desired_price = price_at_bps(mark_price, side, total_bps);
    let aligned_price = align_guard_price(desired_price, side, symbol_info);

    if aligned_price <= Decimal::ZERO {
        return None;
    }

    Some((side, aligned_price))
}

fn align_guard_price(price: Decimal, side: Side, symbol_info: Option<&SymbolInfo>) -> Decimal {
    if price <= Decimal::ZERO {
        return price;
    }

    let Some(info) = symbol_info else {
        return price;
    };

    let decimals = info.price_tick_decimals;
    let strategy = match side {
        Side::Buy => RoundingStrategy::ToNegativeInfinity,
        Side::Sell => RoundingStrategy::ToPositiveInfinity,
    };

    price.round_dp_with_strategy(decimals, strategy)
}

fn guard_exceeds_deviation_bps(
    mark_price: Decimal,
    order_price: Decimal,
    side: Side,
    policy: ExitGuardPolicy,
) -> bool {
    if mark_price <= Decimal::ZERO || order_price <= Decimal::ZERO {
        return false;
    }

    let deviation_bps = bps_from_price(mark_price, side, order_price);
    deviation_bps >= policy.guard_bps
}

fn price_at_bps(mark_price: Decimal, side: Side, bps: Decimal) -> Decimal {
    let ratio = bps / Decimal::from(BPS_DENOMINATOR);
    match side {
        Side::Buy => mark_price * (Decimal::ONE - ratio),
        Side::Sell => mark_price * (Decimal::ONE + ratio),
    }
}

fn bps_from_price(mark_price: Decimal, side: Side, price: Decimal) -> Decimal {
    if mark_price <= Decimal::ZERO || price <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    let diff = match side {
        Side::Buy => mark_price - price,
        Side::Sell => price - mark_price,
    };

    if diff <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    (diff / mark_price) * Decimal::from(BPS_DENOMINATOR)
}

fn symbol_cache_path() -> PathBuf {
    let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    base_dir.join(".standx-config").join("symbols.json")
}

async fn load_symbol_cache() -> Option<SymbolCache> {
    let path = symbol_cache_path();
    if !path.exists() {
        return None;
    }
    let content = match fs::read_to_string(&path).await {
        Ok(content) => content,
        Err(err) => {
            tracing::warn!("read symbol cache failed: {err}");
            return None;
        }
    };

    match serde_json::from_str::<SymbolCache>(&content) {
        Ok(cache) => Some(cache),
        Err(err) => {
            tracing::warn!("parse symbol cache failed: {err}");
            None
        }
    }
}

async fn save_symbol_cache(cache: &SymbolCache) -> Result<()> {
    let path = symbol_cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let payload = serde_json::to_string_pretty(cache)?;
    fs::write(path, payload).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;
    use std::sync::OnceLock;
    use tokio::sync::Mutex;
    use serde_json::json;

    // Static async lock to serialize wiremock-heavy tests and prevent flakiness
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn test_lock() -> &'static Mutex<()> {
        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn dec(value: &str) -> Decimal {
        Decimal::from_str(value).expect("valid decimal")
    }

    fn test_symbol_info(maker_fee: &str, price_tick_decimals: u32) -> SymbolInfo {
        SymbolInfo {
            base_asset: "BASE".to_string(),
            base_decimals: 6,
            created_at: "0".to_string(),
            def_leverage: dec("1"),
            depth_ticks: "0".to_string(),
            enabled: true,
            maker_fee: dec(maker_fee),
            max_leverage: dec("10"),
            max_open_orders: dec("0"),
            max_order_qty: dec("1000"),
            max_position_size: dec("100000"),
            min_order_qty: dec("0.001"),
            price_cap_ratio: dec("0"),
            price_floor_ratio: dec("0"),
            price_tick_decimals,
            qty_tick_decimals: 3,
            quote_asset: "USD".to_string(),
            quote_decimals: 2,
            symbol: "TEST".to_string(),
            taker_fee: dec(maker_fee),
            updated_at: "0".to_string(),
        }
    }

    #[test]
    fn exit_guard_policy_includes_fee_buffer() {
        let info = test_symbol_info("0.0002", 2);
        let policy = exit_guard_policy_for_risk(RiskLevel::Medium, Some(&info));
        assert_eq!(policy.exit_bps, Decimal::from(DEFAULT_EXIT_BPS_MODERATE));
        assert_eq!(policy.fee_bps, Decimal::from(4));
    }

    #[test]
    fn exit_price_for_position_applies_fee_and_rounding() {
        let info = test_symbol_info("0.0001", 2);
        let policy = exit_guard_policy_for_risk(RiskLevel::High, Some(&info));
        let mark_price = dec("100.00");

        let (side, price) =
            exit_price_for_position(mark_price, dec("1"), policy, Some(&info))
                .expect("exit price should exist");
        assert_eq!(side, Side::Sell);
        assert_eq!(price, dec("100.05"));

        let (side, price) =
            exit_price_for_position(mark_price, dec("-2"), policy, Some(&info))
                .expect("exit price should exist");
        assert_eq!(side, Side::Buy);
        assert_eq!(price, dec("99.95"));
    }

    #[test]
    fn guard_deviation_exceeds_threshold() {
        let policy = ExitGuardPolicy {
            exit_bps: Decimal::ZERO,
            guard_bps: Decimal::from(10),
            fee_bps: Decimal::ZERO,
        };

        assert!(guard_exceeds_deviation_bps(
            dec("100"),
            dec("100.11"),
            Side::Sell,
            policy,
        ));
        assert!(!guard_exceeds_deviation_bps(
            dec("100"),
            dec("100.01"),
            Side::Sell,
            policy,
        ));
    }

    #[test]
    fn parse_ws_positions_handles_objects_and_arrays() {
        let single = json!({"symbol": "XAU-USD", "qty": "1.5"});
        let updates = parse_ws_positions(&single);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].symbol, "XAU-USD");
        assert_eq!(updates[0].qty, dec("1.5"));

        let numeric = json!({"symbol": "XAU-USD", "qty": 2});
        let updates = parse_ws_positions(&numeric);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].qty, dec("2"));

        let nested = json!({"data": {"symbol": "XAU-USD", "qty": "3"}});
        let updates = parse_ws_positions(&nested);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].qty, dec("3"));

        let array = json!([
            {"symbol": "BTC-USD", "qty": "1"},
            {"symbol": "ETH-USD", "qty": "2"}
        ]);
        let updates = parse_ws_positions(&array);
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].symbol, "BTC-USD");
        assert_eq!(updates[1].symbol, "ETH-USD");

        let positions = json!({"positions": [{"symbol": "SOL-USD", "qty": "4"}]});
        let updates = parse_ws_positions(&positions);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].symbol, "SOL-USD");
    }
    use standx_point_adapter::RequestSigner;
    use standx_point_adapter::http::signature::{
        HEADER_REQUEST_ID, HEADER_REQUEST_SIGNATURE, HEADER_REQUEST_TIMESTAMP,
        HEADER_REQUEST_VERSION,
    };
    use std::str;
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

    async fn wait_for_request_count(
        server: &MockServer,
        expected: usize,
        timeout: Duration,
    ) {
        let deadline = Instant::now() + timeout;
        loop {
            let count = server.received_requests().await.unwrap_or_default().len();
            if count >= expected {
                return;
            }
            if Instant::now() >= deadline {
                panic!(
                    "timed out waiting for {expected} requests, last count={count}"
                );
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

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

    fn test_task_config_with_id(
        task_id: &str,
        symbol: &str,
        account_id: &str,
    ) -> TaskConfig {
        TaskConfig {
            id: task_id.to_string(),
            symbol: symbol.to_string(),
            account_id: account_id.to_string(),
            risk: crate::config::RiskConfig {
                level: "low".to_string(),
                budget_usd: "0".to_string(),
            },
        }
    }

    fn test_task_config(symbol: &str, account_id: &str) -> TaskConfig {
        test_task_config_with_id("task-1", symbol, account_id)
    }

    fn test_account_config(id: &str, jwt: &str, signing_key_base64: &str) -> AccountConfig {
        AccountConfig {
            id: id.to_string(),
            jwt_token: jwt.to_string(),
            signing_key: signing_key_base64.to_string(),
            chain: standx_point_adapter::Chain::Bsc,
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

    fn test_balance_json() -> serde_json::Value {
        json!({
            "isolated_balance": "0",
            "isolated_upnl": "0",
            "cross_balance": "0",
            "cross_margin": "0",
            "cross_upnl": "0",
            "locked": "0",
            "cross_available": "0",
            "balance": "0",
            "upnl": "0",
            "equity": "0",
            "pnl_freeze": "0",
        })
    }

    #[tokio::test]
    async fn task_startup_cancels_open_orders() {
        let _guard = test_lock().lock().await;
        let server = MockServer::builder().start().await;
        let base_url = server.uri();

        let jwt = "jwt-token";
        let secret_key = [7u8; 32];
        let signing_key_base64 = BASE64.encode(secret_key);
        let symbol = "BTC-USD";

        Mock::given(method("GET"))
            .and(path("/api/query_balance"))
            .and(header("authorization", format!("Bearer {jwt}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(test_balance_json()))
            .expect(1)
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

        let account = test_account_config("account-1", jwt, &signing_key_base64);
        let task_config = test_task_config(symbol, &account.id);
        let client = Task::build_client_with_config_and_base_urls(
            &task_config,
            &account,
            ClientConfig::default(),
            &base_url,
            &base_url,
        )
        .unwrap();

        let (_tx, rx) = watch::channel(dummy_symbol_price(symbol));
        let shutdown = CancellationToken::new();
        let symbol_cache = std::sync::Arc::new(Mutex::new(SymbolCache::default()));
        let mut task = Task::new_with_client(
            task_config,
            client,
            account.jwt_token.clone(),
            rx,
            shutdown,
            symbol_cache,
        );

        let _ = task.startup_sequence().await.unwrap();
    }

    #[tokio::test]
    async fn task_startup_expands_open_orders_with_query_orders() {
        let _guard = test_lock().lock().await;
        let server = MockServer::builder().start().await;
        let base_url = server.uri();

        let jwt = "jwt-token";
        let secret_key = [13u8; 32];
        let signing_key_base64 = BASE64.encode(secret_key);
        let symbol = "BTC-USD";

        Mock::given(method("GET"))
            .and(path("/api/query_balance"))
            .and(header("authorization", format!("Bearer {jwt}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(test_balance_json()))
            .expect(1)
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

        Mock::given(method("GET"))
            .and(path("/api/query_open_orders"))
            .and(query_param("symbol", symbol))
            .and(header("authorization", format!("Bearer {jwt}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "page_size": 1,
                "result": [test_order_json(1, symbol)],
                "total": 2,
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/query_orders"))
            .and(query_param("symbol", symbol))
            .and(query_param("status", "open"))
            .and(query_param("limit", "2"))
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

        let account = test_account_config("account-1", jwt, &signing_key_base64);
        let task_config = test_task_config(symbol, &account.id);
        let client = Task::build_client_with_config_and_base_urls(
            &task_config,
            &account,
            ClientConfig::default(),
            &base_url,
            &base_url,
        )
        .unwrap();

        let (_tx, rx) = watch::channel(dummy_symbol_price(symbol));
        let shutdown = CancellationToken::new();
        let symbol_cache = std::sync::Arc::new(Mutex::new(SymbolCache::default()));
        let mut task = Task::new_with_client(
            task_config,
            client,
            account.jwt_token.clone(),
            rx,
            shutdown,
            symbol_cache,
        );

        let _ = task.startup_sequence().await.unwrap();
    }

    #[tokio::test]
    async fn task_startup_treats_open_orders_404_as_empty() {
        let _guard = test_lock().lock().await;
        let server = MockServer::builder().start().await;
        let base_url = server.uri();

        let jwt = "jwt-token";
        let secret_key = [11u8; 32];
        let signing_key_base64 = BASE64.encode(secret_key);
        let symbol = "BTC-USD";

        Mock::given(method("GET"))
            .and(path("/api/query_balance"))
            .and(header("authorization", format!("Bearer {jwt}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(test_balance_json()))
            .expect(1)
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

        Mock::given(method("GET"))
            .and(path("/api/query_open_orders"))
            .and(query_param("symbol", symbol))
            .and(header("authorization", format!("Bearer {jwt}")))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "message": "no open orders found"
            })))
            .mount(&server)
            .await;

        let account = test_account_config("account-1", jwt, &signing_key_base64);
        let task_config = test_task_config(symbol, &account.id);
        let client = Task::build_client_with_config_and_base_urls(
            &task_config,
            &account,
            ClientConfig::default(),
            &base_url,
            &base_url,
        )
        .unwrap();

        let (_tx, rx) = watch::channel(dummy_symbol_price(symbol));
        let shutdown = CancellationToken::new();
        let symbol_cache = std::sync::Arc::new(Mutex::new(SymbolCache::default()));
        let mut task = Task::new_with_client(
            task_config,
            client,
            account.jwt_token.clone(),
            rx,
            shutdown,
            symbol_cache,
        );

        let _ = task.startup_sequence().await.unwrap();
    }

    #[tokio::test]
    async fn task_shutdown_cancels_orders_and_closes_positions() {
        let _guard = test_lock().lock().await;
        let server = MockServer::builder().start().await;
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

        let account = test_account_config("account-1", jwt, &signing_key_base64);
        let task_config = test_task_config(symbol, &account.id);
        let client = Task::build_client_with_config_and_base_urls(
            &task_config,
            &account,
            ClientConfig::default(),
            &base_url,
            &base_url,
        )
        .unwrap();

        let (_tx, rx) = watch::channel(dummy_symbol_price(symbol));
        let shutdown = CancellationToken::new();
        let symbol_cache = std::sync::Arc::new(Mutex::new(SymbolCache::default()));
        let task = Task::new_with_client(
            task_config,
            client,
            account.jwt_token.clone(),
            rx,
            shutdown,
            symbol_cache,
        );

        task.shutdown_sequence().await.unwrap();
    }

    #[tokio::test]
    async fn task_manager_spawns_and_shutdowns_tasks() {
        let _guard = test_lock().lock().await;
        let server = MockServer::builder().start().await;
        let base_url = server.uri();

        let jwt = "jwt-token";
        let secret_key = [1u8; 32];
        let signing_key_base64 = BASE64.encode(secret_key);
        let symbol = "BTC-USD";

        Mock::given(method("GET"))
            .and(path("/api/query_balance"))
            .and(header("authorization", format!("Bearer {jwt}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(test_balance_json()))
            .expect(1)
            .mount(&server)
            .await;

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
            .expect(2)
            .mount(&server)
            .await;

        let account = test_account_config("account-1", jwt, &signing_key_base64);
        let task_config = test_task_config(symbol, &account.id);
        let strategy_config = StrategyConfig {
            accounts: vec![account.clone()],
            tasks: vec![task_config.clone()],
        };

        let mut manager = TaskManager::new();
        let client_config = ClientConfig {
            timeout: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(30),
        };
        manager
            .spawn_from_config_with_client_builder(strategy_config, |cfg, account_cfg| {
                Task::build_client_with_config_and_base_urls(
                    cfg,
                    account_cfg,
                    client_config.clone(),
                    &base_url,
                    &base_url,
                )
            })
            .await
            .unwrap();

        wait_for_request_count(&server, 1, Duration::from_secs(5)).await;

        manager.shutdown_and_wait().await.unwrap();
        
        // Allow wiremock to finish processing all requests before checking count
        tokio::time::sleep(Duration::from_millis(1000)).await;
        
        wait_for_request_count(&server, 3, Duration::from_secs(10)).await;
    }

    #[tokio::test]
    async fn task_manager_stop_task_errors_when_missing() {
        let mut manager = TaskManager::new();
        let err = manager.stop_task("missing").await.unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn task_manager_stop_task_only_stops_selected() {
        let _guard = test_lock().lock().await;
        let server = MockServer::builder().start().await;
        let base_url = server.uri();

        let jwt = "jwt-token";
        let secret_key = [2u8; 32];
        let signing_key_base64 = BASE64.encode(secret_key);
        let symbol_1 = "BTC-USD";
        let symbol_2 = "ETH-USD";

        Mock::given(method("GET"))
            .and(path("/api/query_balance"))
            .and(header("authorization", format!("Bearer {jwt}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(test_balance_json()))
            .expect(2)
            .mount(&server)
            .await;

        // Each task: one call during startup + one call during stop/shutdown.
        for symbol in [symbol_1, symbol_2] {
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
                .expect(2)
                .mount(&server)
                .await;
        }

        let account = test_account_config("account-1", jwt, &signing_key_base64);
        let strategy_config = StrategyConfig {
            accounts: vec![account.clone()],
            tasks: vec![
                test_task_config_with_id("task-1", symbol_1, &account.id),
                test_task_config_with_id("task-2", symbol_2, &account.id),
            ],
        };

        let mut manager = TaskManager::new();
        let client_config = ClientConfig {
            timeout: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(30),
        };
        manager
            .spawn_from_config_with_client_builder(strategy_config, |cfg, account_cfg| {
                Task::build_client_with_config_and_base_urls(
                    cfg,
                    account_cfg,
                    client_config.clone(),
                    &base_url,
                    &base_url,
                )
            })
            .await
            .unwrap();

        wait_for_request_count(&server, 2, Duration::from_secs(5)).await;

        manager.stop_task("task-1").await.unwrap();
        wait_for_request_count(&server, 4, Duration::from_secs(10)).await;

        manager.shutdown_and_wait().await.unwrap();

        // Allow wiremock to finish processing all requests before checking count.
        tokio::time::sleep(Duration::from_millis(1000)).await;

        wait_for_request_count(&server, 6, Duration::from_secs(10)).await;
    }
}
