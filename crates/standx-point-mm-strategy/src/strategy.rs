/*
[INPUT]:  `watch::Receiver<SymbolPrice>` (mark price), `StandxClient` for order placement,
          and `OrderTracker` updates (ack/fills/cancels via external WS reconciliation).
[OUTPUT]: PostOnly limit orders (bid+ask ladder) kept in sync with mark price,
          plus uptime accounting for reward eligibility.
[POS]:    Strategy layer - conservative market making core loop.
[UPDATE]: When changing tier ranges, sizing rules, drift thresholds, or cooldown/mode semantics.
*/

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use rust_decimal::Decimal;
use tokio::sync::{Mutex, watch};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use uuid::Uuid;

use standx_point_adapter::{
    CancelOrderRequest, CancelOrderResponse, NewOrderRequest, NewOrderResponse, OrderType, Side,
    StandxClient, SymbolPrice, TimeInForce,
};

use crate::order_state::{OrderState, OrderTracker};
use crate::risk::{RiskManager, RiskState};

const BPS_DENOMINATOR: i64 = 10_000;
const QUOTE_REFRESH_INTERVAL: Duration = Duration::from_secs(4); // 3-5s target

const FULL_FILL_COOLDOWN: Duration = Duration::from_secs(10);
const SURVIVAL_AFTER_FILL: Duration = Duration::from_secs(60);

// Replace quotes when desired price drifts by >= 1 bps.
const REPLACE_DRIFT_BPS: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Conservative,
    Moderate,
    Aggressive,
}

impl std::str::FromStr for RiskLevel {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value.trim().to_ascii_lowercase().as_str() {
            "aggressive" => Self::Aggressive,
            "moderate" => Self::Moderate,
            _ => Self::Conservative,
        })
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
}

impl Tier {
    fn all() -> [Tier; 3] {
        [Tier::L1, Tier::L2, Tier::L3]
    }

