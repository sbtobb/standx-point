/*
[INPUT]:  `watch::Receiver<SymbolPrice>` (mark price), `StandxClient` for order placement,
          and `OrderTracker` updates (ack/fills/cancels via external WS reconciliation).
[OUTPUT]: PostOnly limit orders (bid+ask ladder) kept in sync with mark price,
          plus uptime accounting for reward eligibility.
[POS]:    Strategy layer - conservative market making core loop.
[UPDATE]: When changing tier ranges, sizing rules, drift thresholds, or cooldown/mode semantics.
[UPDATE]: 2026-02-06 Align fusion tiers, weights, and fill backoff handling.
[UPDATE]: 2026-02-07 Align order price/qty with tick constraints.
[UPDATE]: 2026-02-07 Replace quotes when bps exits safety band.
[UPDATE]: 2026-02-07 Budget reflects total bid+ask notional.
[UPDATE]: 2026-02-09 Gate replace on cancel ack with reconcile fallback.
*/

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use rust_decimal::{Decimal, RoundingStrategy};
use tokio::sync::{Mutex, mpsc, watch};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use standx_point_adapter::{
    CancelOrderRequest, CancelOrderResponse, NewOrderRequest, NewOrderResponse, OrderType, Side,
    StandxClient, SymbolPrice, TimeInForce,
};

use crate::order_state::{OrderState, OrderTracker};
use crate::metrics::TaskMetrics;
use crate::risk::{RiskManager, RiskState};

const BPS_DENOMINATOR: i64 = 10_000;
const QUOTE_REFRESH_INTERVAL: Duration = Duration::from_secs(5); // >=5s min resting
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

const SURVIVAL_AFTER_FILL: Duration = Duration::from_secs(60);
const FILL_BACKOFF_DURATION: Duration = Duration::from_secs(600);

// Replace quotes when desired price drifts by >= 1 bps.
const REPLACE_DRIFT_BPS: i64 = 1;
const L1_MIN_REST: Duration = Duration::from_secs(3);
const CANCEL_ACK_TIMEOUT: Duration = Duration::from_secs(10);
const CANCEL_RETRY_INTERVAL: Duration = Duration::from_secs(15);
const CANCEL_RECONCILE_COOLDOWN: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    XHigh,
}

impl std::str::FromStr for RiskLevel {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "xhigh" => Ok(Self::XHigh),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StrategyMode {
    Aggressive { target_bps: (Decimal, Decimal) },
    Survival { target_bps: (Decimal, Decimal) },
}

impl StrategyMode {
    pub fn aggressive_default() -> Self {
        Self::Aggressive {
            target_bps: (Decimal::ZERO, Decimal::from(8)),
        }
    }

    pub fn aggressive_for_risk(risk_level: RiskLevel) -> Self {
        let target_bps = match risk_level {
            RiskLevel::Low => (Decimal::from(5), Decimal::from(30)),
            RiskLevel::Medium => (Decimal::from(5), Decimal::from(15)),
            RiskLevel::High => (Decimal::from(5), Decimal::from(10)),
            RiskLevel::XHigh => (Decimal::from(5), Decimal::from(8)),
        };
        Self::Aggressive { target_bps }
    }

    pub fn survival_default() -> Self {
        Self::Survival {
            target_bps: (Decimal::from(2), Decimal::from(9)),
        }
    }

    fn target_range(&self) -> (Decimal, Decimal) {
        match *self {
            StrategyMode::Aggressive { target_bps } => target_bps,
            StrategyMode::Survival { target_bps } => target_bps,
        }
    }

