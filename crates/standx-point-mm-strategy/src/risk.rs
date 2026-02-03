/*
[INPUT]:  Price/depth snapshots, positions, fill events, and risk thresholds.
[OUTPUT]: RiskState (Safe/Caution/Halt) with guard reasons.
[POS]:    Risk layer - safety guards and trading throttles.
[UPDATE]: When guard logic or risk thresholds change.
*/

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use rust_decimal::Decimal;
use standx_point_adapter::types::models::{DepthBook, DepthLevel, Position};

const BPS_DENOMINATOR: i64 = 10_000;
const PRICE_WINDOW: Duration = Duration::from_secs(1);
const FILL_WINDOW: Duration = Duration::from_secs(60);

const DEFAULT_MAX_PRICE_VELOCITY_BPS: i64 = 1_000_000;
const DEFAULT_MAX_POSITION_SIZE: i64 = 1_000_000_000;
const DEFAULT_MAX_SPREAD_BPS: i64 = 1_000_000;

/// Risk manager that monitors trading conditions.
#[derive(Debug, Clone)]
pub struct RiskManager {
    max_price_velocity_bps: Decimal,
    min_depth_threshold: Decimal,
    max_position_size: Decimal,
    max_fill_rate_per_minute: u32,
    max_spread_bps: Decimal,
    price_history: VecDeque<(Instant, Decimal)>,
    fills_history: VecDeque<Instant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskState {
    Safe,
    Caution { reasons: Vec<String> },
    Halt { reasons: Vec<String> },
}

impl RiskManager {
    /// Create a new risk manager with permissive defaults.
    pub fn new() -> Self {
        Self::with_limits(
            Decimal::from(DEFAULT_MAX_PRICE_VELOCITY_BPS),
            Decimal::ZERO,
            Decimal::from(DEFAULT_MAX_POSITION_SIZE),
            u32::MAX,
            Decimal::from(DEFAULT_MAX_SPREAD_BPS),
        )
    }

    pub fn with_limits(
        max_price_velocity_bps: Decimal,
        min_depth_threshold: Decimal,
        max_position_size: Decimal,
        max_fill_rate_per_minute: u32,
        max_spread_bps: Decimal,
    ) -> Self {
        Self {
            max_price_velocity_bps,
            min_depth_threshold,
            max_position_size,
            max_fill_rate_per_minute,
            max_spread_bps,
            price_history: VecDeque::new(),
            fills_history: VecDeque::new(),
        }
    }

    pub fn record_price(&mut self, now: Instant, price: Decimal) {
        if price <= Decimal::ZERO {
            return;
        }

        self.price_history.push_back((now, price));
        self.trim_price_history(now);
    }

    pub fn record_fill(&mut self, now: Instant) {
        self.fills_history.push_back(now);
        self.trim_fills_history(now);
    }

    pub fn assess(
        &mut self,
        now: Instant,
        depth: Option<&DepthBook>,
        position: Option<&Position>,
    ) -> RiskState {
        self.trim_price_history(now);
        self.trim_fills_history(now);

        let mut halt_reasons = Vec::new();
        let mut caution_reasons = Vec::new();

        if let Some(velocity) = self.max_price_velocity_bps_per_second() {
            if velocity > self.max_price_velocity_bps {
                halt_reasons.push(format!(
                    "price velocity {:.2} bps/s exceeds limit {:.2}",
                    velocity, self.max_price_velocity_bps
                ));
            }
        }

        if let Some(depth_snapshot) = depth {
            let total_depth = aggregate_depth(depth_snapshot);
            if total_depth < self.min_depth_threshold {
                halt_reasons.push(format!(
                    "depth {:.4} below threshold {:.4}",
                    total_depth, self.min_depth_threshold
                ));
            }

            if let Some(spread) = spread_bps(depth_snapshot) {
                if spread > self.max_spread_bps {
                    caution_reasons.push(format!(
                        "spread {:.2} bps exceeds limit {:.2}",
                        spread, self.max_spread_bps
                    ));
                }
            }
        }

        if let Some(position_snapshot) = position {
            let position_value = position_snapshot.position_value.abs();
            if position_value > self.max_position_size {
                caution_reasons.push(format!(
                    "position value {:.4} exceeds limit {:.4}",
                    position_value, self.max_position_size
                ));
            }
        }

        let fill_rate = self.fills_history.len() as u32;
        if fill_rate > self.max_fill_rate_per_minute {
            halt_reasons.push(format!(
                "fill rate {} per minute exceeds limit {}",
                fill_rate, self.max_fill_rate_per_minute
            ));
        }

        if !halt_reasons.is_empty() {
            let mut reasons = halt_reasons;
            reasons.extend(caution_reasons);
            return RiskState::Halt { reasons };
        }

        if !caution_reasons.is_empty() {
            return RiskState::Caution {
                reasons: caution_reasons,
            };
        }

        RiskState::Safe
    }