    fn min_max_bps(self) -> (Decimal, Decimal) {
        match self {
            Tier::L1 => (Decimal::ZERO, Decimal::from(5)),
            Tier::L2 => (Decimal::from(5), Decimal::from(10)),
            Tier::L3 => (Decimal::from(10), Decimal::from(20)),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Tier::L1 => "l1",
            Tier::L2 => "l2",
            Tier::L3 => "l3",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum QuoteSide {
    Bid,
    Ask,
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
struct LiveQuote {
    cl_ord_id: String,
    price: Decimal,
    qty: Decimal,
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

trait OrderExecutor {
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
    risk_level: RiskLevel,
    price_rx: watch::Receiver<SymbolPrice>,
    order_tracker: Arc<Mutex<OrderTracker>>,
    risk_manager: RiskManager,
    uptime_tracker: UptimeTracker,
    mode: StrategyMode,

    preferred_mode: StrategyMode,
    survival_until: Option<tokio::time::Instant>,
    cooldown_until: Option<tokio::time::Instant>,
    live_quotes: HashMap<QuoteSlot, LiveQuote>,
    handled_fills: HashSet<String>,
}

impl MarketMakingStrategy {
    /// Create a new market making strategy.
    ///
    /// Note: this uses placeholder values. Prefer `new_with_params` for real wiring.
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(initial_symbol_price(""));
        drop(tx);

        let now = tokio::time::Instant::now();
        let mode = StrategyMode::aggressive_default();
        Self {
            symbol: String::new(),
            base_qty: Decimal::ZERO,
            risk_level: RiskLevel::Conservative,
            price_rx: rx,
            order_tracker: Arc::new(Mutex::new(OrderTracker::new())),
            risk_manager: RiskManager::new(),
            uptime_tracker: UptimeTracker::new(now),
            mode,
            preferred_mode: mode,
            survival_until: None,
            cooldown_until: None,
            live_quotes: HashMap::new(),
            handled_fills: HashSet::new(),
        }
    }

    pub fn new_with_params(
        symbol: String,
        base_qty: Decimal,
        risk_level: RiskLevel,
        price_rx: watch::Receiver<SymbolPrice>,
        order_tracker: Arc<Mutex<OrderTracker>>,
        mode: StrategyMode,
    ) -> Self {
        let now = tokio::time::Instant::now();
        Self {
            symbol,
            base_qty,
            risk_level,
            price_rx,
            order_tracker,
            risk_manager: RiskManager::new(),
            uptime_tracker: UptimeTracker::new(now),
            mode,
            preferred_mode: mode,
            survival_until: None,
            cooldown_until: None,
            live_quotes: HashMap::new(),
            handled_fills: HashSet::new(),
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

        // Try to quote immediately using the current snapshot.
        self.refresh_from_latest(executor, tokio::time::Instant::now())
            .await?;

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    info!(symbol = %self.symbol, "strategy shutdown requested");
                    let now = tokio::time::Instant::now();
                    self.cancel_all_quotes(executor).await;
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

                    // Avoid refreshing on every price update; only kick-start quoting.
                    if self.live_quotes.is_empty() {
                        self.refresh_from_latest(executor, tokio::time::Instant::now()).await?;
                    }
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

        if self.in_cooldown(now) {
            self.cancel_all_quotes(executor).await;
            self.uptime_tracker.update(now, false);
            return Ok(());
        }

        // Check fills before placing new quotes.
        if self.handle_fills(executor, now).await? {
            return Ok(());
        }

        let mark_price = self.price_rx.borrow().mark_price;
        if mark_price <= Decimal::ZERO {
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
                self.cancel_all_quotes(executor).await;
                self.uptime_tracker.update(now, false);
                return Ok(());
            }
            RiskState::Halt { reasons } => {
                warn!(symbol = %self.symbol, ?reasons, "risk halt; skipping quotes");
                self.cancel_all_quotes(executor).await;
                self.uptime_tracker.update(now, false);
                return Ok(());
            }
        }

        self.refresh_quotes(executor, now, mark_price).await
    }

    fn update_mode_for_timers(&mut self, now: tokio::time::Instant) {
        if let Some(until) = self.survival_until && now >= until {
            self.survival_until = None;
            self.mode = self.preferred_mode;
        }
    }

    fn in_cooldown(&self, now: tokio::time::Instant) -> bool {
        self.cooldown_until.is_some_and(|until| now < until)
    }

    async fn handle_fills(
        &mut self,
        executor: &dyn OrderExecutor,
        now: tokio::time::Instant,
    ) -> Result<bool> {
        let mut filled: Option<String> = None;

        for quote in self.live_quotes.values() {
            let cl_ord_id = quote.cl_ord_id.as_str();
            if self.handled_fills.contains(cl_ord_id) {
                continue;
            }

            let state = {
                let tracker = self.order_tracker.lock().await;
                tracker.state(cl_ord_id).cloned()
            };

            if matches!(state, Some(OrderState::Filled { .. })) {
                filled = Some(cl_ord_id.to_string());
                break;
            }
        }

        let Some(filled_id) = filled else {
            return Ok(false);
        };

        self.handled_fills.insert(filled_id);
        self.risk_manager.record_fill(std::time::Instant::now());
        self.cooldown_until = Some(now + FULL_FILL_COOLDOWN);

        if !self.mode.is_survival() {
            self.mode = StrategyMode::survival_default();
            self.survival_until = Some(now + SURVIVAL_AFTER_FILL);
        }

        info!(symbol = %self.symbol, ?self.mode, cooldown_secs = FULL_FILL_COOLDOWN.as_secs(), "full fill detected; entering cooldown");

        self.cancel_all_quotes(executor).await;
        self.uptime_tracker.update(now, false);
        Ok(true)
    }

    async fn refresh_quotes(
        &mut self,
        executor: &dyn OrderExecutor,
        now: tokio::time::Instant,
        mark_price: Decimal,
    ) -> Result<()> {
        if self.base_qty <= Decimal::ZERO {
            self.cancel_all_quotes(executor).await;
            self.uptime_tracker.update(now, false);
            return Ok(());
        }

        for tier in Tier::all() {
            for side in [QuoteSide::Bid, QuoteSide::Ask] {
                let slot = QuoteSlot { tier, side };
                self.refresh_slot(executor, now, mark_price, slot).await?;
            }
        }

        let active = self.is_uptime_active();
        self.uptime_tracker.update(now, active);
        Ok(())
    }

    fn is_uptime_active(&self) -> bool {
        // Require full bilateral ladder (all tiers) to count as uptime.
        self.live_quotes.len() == Tier::all().len() * 2
    }

    async fn refresh_slot(
        &mut self,
        executor: &dyn OrderExecutor,
        now: tokio::time::Instant,
        mark_price: Decimal,
        slot: QuoteSlot,
    ) -> Result<()> {
        let target_bps = self.target_bps_for_tier(slot.tier);
        let desired_price = price_at_bps(mark_price, slot.side.to_order_side(), target_bps);
        let desired_qty = self.desired_qty_for_bps(target_bps);

        if desired_qty <= Decimal::ZERO || desired_price <= Decimal::ZERO {
            self.cancel_slot_if_present(executor, slot).await;
            return Ok(());
        }

        let mut effective_qty = desired_qty;

        if let Some(existing) = self.live_quotes.get(&slot).cloned() {
            let tracked_state = {
                let tracker = self.order_tracker.lock().await;
                tracker.get(&existing.cl_ord_id).cloned()
            };

            if let Some(tracked) = tracked_state {
                match tracked.state {
                    OrderState::PartiallyFilled { remaining_qty, .. } => {
                        effective_qty = remaining_qty;
                    }
                    OrderState::Filled { .. } => {
                        // Filled will be handled by `handle_fills`.
                        return Ok(());
                    }
                    OrderState::Cancelled { .. } | OrderState::Failed { .. } => {
                        self.live_quotes.remove(&slot);
                    }
                    _ => {}
                }
            }

            if let Some(still_live) = self.live_quotes.get(&slot) {
                if should_replace(
                    still_live.price,
                    desired_price,
                    self.replace_drift_threshold_bps(),
                ) {
                    self.cancel_slot_if_present(executor, slot).await;
                    self.place_slot(executor, now, slot, desired_price, effective_qty)
                        .await?;
                } else {
                    // Keep current quote; update qty bookkeeping when partially filled.
                    if effective_qty != still_live.qty
                        && let Some(q) = self.live_quotes.get_mut(&slot) {
                            q.qty = effective_qty;
                        }
                }
                return Ok(());
            }
        }

        self.place_slot(executor, now, slot, desired_price, effective_qty)
            .await?;
        Ok(())
    }

    fn target_bps_for_tier(&self, tier: Tier) -> Decimal {
        let (tier_min, tier_max) = tier.min_max_bps();
        let (mode_min, mode_max) = self.mode.target_range();

        let min = decimal_max(tier_min, mode_min);
        let max = decimal_min(tier_max, mode_max);

        if min <= max {
            (min + max) / Decimal::from(2)
        } else {
            (tier_min + tier_max) / Decimal::from(2)
        }
    }

    fn desired_qty_for_bps(&self, bps: Decimal) -> Decimal {
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

        self.base_qty * multiplier
    }

    fn replace_drift_threshold_bps(&self) -> Decimal {
        // Conservative mode avoids churn by requiring larger drift before cancel+replace.
        match self.risk_level {
            RiskLevel::Conservative => Decimal::from(2),
            RiskLevel::Moderate => Decimal::from(REPLACE_DRIFT_BPS),
            RiskLevel::Aggressive => Decimal::from(REPLACE_DRIFT_BPS),
        }
    }

    async fn place_slot(
        &mut self,
        executor: &dyn OrderExecutor,
        now: tokio::time::Instant,
        slot: QuoteSlot,
        price: Decimal,
        qty: Decimal,
    ) -> Result<()> {
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

                debug!(
                    symbol = %self.symbol,
                    side = %slot.side.as_str(),
                    tier = %slot.tier.as_str(),
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
                    },
                );
            }
            Ok(resp) => {
                let mut tracker = self.order_tracker.lock().await;
                let _ = tracker.mark_failed(&cl_ord_id, format!("new_order code={}", resp.code));
                return Err(anyhow!(
                    "new_order returned code={} message={}",
                    resp.code,
                    resp.message
                ));
            }
            Err(err) => {
                let mut tracker = self.order_tracker.lock().await;
                let _ = tracker.mark_failed(&cl_ord_id, format!("new_order http={err}"));
                return Err(anyhow!(err));
            }
        }

        // First placement can flip uptime active quickly.
        self.uptime_tracker.update(now, self.is_uptime_active());

        Ok(())
    }

    async fn cancel_slot_if_present(&mut self, executor: &dyn OrderExecutor, slot: QuoteSlot) {
        let Some(existing) = self.live_quotes.remove(&slot) else {
            return;
        };

        let cl_ord_id = existing.cl_ord_id;

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
                debug!(symbol = %self.symbol, cl_ord_id = %cl_ord_id, "cancel requested");
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
    }

    async fn cancel_all_quotes(&mut self, executor: &dyn OrderExecutor) {
        let slots: Vec<QuoteSlot> = self.live_quotes.keys().copied().collect();
        for slot in slots {
            self.cancel_slot_if_present(executor, slot).await;
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

fn should_replace(current_price: Decimal, desired_price: Decimal, threshold_bps: Decimal) -> bool {
    if current_price <= Decimal::ZERO {
        return true;
    }
    let diff = (desired_price - current_price).abs();
    let drift_bps = (diff / current_price) * Decimal::from(BPS_DENOMINATOR);
    drift_bps >= threshold_bps
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::risk::RiskManager;
    use std::str::FromStr;

    fn dec(value: &str) -> Decimal {
        Decimal::from_str(value).expect("valid decimal")
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
    fn strategy_mode_target_bps_respects_tier_ranges() {
        let (tx, rx) = watch::channel(initial_symbol_price("BTC-USD"));
        drop(tx);

        let strategy = MarketMakingStrategy::new_with_params(
            "BTC-USD".to_string(),
            dec("1"),
            RiskLevel::Conservative,
            rx,
            Arc::new(Mutex::new(OrderTracker::new())),
            StrategyMode::aggressive_default(),
        );

        let l1 = strategy.target_bps_for_tier(Tier::L1);
        let l2 = strategy.target_bps_for_tier(Tier::L2);
        let l3 = strategy.target_bps_for_tier(Tier::L3);

        assert!(l1 >= dec("0") && l1 <= dec("5"));
        assert!(l2 >= dec("5") && l2 <= dec("10"));
        assert!(l3 >= dec("10") && l3 <= dec("20"));
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
            dec("1"),
            RiskLevel::Conservative,
            rx,
            Arc::new(Mutex::new(OrderTracker::new())),
            StrategyMode::aggressive_default(),
        );

        strategy
            .refresh_from_latest(&executor, tokio::time::Instant::now())
            .await
            .unwrap();

        // 3 tiers * 2 sides
        assert_eq!(executor.new_order_count().await, 6);

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
    async fn strategy_full_fill_triggers_cooldown_and_cancels_quotes() {
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
            dec("1"),
            RiskLevel::Conservative,
            rx,
            tracker.clone(),
            StrategyMode::aggressive_default(),
        );

        strategy
            .refresh_from_latest(&executor, tokio::time::Instant::now())
            .await
            .unwrap();
        assert_eq!(executor.new_order_count().await, 6);

        // Mark one quote as filled.
        let filled_quote = strategy
            .live_quotes
            .values()
            .next()
            .expect("has quote")
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

        assert!(strategy.in_cooldown(tokio::time::Instant::now()));
        assert!(matches!(strategy.mode, StrategyMode::Survival { .. }));
        assert!(executor.cancel_count().await > 0);
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
            dec("1"),
            RiskLevel::Conservative,
            rx,
            tracker.clone(),
            StrategyMode::aggressive_default(),
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
            fill_qty: dec("0.4"),
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

        // Expect the replacement order to use remaining quantity (1.0 - 0.4 = 0.6).
        let last = l1_bid.last().expect("has l1 bid orders");
        assert_eq!(last.qty, dec("0.6"));
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
            dec("1"),
            RiskLevel::Conservative,
            rx,
            Arc::new(Mutex::new(OrderTracker::new())),
            StrategyMode::aggressive_default(),
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