    fn is_survival(&self) -> bool {
        matches!(self, StrategyMode::Survival { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Tier {
    L1,
    L2,
    L3,
    L4,
    L5,
}

impl Tier {
    fn min_max_bps(self) -> (Decimal, Decimal) {
        match self {
            Tier::L1 => (Decimal::from(5), Decimal::from(8)),
            Tier::L2 => (Decimal::from(8), Decimal::from(10)),
            Tier::L3 => (Decimal::from(10), Decimal::from(15)),
            Tier::L4 => (Decimal::from(15), Decimal::from(20)),
            Tier::L5 => (Decimal::from(20), Decimal::from(30)),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Tier::L1 => "l1",
            Tier::L2 => "l2",
            Tier::L3 => "l3",
            Tier::L4 => "l4",
            Tier::L5 => "l5",
        }
    }
}

const TIERS_L1: [Tier; 1] = [Tier::L1];
const TIERS_L1_L2: [Tier; 2] = [Tier::L1, Tier::L2];
const TIERS_L1_L2_L3: [Tier; 3] = [Tier::L1, Tier::L2, Tier::L3];
const TIERS_ALL: [Tier; 5] = [Tier::L1, Tier::L2, Tier::L3, Tier::L4, Tier::L5];

fn normalize_tier_count(tiers: u8) -> usize {
    match tiers {
        0 | 1 => 1,
        2 => 2,
        3 => 3,
        _ => 5,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum QuoteSide {
    Bid,
    Ask,
}

#[derive(Debug, Clone)]
pub enum OrderReconcileReason {
    CancelTimeout,
}

#[derive(Debug, Clone)]
pub struct OrderReconcileRequest {
    pub cl_ord_id: String,
    pub reason: OrderReconcileReason,
}

impl QuoteSide {
    fn to_order_side(self) -> Side {
        match self {
            QuoteSide::Bid => Side::Buy,
            QuoteSide::Ask => Side::Sell,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            QuoteSide::Bid => "bid",
            QuoteSide::Ask => "ask",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct QuoteSlot {
    tier: Tier,
    side: QuoteSide,
}

#[derive(Debug, Clone)]
struct PendingQuote {
    price: Decimal,
    qty: Decimal,
}

#[derive(Debug, Clone)]
struct CancelInFlight {
    sent_at: tokio::time::Instant,
    deadline: tokio::time::Instant,
    last_reconcile_at: Option<tokio::time::Instant>,
    pending: Option<PendingQuote>,
}

#[derive(Debug, Clone)]
struct LiveQuote {
    cl_ord_id: String,
    price: Decimal,
    qty: Decimal,
    placed_at: tokio::time::Instant,
    cancel_in_flight: Option<CancelInFlight>,
}

#[derive(Debug, Clone)]
pub struct UptimeSnapshot {
    pub active: bool,
    pub active_duration: Duration,
    pub total_duration: Duration,
    pub uptime_ratio: Decimal,
}

#[derive(Debug, Clone)]
pub struct UptimeTracker {
    last_update: tokio::time::Instant,
    active: bool,
    active_duration: Duration,
    inactive_duration: Duration,
}

impl UptimeTracker {
    pub fn new(now: tokio::time::Instant) -> Self {
        Self {
            last_update: now,
            active: false,
            active_duration: Duration::from_secs(0),
            inactive_duration: Duration::from_secs(0),
        }
    }

    pub fn update(&mut self, now: tokio::time::Instant, active: bool) {
        let delta = now.saturating_duration_since(self.last_update);

        if self.active {
            self.active_duration = self.active_duration.saturating_add(delta);
        } else {
            self.inactive_duration = self.inactive_duration.saturating_add(delta);
        }

        self.active = active;
        self.last_update = now;
    }

    pub fn snapshot(&self, now: tokio::time::Instant) -> UptimeSnapshot {
        let mut active_duration = self.active_duration;
        let mut inactive_duration = self.inactive_duration;

        let delta = now.saturating_duration_since(self.last_update);
        if self.active {
            active_duration = active_duration.saturating_add(delta);
        } else {
            inactive_duration = inactive_duration.saturating_add(delta);
        }

        let total = active_duration.saturating_add(inactive_duration);
        let uptime_ratio = if total.as_nanos() == 0 {
            Decimal::ZERO
        } else {
            let active_ms = Decimal::from(active_duration.as_millis() as i64);
            let total_ms = Decimal::from(total.as_millis() as i64);
            if total_ms.is_zero() {
                Decimal::ZERO
            } else {
                active_ms / total_ms
            }
        };

        UptimeSnapshot {
            active: self.active,
            active_duration,
            total_duration: total,
            uptime_ratio,
        }
    }
}

trait OrderExecutor: Send + Sync {
    fn new_order(
        &self,
        req: NewOrderRequest,
    ) -> Pin<Box<dyn Future<Output = standx_point_adapter::Result<NewOrderResponse>> + Send + '_>>;

    fn cancel_order(
        &self,
        req: CancelOrderRequest,
    ) -> Pin<Box<dyn Future<Output = standx_point_adapter::Result<CancelOrderResponse>> + Send + '_>>;
}

impl OrderExecutor for StandxClient {
    fn new_order(
        &self,
        req: NewOrderRequest,
    ) -> Pin<Box<dyn Future<Output = standx_point_adapter::Result<NewOrderResponse>> + Send + '_>>
    {
        Box::pin(async move { StandxClient::new_order(self, req).await })
    }

    fn cancel_order(
        &self,
        req: CancelOrderRequest,
    ) -> Pin<Box<dyn Future<Output = standx_point_adapter::Result<CancelOrderResponse>> + Send + '_>>
    {
        Box::pin(async move { StandxClient::cancel_order(self, req).await })
    }
}

/// Market making strategy implementation.
#[derive(Debug)]
pub struct MarketMakingStrategy {
    symbol: String,
    base_qty: Decimal,
    budget_usd: Decimal,
    tier_count: usize,
    risk_level: RiskLevel,
    price_tick_decimals: Option<u32>,
    qty_tick_decimals: Option<u32>,
    min_order_qty: Option<Decimal>,
    max_order_qty: Option<Decimal>,
    price_rx: watch::Receiver<SymbolPrice>,
    order_tracker: Arc<Mutex<OrderTracker>>,
    risk_manager: RiskManager,
    uptime_tracker: UptimeTracker,
    mode: StrategyMode,

    preferred_mode: StrategyMode,
    survival_until: Option<tokio::time::Instant>,
    bid_backoff_until: Option<tokio::time::Instant>,
    ask_backoff_until: Option<tokio::time::Instant>,
    live_quotes: HashMap<QuoteSlot, LiveQuote>,
    handled_fills: HashSet<String>,
    inventory_qty: Decimal,
    max_non_usd_value: Decimal,
    bootstrap_side: Option<QuoteSide>,
    order_reconcile_tx: mpsc::UnboundedSender<OrderReconcileRequest>,
    metrics: Option<Arc<Mutex<TaskMetrics>>>,
}

impl MarketMakingStrategy {
    /// Create a new market making strategy.
    ///
    /// Note: this uses placeholder values. Prefer `new_with_params` for real wiring.
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(initial_symbol_price(""));
        drop(tx);
        let (reconcile_tx, _reconcile_rx) = mpsc::unbounded_channel();

        let now = tokio::time::Instant::now();
        let mode = StrategyMode::aggressive_default();
        Self {
            symbol: String::new(),
            base_qty: Decimal::ZERO,
            budget_usd: Decimal::ZERO,
            tier_count: 5,
            risk_level: RiskLevel::Low,
            price_tick_decimals: None,
            qty_tick_decimals: None,
            min_order_qty: None,
            max_order_qty: None,
            price_rx: rx,
            order_tracker: Arc::new(Mutex::new(OrderTracker::new())),
            risk_manager: RiskManager::new(),
            uptime_tracker: UptimeTracker::new(now),
            mode,
            preferred_mode: mode,
            survival_until: None,
            bid_backoff_until: None,
            ask_backoff_until: None,
            live_quotes: HashMap::new(),
            handled_fills: HashSet::new(),
            inventory_qty: Decimal::ZERO,
            max_non_usd_value: Decimal::ZERO,
            bootstrap_side: None,
            order_reconcile_tx: reconcile_tx,
            metrics: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_params(
        symbol: String,
        budget_usd: Decimal,
        risk_level: RiskLevel,
        price_rx: watch::Receiver<SymbolPrice>,
        order_tracker: Arc<Mutex<OrderTracker>>,
        order_reconcile_tx: mpsc::UnboundedSender<OrderReconcileRequest>,
        mode: StrategyMode,
        tier_count: u8,
        initial_position_qty: Decimal,
    ) -> Self {
        let now = tokio::time::Instant::now();
        let bootstrap_side = None;
        let max_non_usd_value = if budget_usd <= Decimal::ZERO {
            Decimal::ZERO
        } else {
            budget_usd
        };
        Self {
            symbol,
            base_qty: Decimal::ZERO,
            budget_usd,
            tier_count: normalize_tier_count(tier_count),
            risk_level,
            price_tick_decimals: None,
            qty_tick_decimals: None,
            min_order_qty: None,
            max_order_qty: None,
            price_rx,
            order_tracker,
            risk_manager: RiskManager::new(),
            uptime_tracker: UptimeTracker::new(now),
            mode,
            preferred_mode: mode,
            survival_until: None,
            bid_backoff_until: None,
            ask_backoff_until: None,
            live_quotes: HashMap::new(),
            handled_fills: HashSet::new(),
            inventory_qty: initial_position_qty,
            max_non_usd_value,
            bootstrap_side,
            order_reconcile_tx,
            metrics: None,
        }
    }

    pub fn set_metrics(&mut self, metrics: Arc<Mutex<TaskMetrics>>) {
        self.metrics = Some(metrics);
    }

    pub(crate) fn tier_count_for_risk(risk_level: RiskLevel) -> u8 {
        match risk_level {
            RiskLevel::Low => 5,
            RiskLevel::Medium => 3,
            RiskLevel::High => 2,
            RiskLevel::XHigh => 1,
        }
    }

    pub fn set_mode(&mut self, mode: StrategyMode) {
        self.preferred_mode = mode;
        self.mode = mode;
        self.survival_until = None;
    }

    pub fn uptime_snapshot(&self) -> UptimeSnapshot {
        self.uptime_tracker.snapshot(tokio::time::Instant::now())
    }

    pub fn set_symbol_constraints(
        &mut self,
        price_tick_decimals: Option<u32>,
        qty_tick_decimals: Option<u32>,
        min_order_qty: Option<Decimal>,
        max_order_qty: Option<Decimal>,
    ) {
        self.price_tick_decimals = price_tick_decimals;
        self.qty_tick_decimals = qty_tick_decimals;
        self.min_order_qty = min_order_qty;
        self.max_order_qty = max_order_qty;
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub async fn run(&mut self, client: &StandxClient, shutdown: CancellationToken) -> Result<()> {
        self.run_with_executor(client, shutdown).await
    }

    async fn run_with_executor(
        &mut self,
        executor: &dyn OrderExecutor,
        shutdown: CancellationToken,
    ) -> Result<()> {
        let mut refresh = tokio::time::interval(QUOTE_REFRESH_INTERVAL);
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        info!(
            symbol = %self.symbol,
            risk_level = ?self.risk_level,
            tier_count = self.tier_count,
            "strategy run loop starting"
        );

        // Try to quote immediately using the current snapshot.
        self.refresh_from_latest(executor, tokio::time::Instant::now())
            .await?;

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    info!(symbol = %self.symbol, "strategy shutdown requested");
                    let now = tokio::time::Instant::now();
                    self.cancel_all_quotes(executor, now).await;
                    self.uptime_tracker.update(now, false);
                    return Ok(());
                }
                tick = refresh.tick() => {
                    self.refresh_from_latest(executor, tick).await?;
                }
                changed = self.price_rx.changed() => {
                    if changed.is_err() {
                        // Sender dropped (hub stopped). Keep ticking on interval.
                        continue;
                    }

                    let (mark_price, reference_price) = {
                        let snapshot = self.price_rx.borrow();
                        (snapshot.mark_price, self.quote_reference_price(&snapshot))
                    };
                    if let Some(metrics) = self.metrics.as_ref() {
                        let mut metrics = metrics.lock().await;
                        metrics.record_price(mark_price);
                    }
                    if self.live_quotes.is_empty() {
                        // Kick-start quoting when idle.
                        self.refresh_from_latest(executor, tokio::time::Instant::now()).await?;
                    } else if self.should_refresh_for_price(reference_price, tokio::time::Instant::now()) {
                        // Re-quote immediately when reference price drift exceeds threshold.
                        self.refresh_from_latest(executor, tokio::time::Instant::now()).await?;
                    }
                }
                _ = heartbeat.tick() => {
                    let snapshot = self.uptime_snapshot();
                    if let Some(metrics) = self.metrics.as_ref() {
                        let mut metrics = metrics.lock().await;
                        metrics.record_heartbeat();
                    }
                    debug!(
                        symbol = %self.symbol,
                        mode = ?self.mode,
                        live_quotes = self.live_quotes.len(),
                        uptime_ratio = %snapshot.uptime_ratio,
                        "strategy heartbeat"
                    );
                }
            }
        }
    }

    async fn refresh_from_latest(
        &mut self,
        executor: &dyn OrderExecutor,
        now: tokio::time::Instant,
    ) -> Result<()> {
        self.update_mode_for_timers(now);

        // Check fills before placing new quotes.
        self.handle_fills(now).await?;

        let (mark_price, reference_price) = {
            let snapshot = self.price_rx.borrow();
            (snapshot.mark_price, self.quote_reference_price(&snapshot))
        };
        if reference_price <= Decimal::ZERO {
            self.uptime_tracker.update(now, false);
            return Ok(());
        }

        let risk_now = std::time::Instant::now();
        self.risk_manager.record_price(risk_now, mark_price);
        let risk_state = self.risk_manager.assess(risk_now, None, None);
        match &risk_state {
            RiskState::Safe => {}
            RiskState::Caution { reasons } => {
                warn!(symbol = %self.symbol, ?reasons, "risk caution; skipping quotes");
                self.cancel_all_quotes(executor, now).await;
                self.uptime_tracker.update(now, false);
                return Ok(());
            }
            RiskState::Halt { reasons } => {
                warn!(symbol = %self.symbol, ?reasons, "risk halt; skipping quotes");
                self.cancel_all_quotes(executor, now).await;
                self.uptime_tracker.update(now, false);
                return Ok(());
            }
        }

        self.refresh_quotes(executor, now, reference_price).await
    }

    fn update_mode_for_timers(&mut self, now: tokio::time::Instant) {
        if let Some(until) = self.survival_until
            && now >= until
        {
            self.survival_until = None;
            self.mode = self.preferred_mode;
        }

        self.update_backoff_for_timers(now);
    }

    async fn handle_fills(&mut self, now: tokio::time::Instant) -> Result<()> {
        let mut filled_slots = Vec::new();

        for (slot, quote) in self.live_quotes.iter() {
            let cl_ord_id = quote.cl_ord_id.as_str();
            if self.handled_fills.contains(cl_ord_id) {
                continue;
            }

            let state = {
                let tracker = self.order_tracker.lock().await;
                tracker.state(cl_ord_id).cloned()
            };

            if matches!(state, Some(OrderState::Filled { .. })) {
                filled_slots.push((*slot, quote.cl_ord_id.clone()));
            }
        }

        if filled_slots.is_empty() {
            return Ok(());
        }

        for (slot, cl_ord_id) in filled_slots {
            if self.bootstrap_side.is_some() {
                self.bootstrap_side = None;
                info!(symbol = %self.symbol, "bootstrap fill detected; switching to bilateral quoting");
            }
            self.handled_fills.insert(cl_ord_id);
            self.risk_manager.record_fill(std::time::Instant::now());
            if let Some(quote) = self.live_quotes.get(&slot) {
                let signed_qty = match slot.side {
                    QuoteSide::Bid => quote.qty,
                    QuoteSide::Ask => -quote.qty,
                };
                self.inventory_qty += signed_qty;
            }
            self.apply_fill_backoff(slot.side, now);
            self.live_quotes.remove(&slot);
        }

        self.enter_survival(now);

        info!(
            symbol = %self.symbol,
            ?self.mode,
            backoff_secs = FILL_BACKOFF_DURATION.as_secs(),
            "full fill detected; applying side backoff"
        );

        Ok(())
    }

    async fn refresh_quotes(
        &mut self,
        executor: &dyn OrderExecutor,
        now: tokio::time::Instant,
        reference_price: Decimal,
    ) -> Result<()> {
        self.base_qty = self.derived_base_qty(reference_price);
        if self.base_qty <= Decimal::ZERO {
            self.cancel_all_quotes(executor, now).await;
            self.uptime_tracker.update(now, false);
            return Ok(());
        }

        for tier in self.active_tiers() {
            for side in [QuoteSide::Bid, QuoteSide::Ask] {
                if !self.bootstrap_allows_side(side) {
                    let slot = QuoteSlot { tier: *tier, side };
                    self.cancel_slot_if_present(executor, now, slot, None).await;
                    continue;
                }
                let slot = QuoteSlot { tier: *tier, side };
                self.refresh_slot(executor, now, reference_price, slot).await?;
            }
        }

        let active = self.is_uptime_active();
        self.uptime_tracker.update(now, active);
        Ok(())
    }

    fn is_uptime_active(&self) -> bool {
        // Require full bilateral ladder (active tiers) to count as uptime.
        let live = self
            .live_quotes
            .values()
            .filter(|quote| quote.cancel_in_flight.is_none())
            .count();
        live == self.active_tiers().len() * 2
    }

    async fn refresh_slot(
        &mut self,
        executor: &dyn OrderExecutor,
        now: tokio::time::Instant,
        reference_price: Decimal,
        slot: QuoteSlot,
    ) -> Result<()> {
        let target_bps = self.target_bps_for_tier(slot.tier);
        let mut desired_price = price_at_bps(reference_price, slot.side.to_order_side(), target_bps);
        desired_price = self.align_price_for_order(desired_price);
        let desired_qty = self.desired_qty_for_slot(slot.tier, slot.side, target_bps, now);
        let capped_qty = self.cap_qty_for_inventory(slot.side, desired_qty, reference_price);
        let backoff_active = self.is_backoff_active(slot.side, now);

        if capped_qty <= Decimal::ZERO || desired_price <= Decimal::ZERO {
            if let Some(existing) = self.live_quotes.get_mut(&slot)
                && let Some(cancel) = existing.cancel_in_flight.as_mut()
            {
                cancel.pending = None;
                return Ok(());
            }
            self.cancel_slot_if_present(executor, now, slot, None).await;
            return Ok(());
        }

        let mut effective_qty = capped_qty;

        if let Some(existing) = self.live_quotes.get(&slot).cloned() {
            let tracked_state = {
                let tracker = self.order_tracker.lock().await;
                tracker.get(&existing.cl_ord_id).cloned()
            };

            if let Some(tracked) = tracked_state {
                match tracked.state {
                    OrderState::PartiallyFilled { remaining_qty, .. } => {
                        let capped_remaining = decimal_min(remaining_qty, capped_qty);
                        effective_qty = capped_remaining;
                    }
                    OrderState::Filled { .. } => {
                        // Filled will be handled by `handle_fills`.
                        return Ok(());
                    }
                    OrderState::Cancelled { .. } | OrderState::Failed { .. } => {
                        if let Some(pending) = existing
                            .cancel_in_flight
                            .as_ref()
                            .and_then(|cancel| cancel.pending.clone())
                        {
                            desired_price = pending.price;
                            effective_qty = pending.qty;
                        }
                        self.live_quotes.remove(&slot);
                    }
                    _ => {}
                }
            }

            effective_qty = self.align_qty_for_order(effective_qty);
            if effective_qty <= Decimal::ZERO {
                if let Some(existing) = self.live_quotes.get_mut(&slot)
                    && let Some(cancel) = existing.cancel_in_flight.as_mut()
                {
                    cancel.pending = None;
                    return Ok(());
                }
                self.cancel_slot_if_present(executor, now, slot, None).await;
                return Ok(());
            }

            let mut has_live = false;
            let mut cancel_action: Option<(String, bool, bool)> = None;
            if let Some(still_live) = self.live_quotes.get_mut(&slot) {
                has_live = true;
                if let Some(cancel) = still_live.cancel_in_flight.as_mut() {
                    let mut request_reconcile = false;
                    let mut retry_cancel = false;

                    cancel.pending = Some(PendingQuote {
                        price: desired_price,
                        qty: effective_qty,
                    });

                    if now >= cancel.deadline {
                        if cancel
                            .last_reconcile_at
                            .is_none_or(|last| {
                                now.saturating_duration_since(last) >= CANCEL_RECONCILE_COOLDOWN
                            })
                        {
                            request_reconcile = true;
                            cancel.last_reconcile_at = Some(now);
                        }

                        if now.saturating_duration_since(cancel.sent_at) >= CANCEL_RETRY_INTERVAL {
                            retry_cancel = true;
                            cancel.sent_at = now;
                            cancel.deadline = now + CANCEL_ACK_TIMEOUT;
                        }
                    }

                    cancel_action = Some((still_live.cl_ord_id.clone(), request_reconcile, retry_cancel));
                }
            }

            if let Some((cl_ord_id, request_reconcile, retry_cancel)) = cancel_action {
                if request_reconcile {
                    self.request_reconcile(OrderReconcileRequest {
                        cl_ord_id: cl_ord_id.clone(),
                        reason: OrderReconcileReason::CancelTimeout,
                    });
                }

                if retry_cancel {
                    let req = CancelOrderRequest {
                        order_id: None,
                        cl_ord_id: Some(cl_ord_id.clone()),
                    };

                    match executor.cancel_order(req).await {
                        Ok(resp) if resp.code == 0 => {
                            info!(symbol = %self.symbol, cl_ord_id = %cl_ord_id, "cancel retry requested");
                        }
                        Ok(resp) => {
                            warn!(
                                symbol = %self.symbol,
                                cl_ord_id = %cl_ord_id,
                                code = resp.code,
                                message = %resp.message,
                                "cancel retry returned non-zero code"
                            );
                        }
                        Err(err) => {
                            warn!(symbol = %self.symbol, cl_ord_id = %cl_ord_id, error = %err, "cancel retry http failed");
                        }
                    }
                }
                return Ok(());
            }

            if has_live {
                let (still_price, still_qty, placed_at) = match self.live_quotes.get(&slot) {
                    Some(quote) => (quote.price, quote.qty, quote.placed_at),
                    None => return Ok(()),
                };

                let wants_reduce = backoff_active && capped_qty < still_qty;
                let (band_min, band_max) = self.quote_band_for_tier(slot.tier);
                let current_bps =
                    bps_from_price(reference_price, slot.side.to_order_side(), still_price);
                let outside_band = current_bps < band_min || current_bps > band_max;
                let drift_replace = if slot.tier == Tier::L1 {
                    let age = now.saturating_duration_since(placed_at);
                    if age >= L1_MIN_REST {
                        should_replace(
                            still_price,
                            desired_price,
                            self.replace_drift_threshold_bps(slot.tier),
                        )
                    } else {
                        false
                    }
                } else {
                    false
                };

                if outside_band || drift_replace || wants_reduce {
                    self.cancel_slot_if_present(
                        executor,
                        now,
                        slot,
                        Some(PendingQuote {
                            price: desired_price,
                            qty: effective_qty,
                        }),
                    )
                    .await;
                } else {
                    // Keep current quote; update qty bookkeeping when partially filled.
                    if effective_qty != still_qty
                        && let Some(q) = self.live_quotes.get_mut(&slot)
                    {
                        q.qty = effective_qty;
                    }
                }
                return Ok(());
            }
        }

        effective_qty = self.align_qty_for_order(effective_qty);
        if effective_qty <= Decimal::ZERO {
            self.cancel_slot_if_present(executor, now, slot, None).await;
            return Ok(());
        }

        self.place_slot(
            executor,
            now,
            slot,
            desired_price,
            effective_qty,
            reference_price,
        )
            .await?;
        Ok(())
    }

    fn request_reconcile(&self, request: OrderReconcileRequest) {
        let _ = self.order_reconcile_tx.send(request);
    }

    fn target_bps_for_tier(&self, tier: Tier) -> Decimal {
        let (min, max) = self.quote_band_for_tier(tier);
        (min + max) / Decimal::from(2)
    }

    fn quote_band_for_tier(&self, tier: Tier) -> (Decimal, Decimal) {
        let (tier_min, tier_max) = tier.min_max_bps();
        let (mode_min, mode_max) = self.mode.target_range();

        let min = decimal_max(tier_min, mode_min);
        let max = decimal_min(tier_max, mode_max);

        if min <= max {
            (min, max)
        } else {
            (tier_min, tier_max)
        }
    }

    fn should_refresh_for_price(&self, reference_price: Decimal, now: tokio::time::Instant) -> bool {
        if reference_price <= Decimal::ZERO {
            return false;
        }

        for (slot, quote) in self.live_quotes.iter() {
            let (band_min, band_max) = self.quote_band_for_tier(slot.tier);
            let current_bps = bps_from_price(reference_price, slot.side.to_order_side(), quote.price);
            if current_bps < band_min || current_bps > band_max {
                return true;
            }

            if slot.tier == Tier::L1 {
                let age = now.saturating_duration_since(quote.placed_at);
                if age >= L1_MIN_REST {
                    let target_bps = self.target_bps_for_tier(slot.tier);
                    let desired_price =
                        price_at_bps(reference_price, slot.side.to_order_side(), target_bps);
                    let drift_threshold = self.replace_drift_threshold_bps(slot.tier);
                    if should_replace(quote.price, desired_price, drift_threshold) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn quote_reference_price(&self, snapshot: &SymbolPrice) -> Decimal {
        if let Some(mid_price) = snapshot.mid_price
            && mid_price > Decimal::ZERO
        {
            return mid_price;
        }

        if let Some(last_price) = snapshot.last_price
            && last_price > Decimal::ZERO
        {
            return last_price;
        }

        snapshot.mark_price
    }

    fn bootstrap_allows_side(&self, side: QuoteSide) -> bool {
        self.bootstrap_side
            .is_none_or(|bootstrap| bootstrap == side)
    }

    fn side_increases_inventory_abs(&self, side: QuoteSide) -> bool {
        match side {
            QuoteSide::Bid => self.inventory_qty >= Decimal::ZERO,
            QuoteSide::Ask => self.inventory_qty <= Decimal::ZERO,
        }
    }

    fn cap_qty_for_inventory(
        &self,
        side: QuoteSide,
        desired_qty: Decimal,
        mark_price: Decimal,
    ) -> Decimal {
        if desired_qty <= Decimal::ZERO {
            return Decimal::ZERO;
        }

        if self.max_non_usd_value <= Decimal::ZERO {
            return desired_qty;
        }

        if mark_price <= Decimal::ZERO {
            return Decimal::ZERO;
        }

        if !self.side_increases_inventory_abs(side) {
            return desired_qty;
        }

        let inventory_value = self.inventory_qty.abs() * mark_price;
        if inventory_value >= self.max_non_usd_value {
            return Decimal::ZERO;
        }

        let allowed_value = self.max_non_usd_value - inventory_value;
        let allowed_qty = allowed_value / mark_price;
        decimal_min(desired_qty, allowed_qty)
    }

    fn desired_qty_for_slot(
        &self,
        tier: Tier,
        side: QuoteSide,
        bps: Decimal,
        now: tokio::time::Instant,
    ) -> Decimal {
        let weight = self.tier_weight(tier);
        if weight <= Decimal::ZERO {
            return Decimal::ZERO;
        }

        // Sizing heuristic (inherited):
        // - 0-10 bps: 100%
        // - 10-30 bps: 50%
        // - 30-100 bps: 10%
        let multiplier = if bps <= Decimal::from(10) {
            Decimal::ONE
        } else if bps <= Decimal::from(30) {
            Decimal::new(5, 1)
        } else if bps <= Decimal::from(100) {
            Decimal::new(1, 1)
        } else {
            Decimal::ZERO
        };

        let backoff = if self.is_backoff_active(side, now) {
            fill_backoff_multiplier()
        } else {
            Decimal::ONE
        };

        self.base_qty * weight * multiplier * backoff
    }

    fn derived_base_qty(&self, mark_price: Decimal) -> Decimal {
        if mark_price <= Decimal::ZERO || self.budget_usd <= Decimal::ZERO {
            return Decimal::ZERO;
        }

        let total_weight = self.total_weight();
        if total_weight <= Decimal::ZERO {
            return Decimal::ZERO;
        }

        // Split total budget across bid/ask; total notional equals budget_usd.
        let per_side_budget = self.budget_usd / Decimal::from(2);
        per_side_budget / mark_price / total_weight
    }

    fn total_weight(&self) -> Decimal {
        self.active_tiers()
            .iter()
            .fold(Decimal::ZERO, |acc, tier| acc + self.tier_weight(*tier))
    }

    fn tier_weight(&self, tier: Tier) -> Decimal {
        match self.tier_count {
            1 => match tier {
                Tier::L1 => Decimal::ONE,
                _ => Decimal::ZERO,
            },
            2 => match tier {
                Tier::L1 => Decimal::new(6, 1),
                Tier::L2 => Decimal::new(4, 1),
                _ => Decimal::ZERO,
            },
            3 => match tier {
                Tier::L1 => Decimal::new(4, 1),
                Tier::L2 => Decimal::new(35, 2),
                Tier::L3 => Decimal::new(25, 2),
                _ => Decimal::ZERO,
            },
            _ => match tier {
                Tier::L1 => Decimal::new(3, 1),
                Tier::L2 => Decimal::new(25, 2),
                Tier::L3 => Decimal::new(2, 1),
                Tier::L4 => Decimal::new(15, 2),
                Tier::L5 => Decimal::new(1, 1),
            },
        }
    }

    fn active_tiers(&self) -> &'static [Tier] {
        match self.tier_count {
            1 => &TIERS_L1,
            2 => &TIERS_L1_L2,
            3 => &TIERS_L1_L2_L3,
            _ => &TIERS_ALL,
        }
    }

    fn enter_survival(&mut self, now: tokio::time::Instant) {
        if !self.mode.is_survival() {
            self.mode = StrategyMode::survival_default();
        }
        self.survival_until = Some(now + SURVIVAL_AFTER_FILL);
    }

    fn is_backoff_active(&self, side: QuoteSide, now: tokio::time::Instant) -> bool {
        match side {
            QuoteSide::Bid => self.bid_backoff_until.is_some_and(|until| now < until),
            QuoteSide::Ask => self.ask_backoff_until.is_some_and(|until| now < until),
        }
    }

    fn apply_fill_backoff(&mut self, side: QuoteSide, now: tokio::time::Instant) {
        let until = now + FILL_BACKOFF_DURATION;
        match side {
            QuoteSide::Bid => {
                self.bid_backoff_until = Some(match self.bid_backoff_until {
                    Some(existing) if existing > until => existing,
                    _ => until,
                });
            }
            QuoteSide::Ask => {
                self.ask_backoff_until = Some(match self.ask_backoff_until {
                    Some(existing) if existing > until => existing,
                    _ => until,
                });
            }
        }
    }

    fn update_backoff_for_timers(&mut self, now: tokio::time::Instant) {
        if self.bid_backoff_until.is_some_and(|until| now >= until) {
            self.bid_backoff_until = None;
        }
        if self.ask_backoff_until.is_some_and(|until| now >= until) {
            self.ask_backoff_until = None;
        }
    }

    fn replace_drift_threshold_bps(&self, tier: Tier) -> Decimal {
        match tier {
            Tier::L1 => Decimal::new(25, 1),
            _ => Decimal::from(REPLACE_DRIFT_BPS),
        }
    }

    async fn place_slot(
        &mut self,
        executor: &dyn OrderExecutor,
        now: tokio::time::Instant,
        slot: QuoteSlot,
        price: Decimal,
        qty: Decimal,
        reference_price: Decimal,
    ) -> Result<()> {
        let price = self.align_price_for_order(price);
        if price <= Decimal::ZERO {
            return Ok(());
        }
        let qty = self.align_qty_for_order(qty);
        if qty <= Decimal::ZERO {
            return Ok(());
        }

        let cl_ord_id = format!(
            "mm:{}:{}:{}:{}",
            self.symbol,
            slot.side.as_str(),
            slot.tier.as_str(),
            Uuid::new_v4()
        );

        {
            let mut tracker = self.order_tracker.lock().await;
            tracker
                .register_pending(cl_ord_id.clone(), qty, std::time::Instant::now())
                .map_err(|err| anyhow!("order_tracker register_pending failed: {err}"))?;
        }

        let req = NewOrderRequest {
            symbol: self.symbol.clone(),
            side: slot.side.to_order_side(),
            order_type: OrderType::Limit,
            qty,
            time_in_force: TimeInForce::PostOnly,
            reduce_only: false,
            price: Some(price),
            cl_ord_id: Some(cl_ord_id.clone()),
            margin_mode: None,
            leverage: None,
            tp_price: None,
            sl_price: None,
        };

        match executor.new_order(req).await {
            Ok(resp) if resp.code == 0 => {
                let mut tracker = self.order_tracker.lock().await;
                if let Err(err) = tracker.mark_sent(&cl_ord_id, std::time::Instant::now()) {
                    warn!(symbol = %self.symbol, cl_ord_id = %cl_ord_id, error = %err, "order_tracker mark_sent failed");
                }

                info!(
                    symbol = %self.symbol,
                    side = %slot.side.as_str(),
                    tier = %slot.tier.as_str(),
                    reference_price = %reference_price,
                    %price,
                    %qty,
                    "placed PostOnly quote"
                );

                self.live_quotes.insert(
                    slot,
                    LiveQuote {
                        cl_ord_id,
                        price,
                        qty,
                        placed_at: now,
                        cancel_in_flight: None,
                    },
                );
            }
            Ok(resp) => {
                let mut tracker = self.order_tracker.lock().await;
                let _ = tracker.mark_failed(&cl_ord_id, format!("new_order code={}", resp.code));
                error!(
                    symbol = %self.symbol,
                    side = %slot.side.as_str(),
                    tier = %slot.tier.as_str(),
                    %price,
                    %qty,
                    code = resp.code,
                    message = %resp.message,
                    "new_order returned non-zero code"
                );
                return Err(anyhow!(
                    "new_order returned code={} message={}",
                    resp.code,
                    resp.message
                ));
            }
            Err(err) => {
                let mut tracker = self.order_tracker.lock().await;
                let _ = tracker.mark_failed(&cl_ord_id, format!("new_order http={err}"));
                error!(
                    symbol = %self.symbol,
                    side = %slot.side.as_str(),
                    tier = %slot.tier.as_str(),
                    %price,
                    %qty,
                    error = %err,
                    "new_order http failed"
                );
                return Err(anyhow!(err));
            }
        }

        // First placement can flip uptime active quickly.
        self.uptime_tracker.update(now, self.is_uptime_active());

        Ok(())
    }

    async fn cancel_slot_if_present(
        &mut self,
        executor: &dyn OrderExecutor,
        now: tokio::time::Instant,
        slot: QuoteSlot,
        pending: Option<PendingQuote>,
    ) {
        let Some(existing) = self.live_quotes.get_mut(&slot) else {
            return;
        };

        if let Some(cancel) = existing.cancel_in_flight.as_mut() {
            if pending.is_some() {
                cancel.pending = pending;
            }
            return;
        }

        let cl_ord_id = existing.cl_ord_id.clone();

        {
            let mut tracker = self.order_tracker.lock().await;
            let _ = tracker.mark_cancelling(&cl_ord_id, std::time::Instant::now());
        }

        let req = CancelOrderRequest {
            order_id: None,
            cl_ord_id: Some(cl_ord_id.clone()),
        };

        match executor.cancel_order(req).await {
            Ok(resp) if resp.code == 0 => {
                info!(symbol = %self.symbol, cl_ord_id = %cl_ord_id, "cancel requested");
            }
            Ok(resp) => {
                warn!(
                    symbol = %self.symbol,
                    cl_ord_id = %cl_ord_id,
                    code = resp.code,
                    message = %resp.message,
                    "cancel_order returned non-zero code"
                );
            }
            Err(err) => {
                warn!(symbol = %self.symbol, cl_ord_id = %cl_ord_id, error = %err, "cancel_order http failed");
            }
        }

        existing.cancel_in_flight = Some(CancelInFlight {
            sent_at: now,
            deadline: now + CANCEL_ACK_TIMEOUT,
            last_reconcile_at: None,
            pending,
        });
    }

    async fn cancel_all_quotes(&mut self, executor: &dyn OrderExecutor, now: tokio::time::Instant) {
        let slots: Vec<QuoteSlot> = self.live_quotes.keys().copied().collect();
        for slot in slots {
            self.cancel_slot_if_present(executor, now, slot, None).await;
        }
    }

    fn align_qty_for_order(&self, qty: Decimal) -> Decimal {
        if qty <= Decimal::ZERO {
            return qty;
        }

        let mut aligned = match self.qty_tick_decimals {
            Some(decimals) => qty.round_dp_with_strategy(decimals, RoundingStrategy::ToZero),
            None => qty,
        };

        if let Some(max_qty) = self.max_order_qty
            && aligned > max_qty
        {
            aligned = match self.qty_tick_decimals {
                Some(decimals) => max_qty.round_dp_with_strategy(decimals, RoundingStrategy::ToZero),
                None => max_qty,
            };
        }

        if let Some(min_qty) = self.min_order_qty
            && aligned < min_qty
        {
            return Decimal::ZERO;
        }

        aligned
    }

    fn align_price_for_order(&self, price: Decimal) -> Decimal {
        if price <= Decimal::ZERO {
            return price;
        }

        match self.price_tick_decimals {
            Some(decimals) => price.round_dp_with_strategy(decimals, RoundingStrategy::ToZero),
            None => price,
        }
    }
}

impl Default for MarketMakingStrategy {
    fn default() -> Self {
        Self::new()
    }
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

fn decimal_max(a: Decimal, b: Decimal) -> Decimal {
    if a >= b { a } else { b }
}

fn decimal_min(a: Decimal, b: Decimal) -> Decimal {
    if a <= b { a } else { b }
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

fn should_replace(current_price: Decimal, desired_price: Decimal, threshold_bps: Decimal) -> bool {
    if current_price <= Decimal::ZERO {
        return true;
    }
    let diff = (desired_price - current_price).abs();
    let drift_bps = (diff / current_price) * Decimal::from(BPS_DENOMINATOR);
    drift_bps >= threshold_bps
}

fn fill_backoff_multiplier() -> Decimal {
    Decimal::new(3, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::risk::RiskManager;
    use std::str::FromStr;
    use standx_point_adapter::ws::message::OrderUpdateData;
    use tokio::sync::mpsc;

    fn dec(value: &str) -> Decimal {
        Decimal::from_str(value).expect("valid decimal")
    }

    fn reconcile_tx() -> mpsc::UnboundedSender<OrderReconcileRequest> {
        let (tx, _rx) = mpsc::unbounded_channel();
        tx
    }

    #[derive(Debug, Default)]
    struct MockExecutor {
        new_orders: tokio::sync::Mutex<Vec<NewOrderRequest>>,
        cancels: tokio::sync::Mutex<Vec<CancelOrderRequest>>,
    }

    impl MockExecutor {
        async fn new_order_count(&self) -> usize {
            self.new_orders.lock().await.len()
        }

        async fn cancel_count(&self) -> usize {
            self.cancels.lock().await.len()
        }

        async fn last_new_order(&self) -> Option<NewOrderRequest> {
            self.new_orders.lock().await.last().cloned()
        }
    }

    impl OrderExecutor for MockExecutor {
        fn new_order(
            &self,
            req: NewOrderRequest,
        ) -> Pin<Box<dyn Future<Output = standx_point_adapter::Result<NewOrderResponse>> + Send + '_>>
        {
            Box::pin(async move {
                self.new_orders.lock().await.push(req);
                Ok(NewOrderResponse {
                    code: 0,
                    message: "ok".to_string(),
                    request_id: "req".to_string(),
                })
            })
        }

        fn cancel_order(
            &self,
            req: CancelOrderRequest,
        ) -> Pin<
            Box<dyn Future<Output = standx_point_adapter::Result<CancelOrderResponse>> + Send + '_>,
        > {
            Box::pin(async move {
                self.cancels.lock().await.push(req);
                Ok(CancelOrderResponse {
                    code: 0,
                    message: "ok".to_string(),
                    request_id: "req".to_string(),
                })
            })
        }
    }

    #[test]
    fn strategy_price_at_bps_is_relative_to_mark() {
        let mark = dec("100");
        let bid = price_at_bps(mark, Side::Buy, dec("5"));
        let ask = price_at_bps(mark, Side::Sell, dec("5"));

        // 5 bps = 0.05%
        assert_eq!(bid, dec("99.95"));
        assert_eq!(ask, dec("100.05"));
    }

    #[test]
    fn strategy_bps_from_price_is_relative_to_mark() {
        let mark = dec("100");
        let bid_bps = bps_from_price(mark, Side::Buy, dec("99.95"));
        let ask_bps = bps_from_price(mark, Side::Sell, dec("100.05"));

        assert_eq!(bid_bps, dec("5"));
        assert_eq!(ask_bps, dec("5"));
    }

    #[test]
    fn strategy_mode_target_bps_respects_tier_ranges() {
        let (tx, rx) = watch::channel(initial_symbol_price("BTC-USD"));
        drop(tx);

        let strategy = MarketMakingStrategy::new_with_params(
            "BTC-USD".to_string(),
            dec("1000"),
            RiskLevel::Low,
            rx,
            Arc::new(Mutex::new(OrderTracker::new())),
            reconcile_tx(),
            StrategyMode::aggressive_default(),
            5,
            Decimal::ONE,
        );

        let l1 = strategy.target_bps_for_tier(Tier::L1);
        let l2 = strategy.target_bps_for_tier(Tier::L2);
        let l3 = strategy.target_bps_for_tier(Tier::L3);
        let l4 = strategy.target_bps_for_tier(Tier::L4);
        let l5 = strategy.target_bps_for_tier(Tier::L5);

        assert!(l1 >= dec("5") && l1 <= dec("8"));
        assert!(l2 >= dec("8") && l2 <= dec("10"));
        assert!(l3 >= dec("10") && l3 <= dec("15"));
        assert!(l4 >= dec("15") && l4 <= dec("20"));
        assert!(l5 >= dec("20") && l5 <= dec("30"));
    }

    #[test]
    fn strategy_aligns_qty_to_tick_and_bounds() {
        let mut strategy = MarketMakingStrategy::new();
        strategy.set_symbol_constraints(Some(2), Some(2), Some(dec("0.05")), Some(dec("1.00")));

        let rounded = strategy.align_qty_for_order(dec("0.1234"));
        assert_eq!(rounded, dec("0.12"));

        let too_small = strategy.align_qty_for_order(dec("0.041"));
        assert_eq!(too_small, Decimal::ZERO);

        let too_large = strategy.align_qty_for_order(dec("1.239"));
        assert_eq!(too_large, dec("1.00"));
    }

    #[test]
    fn strategy_aligns_price_to_tick() {
        let mut strategy = MarketMakingStrategy::new();
        strategy.set_symbol_constraints(Some(2), None, None, None);

        let rounded = strategy.align_price_for_order(dec("4956.560550"));
        assert_eq!(rounded, dec("4956.56"));
    }

    #[tokio::test]
    async fn strategy_places_post_only_bilateral_ladder() {
        let (tx, rx) = watch::channel(SymbolPrice {
            base: "BTC".to_string(),
            index_price: dec("100"),
            last_price: None,
            mark_price: dec("100"),
            mid_price: None,
            quote: "USD".to_string(),
            spread_ask: None,
            spread_bid: None,
            symbol: "BTC-USD".to_string(),
            time: "0".to_string(),
        });

        let executor = MockExecutor::default();
        let mut strategy = MarketMakingStrategy::new_with_params(
            "BTC-USD".to_string(),
            dec("1000"),
            RiskLevel::Low,
            rx,
            Arc::new(Mutex::new(OrderTracker::new())),
            reconcile_tx(),
            StrategyMode::aggressive_default(),
            5,
            Decimal::ONE,
        );

        strategy
            .refresh_from_latest(&executor, tokio::time::Instant::now())
            .await
            .unwrap();

        // 5 tiers * 2 sides
        assert_eq!(executor.new_order_count().await, 10);

        let last = executor.last_new_order().await.expect("has order");
        assert_eq!(last.order_type, OrderType::Limit);
        assert_eq!(last.time_in_force, TimeInForce::PostOnly);
        assert!(!last.reduce_only);
        assert!(last.price.is_some());

        // Bump mark price; drift should cause cancel+replace.
        tx.send_modify(|price| {
            price.mark_price = dec("101");
        });

        strategy
            .refresh_from_latest(&executor, tokio::time::Instant::now())
            .await
            .unwrap();

        // Drift is large; replace should happen (cancel + new orders).
        assert!(executor.cancel_count().await > 0);
    }

    #[tokio::test]
    async fn strategy_quotes_bilateral_from_start() {
        let (_tx, rx) = watch::channel(SymbolPrice {
            base: "BTC".to_string(),
            index_price: dec("100"),
            last_price: None,
            mark_price: dec("100"),
            mid_price: None,
            quote: "USD".to_string(),
            spread_ask: None,
            spread_bid: None,
            symbol: "BTC-USD".to_string(),
            time: "0".to_string(),
        });

        let executor = MockExecutor::default();
        let tracker = Arc::new(Mutex::new(OrderTracker::new()));
        let mut strategy = MarketMakingStrategy::new_with_params(
            "BTC-USD".to_string(),
            dec("1000"),
            RiskLevel::Low,
            rx,
            tracker,
            reconcile_tx(),
            StrategyMode::aggressive_default(),
            5,
            Decimal::ZERO,
        );

        strategy
            .refresh_from_latest(&executor, tokio::time::Instant::now())
            .await
            .unwrap();

        let orders = executor.new_orders.lock().await.clone();
        assert_eq!(orders.len(), 10);
        assert!(orders.iter().any(|order| order.side == Side::Buy));
        assert!(orders.iter().any(|order| order.side == Side::Sell));
    }

    #[tokio::test]
    async fn strategy_caps_bid_when_inventory_above_half_limit() {
        let (_tx, rx) = watch::channel(SymbolPrice {
            base: "BTC".to_string(),
            index_price: dec("100"),
            last_price: None,
            mark_price: dec("100"),
            mid_price: None,
            quote: "USD".to_string(),
            spread_ask: None,
            spread_bid: None,
            symbol: "BTC-USD".to_string(),
            time: "0".to_string(),
        });

        let executor = MockExecutor::default();
        let mut strategy = MarketMakingStrategy::new_with_params(
            "BTC-USD".to_string(),
            dec("1000"),
            RiskLevel::Low,
            rx,
            Arc::new(Mutex::new(OrderTracker::new())),
            reconcile_tx(),
            StrategyMode::aggressive_default(),
            5,
            dec("10"),
        );

        strategy
            .refresh_from_latest(&executor, tokio::time::Instant::now())
            .await
            .unwrap();

        let orders = executor.new_orders.lock().await.clone();
        assert!(orders.iter().all(|order| order.side == Side::Sell));
    }

    #[tokio::test]
    async fn strategy_full_fill_enters_survival_and_backoff() {
        let (_tx, rx) = watch::channel(SymbolPrice {
            base: "BTC".to_string(),
            index_price: dec("100"),
            last_price: None,
            mark_price: dec("100"),
            mid_price: None,
            quote: "USD".to_string(),
            spread_ask: None,
            spread_bid: None,
            symbol: "BTC-USD".to_string(),
            time: "0".to_string(),
        });

        let executor = MockExecutor::default();
        let tracker = Arc::new(Mutex::new(OrderTracker::new()));

        let mut strategy = MarketMakingStrategy::new_with_params(
            "BTC-USD".to_string(),
            dec("1000"),
            RiskLevel::Low,
            rx,
            tracker.clone(),
            reconcile_tx(),
            StrategyMode::aggressive_default(),
            5,
            Decimal::ONE,
        );

        strategy
            .refresh_from_latest(&executor, tokio::time::Instant::now())
            .await
            .unwrap();
        assert_eq!(executor.new_order_count().await, 10);

        // Mark one quote as filled.
        let slot = QuoteSlot {
            tier: Tier::L1,
            side: QuoteSide::Bid,
        };
        let filled_quote = strategy
            .live_quotes
            .get(&slot)
            .expect("has l1 bid quote")
            .clone();

        let exchange_order = standx_point_adapter::types::models::Order {
            avail_locked: Decimal::ZERO,
            cl_ord_id: filled_quote.cl_ord_id.clone(),
            closed_block: 0,
            created_at: "0".to_string(),
            created_block: 0,
            fill_avg_price: filled_quote.price,
            fill_qty: filled_quote.qty,
            id: 123,
            leverage: Decimal::ONE,
            liq_id: 0,
            margin: Decimal::ZERO,
            order_type: OrderType::Limit,
            payload: None,
            position_id: 0,
            price: Some(filled_quote.price),
            qty: filled_quote.qty,
            reduce_only: false,
            remark: String::new(),
            side: Side::Buy,
            source: "test".to_string(),
            status: standx_point_adapter::types::enums::OrderStatus::Filled,
            symbol: "BTC-USD".to_string(),
            time_in_force: TimeInForce::PostOnly,
            updated_at: "0".to_string(),
            user: "user".to_string(),
        };

        {
            let mut guard = tracker.lock().await;
            guard
                .reconcile_with_exchange(&[exchange_order], std::time::Instant::now())
                .unwrap();
        }

        strategy
            .refresh_from_latest(&executor, tokio::time::Instant::now())
            .await
            .unwrap();

        assert!(matches!(strategy.mode, StrategyMode::Survival { .. }));
        assert!(strategy.survival_until.is_some());
        assert!(strategy.bid_backoff_until.is_some());
        assert!(strategy.ask_backoff_until.is_none());
        assert!(!strategy.live_quotes.is_empty());

        let orders = executor.new_orders.lock().await.clone();
        let l1_bid_orders = orders
            .into_iter()
            .filter(|order| {
                order
                    .cl_ord_id
                    .as_deref()
                    .unwrap_or("")
                    .contains(":bid:l1:")
            })
            .collect::<Vec<_>>();

        let last = l1_bid_orders.last().expect("has l1 bid orders");
        assert_eq!(last.qty, dec("0.45"));
    }

    #[tokio::test]
    async fn strategy_partial_fill_requotes_remaining_qty() {
        let (tx, rx) = watch::channel(SymbolPrice {
            base: "BTC".to_string(),
            index_price: dec("100"),
            last_price: None,
            mark_price: dec("100"),
            mid_price: None,
            quote: "USD".to_string(),
            spread_ask: None,
            spread_bid: None,
            symbol: "BTC-USD".to_string(),
            time: "0".to_string(),
        });

        let executor = MockExecutor::default();
        let tracker = Arc::new(Mutex::new(OrderTracker::new()));
        let mut strategy = MarketMakingStrategy::new_with_params(
            "BTC-USD".to_string(),
            dec("1000"),
            RiskLevel::Low,
            rx,
            tracker.clone(),
            reconcile_tx(),
            StrategyMode::aggressive_default(),
            3,
            Decimal::ONE,
        );

        strategy
            .refresh_from_latest(&executor, tokio::time::Instant::now())
            .await
            .unwrap();

        let slot = QuoteSlot {
            tier: Tier::L1,
            side: QuoteSide::Bid,
        };
        let quote = strategy
            .live_quotes
            .get(&slot)
            .expect("has l1 bid quote")
            .clone();

        let exchange_order = standx_point_adapter::types::models::Order {
            avail_locked: Decimal::ZERO,
            cl_ord_id: quote.cl_ord_id.clone(),
            closed_block: 0,
            created_at: "0".to_string(),
            created_block: 0,
            fill_avg_price: quote.price,
            fill_qty: dec("0.1"),
            id: 777,
            leverage: Decimal::ONE,
            liq_id: 0,
            margin: Decimal::ZERO,
            order_type: OrderType::Limit,
            payload: None,
            position_id: 0,
            price: Some(quote.price),
            qty: quote.qty,
            reduce_only: false,
            remark: String::new(),
            side: Side::Buy,
            source: "test".to_string(),
            status: standx_point_adapter::types::enums::OrderStatus::PartiallyFilled,
            symbol: "BTC-USD".to_string(),
            time_in_force: TimeInForce::PostOnly,
            updated_at: "0".to_string(),
            user: "user".to_string(),
        };

        {
            let mut guard = tracker.lock().await;
            guard
                .reconcile_with_exchange(&[exchange_order], std::time::Instant::now())
                .unwrap();
        }

        tx.send_modify(|price| {
            price.mark_price = dec("101");
        });

        strategy
            .refresh_from_latest(&executor, tokio::time::Instant::now())
            .await
            .unwrap();

        assert!(executor.cancel_count().await > 0);

        {
            let update = OrderUpdateData {
                id: 777,
                symbol: "BTC-USD".to_string(),
                side: "buy".to_string(),
                status: "cancelled".to_string(),
                qty: quote.qty.to_string(),
                fill_qty: "0".to_string(),
                price: quote.price.to_string(),
                order_type: "limit".to_string(),
            };
            let mut guard = tracker.lock().await;
            guard
                .handle_ws_update(&update, std::time::Instant::now())
                .unwrap();
        }

        strategy
            .refresh_from_latest(&executor, tokio::time::Instant::now())
            .await
            .unwrap();

        let orders = executor.new_orders.lock().await.clone();
        let l1_bid = orders
            .into_iter()
            .filter(|order| {
                order
                    .cl_ord_id
                    .as_deref()
                    .unwrap_or("")
                    .contains(":bid:l1:")
            })
            .collect::<Vec<_>>();

        // Expect the replacement order to use remaining quantity (2.0 - 0.1 = 1.9).
        let last = l1_bid.last().expect("has l1 bid orders");
        assert_eq!(last.qty, dec("1.9"));
    }

    #[test]
    fn uptime_tracker_accumulates_active_time() {
        let t0 = tokio::time::Instant::now();
        let mut tracker = UptimeTracker::new(t0);

        tracker.update(t0, false);
        let t1 = t0 + Duration::from_secs(10);
        tracker.update(t1, true);
        let t2 = t1 + Duration::from_secs(30);

        let snapshot = tracker.snapshot(t2);
        assert_eq!(snapshot.active_duration, Duration::from_secs(30));
        assert_eq!(snapshot.total_duration, Duration::from_secs(40));
        assert!(snapshot.uptime_ratio > dec("0.7"));
    }

    #[tokio::test]
    async fn strategy_skips_quotes_on_risk_halt() {
        let (_tx, rx) = watch::channel(SymbolPrice {
            base: "BTC".to_string(),
            index_price: dec("100"),
            last_price: None,
            mark_price: dec("101"),
            mid_price: None,
            quote: "USD".to_string(),
            spread_ask: None,
            spread_bid: None,
            symbol: "BTC-USD".to_string(),
            time: "0".to_string(),
        });

        let executor = MockExecutor::default();
        let mut strategy = MarketMakingStrategy::new_with_params(
            "BTC-USD".to_string(),
            dec("1000"),
            RiskLevel::Low,
            rx,
            Arc::new(Mutex::new(OrderTracker::new())),
            reconcile_tx(),
            StrategyMode::aggressive_default(),
            3,
            Decimal::ONE,
        );

        strategy.risk_manager =
            RiskManager::with_limits(dec("1"), Decimal::ZERO, dec("1000"), u32::MAX, dec("1000"));
        let t0 = std::time::Instant::now() - std::time::Duration::from_millis(500);
        strategy.risk_manager.record_price(t0, dec("100"));

        strategy
            .refresh_from_latest(&executor, tokio::time::Instant::now())
            .await
            .unwrap();

        assert_eq!(executor.new_order_count().await, 0);
    }
}