    fn trim_price_history(&mut self, now: Instant) {
        while let Some((timestamp, _)) = self.price_history.front() {
            let Some(delta) = now.checked_duration_since(*timestamp) else {
                break;
            };

            if delta > PRICE_WINDOW {
                self.price_history.pop_front();
            } else {
                break;
            }
        }
    }

    fn trim_fills_history(&mut self, now: Instant) {
        while let Some(timestamp) = self.fills_history.front() {
            let Some(delta) = now.checked_duration_since(*timestamp) else {
                break;
            };

            if delta > FILL_WINDOW {
                self.fills_history.pop_front();
            } else {
                break;
            }
        }
    }

    fn max_price_velocity_bps_per_second(&self) -> Option<Decimal> {
        if self.price_history.len() < 2 {
            return None;
        }

        let mut max_velocity: Option<Decimal> = None;
        let mut iter = self.price_history.iter().cloned();
        let Some((mut t0, mut p0)) = iter.next() else {
            return None;
        };

        for (t1, p1) in iter {
            let prev_t = t0;
            let prev_p = p0;
            t0 = t1;
            p0 = p1;

            if prev_p <= Decimal::ZERO {
                continue;
            }

            let diff = (p1 - prev_p).abs();
            if diff.is_zero() {
                continue;
            }

            let Some(delta) = t1.checked_duration_since(prev_t) else {
                continue;
            };

            let elapsed_ms = delta.as_millis();
            if elapsed_ms == 0 {
                continue;
            }

            let elapsed_ms = elapsed_ms.min(i64::MAX as u128) as i64;
            let elapsed = Decimal::from(elapsed_ms) / Decimal::from(1000);
            let bps = (diff / prev_p) * Decimal::from(BPS_DENOMINATOR);
            let velocity = bps / elapsed;

            max_velocity = Some(match max_velocity {
                Some(current) => decimal_max(current, velocity),
                None => velocity,
            });
        }

        max_velocity
    }
}

impl Default for RiskManager {
    fn default() -> Self {
        Self::new()
    }
}

fn aggregate_depth(depth: &DepthBook) -> Decimal {
    let bids = sum_depth_levels(&depth.bids);
    let asks = sum_depth_levels(&depth.asks);
    bids + asks
}

fn sum_depth_levels(levels: &[DepthLevel]) -> Decimal {
    levels.iter().fold(Decimal::ZERO, |acc, level| {
        let qty = level.1;
        if qty > Decimal::ZERO {
            acc + qty
        } else {
            acc
        }
    })
}

fn spread_bps(depth: &DepthBook) -> Option<Decimal> {
    let best_bid = best_bid(&depth.bids)?;
    let best_ask = best_ask(&depth.asks)?;

    if best_bid <= Decimal::ZERO || best_ask <= Decimal::ZERO {
        return None;
    }

    if best_ask <= best_bid {
        return Some(Decimal::ZERO);
    }

    let mid = (best_bid + best_ask) / Decimal::from(2);
    if mid <= Decimal::ZERO {
        return None;
    }

    Some(((best_ask - best_bid) / mid) * Decimal::from(BPS_DENOMINATOR))
}

fn best_bid(levels: &[DepthLevel]) -> Option<Decimal> {
    levels
        .iter()
        .map(|level| level.0)
        .filter(|price| *price > Decimal::ZERO)
        .max()
}

fn best_ask(levels: &[DepthLevel]) -> Option<Decimal> {
    levels
        .iter()
        .map(|level| level.0)
        .filter(|price| *price > Decimal::ZERO)
        .min()
}

fn decimal_max(a: Decimal, b: Decimal) -> Decimal {
    if a >= b {
        a
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use standx_point_adapter::types::enums::MarginMode;
    use std::str::FromStr;

    fn dec(value: &str) -> Decimal {
        Decimal::from_str(value).expect("valid decimal")
    }

    fn depth_book(bid_price: &str, bid_qty: &str, ask_price: &str, ask_qty: &str) -> DepthBook {
        DepthBook {
            bids: vec![DepthLevel(dec(bid_price), dec(bid_qty))],
            asks: vec![DepthLevel(dec(ask_price), dec(ask_qty))],
            symbol: "BTC-USD".to_string(),
        }
    }

    fn test_position(position_value: &str) -> Position {
        Position {
            bankruptcy_price: Decimal::ZERO,
            created_at: "0".to_string(),
            entry_price: Decimal::ZERO,
            entry_value: Decimal::ZERO,
            holding_margin: Decimal::ZERO,
            id: 1,
            initial_margin: Decimal::ZERO,
            leverage: Decimal::ONE,
            liq_price: Decimal::ZERO,
            maint_margin: Decimal::ZERO,
            margin_asset: "USD".to_string(),
            margin_mode: MarginMode::Cross,
            mark_price: Decimal::ZERO,
            mmr: Decimal::ZERO,
            position_value: dec(position_value),
            qty: Decimal::ZERO,
            realized_pnl: Decimal::ZERO,
            status: "open".to_string(),
            symbol: "BTC-USD".to_string(),
            time: "0".to_string(),
            updated_at: "0".to_string(),
            upnl: Decimal::ZERO,
            user: "user".to_string(),
        }
    }

    #[test]
    fn risk_price_velocity_triggers_halt() {
        let mut manager =
            RiskManager::with_limits(dec("10"), Decimal::ZERO, dec("1000"), u32::MAX, dec("1000"));

        let t0 = Instant::now();
        let t1 = t0 + Duration::from_secs(1);
        manager.record_price(t0, dec("100"));
        manager.record_price(t1, dec("101"));

        let state = manager.assess(t1, None, None);
        assert!(matches!(state, RiskState::Halt { .. }));
    }

    #[test]
    fn risk_depth_triggers_halt() {
        let mut manager =
            RiskManager::with_limits(dec("1000"), dec("10"), dec("1000"), u32::MAX, dec("1000"));

        let depth = depth_book("100", "2", "101", "2");
        let state = manager.assess(Instant::now(), Some(&depth), None);
        assert!(matches!(state, RiskState::Halt { .. }));
    }

    #[test]
    fn risk_position_limit_triggers_caution() {
        let mut manager =
            RiskManager::with_limits(dec("1000"), Decimal::ZERO, dec("50"), u32::MAX, dec("1000"));

        let position = test_position("100");
        let state = manager.assess(Instant::now(), None, Some(&position));
        assert!(matches!(state, RiskState::Caution { .. }));
    }

    #[test]
    fn risk_fill_rate_triggers_halt() {
        let mut manager =
            RiskManager::with_limits(dec("1000"), Decimal::ZERO, dec("1000"), 2, dec("1000"));

        let t0 = Instant::now();
        manager.record_fill(t0);
        manager.record_fill(t0 + Duration::from_secs(10));
        manager.record_fill(t0 + Duration::from_secs(20));

        let state = manager.assess(t0 + Duration::from_secs(30), None, None);
        assert!(matches!(state, RiskState::Halt { .. }));
    }

    #[test]
    fn risk_spread_triggers_caution() {
        let mut manager =
            RiskManager::with_limits(dec("1000"), Decimal::ZERO, dec("1000"), u32::MAX, dec("50"));

        let depth = depth_book("100", "5", "101", "5");
        let state = manager.assess(Instant::now(), Some(&depth), None);
        assert!(matches!(state, RiskState::Caution { .. }));
    }

    #[test]
    fn risk_safe_when_no_triggers() {
        let mut manager = RiskManager::new();
        let state = manager.assess(Instant::now(), None, None);
        assert_eq!(state, RiskState::Safe);
    }
}
