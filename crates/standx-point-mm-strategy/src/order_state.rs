/*
[INPUT]:  Local order intents, exchange order snapshots, and WebSocket updates.
[OUTPUT]: Tracked order states, timeout results, and reconciliation summary.
[POS]:    State layer - order lifecycle tracking and correlation.
[UPDATE]: When order state transitions or external order schemas change.
*/

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;
use std::time::{Duration, Instant};

use rust_decimal::Decimal;
use standx_point_adapter::types::enums::OrderStatus;
use standx_point_adapter::types::models::Order;
use standx_point_adapter::ws::message::OrderUpdateData;

/// Order state machine for tracking order lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderState {
    /// Order created locally, not yet sent.
    Pending { created_at: Instant },
    /// Order sent to exchange, waiting for ack.
    Sent { sent_at: Instant, cl_ord_id: String },
    /// Order acknowledged by exchange.
    Acknowledged { order_id: i64, acked_at: Instant },
    /// Order partially filled.
    PartiallyFilled {
        filled_qty: Decimal,
        remaining_qty: Decimal,
    },
    /// Order fully filled.
    Filled { filled_at: Instant },
    /// Cancel request sent.
    Cancelling { cancel_sent_at: Instant },
    /// Order cancelled.
    Cancelled { cancelled_at: Instant },
    /// Order failed.
    Failed { error: String },
}

impl OrderState {
    /// Returns true for terminal states.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            OrderState::Filled { .. } | OrderState::Cancelled { .. } | OrderState::Failed { .. }
        )
    }
}

/// Tracked order metadata keyed by client order id.
#[derive(Debug, Clone)]
pub struct TrackedOrder {
    pub cl_ord_id: String,
    pub order_id: Option<i64>,
    pub total_qty: Decimal,
    pub filled_qty: Decimal,
    pub state: OrderState,
}

/// Errors emitted by the order tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderTrackerError {
    DuplicateClOrdId {
        cl_ord_id: String,
    },
    UnknownClOrdId {
        cl_ord_id: String,
    },
    UnknownOrderId {
        order_id: i64,
    },
    OrderIdMismatch {
        order_id: i64,
        expected_cl_ord_id: String,
        actual_cl_ord_id: String,
    },
    OrderIdConflict {
        cl_ord_id: String,
        existing_order_id: i64,
        new_order_id: i64,
    },
    InvalidTransition {
        cl_ord_id: String,
        from: &'static str,
        to: &'static str,
    },
    InvalidDecimal {
        field: &'static str,
        value: String,
    },
    UnknownOrderStatus {
        value: String,
    },
}

impl fmt::Display for OrderTrackerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderTrackerError::DuplicateClOrdId { cl_ord_id } => {
                write!(f, "duplicate cl_ord_id: {cl_ord_id}")
            }
            OrderTrackerError::UnknownClOrdId { cl_ord_id } => {
                write!(f, "unknown cl_ord_id: {cl_ord_id}")
            }
            OrderTrackerError::UnknownOrderId { order_id } => {
                write!(f, "unknown order_id: {order_id}")
            }
            OrderTrackerError::OrderIdMismatch {
                order_id,
                expected_cl_ord_id,
                actual_cl_ord_id,
            } => write!(
                f,
                "order_id {order_id} mapped to {expected_cl_ord_id}, got {actual_cl_ord_id}"
            ),
            OrderTrackerError::OrderIdConflict {
                cl_ord_id,
                existing_order_id,
                new_order_id,
            } => write!(
                f,
                "order_id conflict for {cl_ord_id}: {existing_order_id} vs {new_order_id}"
            ),
            OrderTrackerError::InvalidTransition {
                cl_ord_id,
                from,
                to,
            } => {
                write!(f, "invalid transition for {cl_ord_id}: {from} -> {to}")
            }
            OrderTrackerError::InvalidDecimal { field, value } => {
                write!(f, "invalid decimal for {field}: {value}")
            }
            OrderTrackerError::UnknownOrderStatus { value } => {
                write!(f, "unknown order status: {value}")
            }
        }
    }
}

impl std::error::Error for OrderTrackerError {}

/// Summary of reconciliation actions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReconcileSummary {
    pub inserted: usize,
    pub updated: usize,
    pub missing_failed: usize,
}

/// Order tracker that manages all orders for a task.
#[derive(Debug)]
pub struct OrderTracker {
    orders: HashMap<String, TrackedOrder>,
    order_id_index: HashMap<i64, String>,
    timeout: Duration,
}

impl OrderTracker {
    /// Create a new order tracker with a default timeout.
    pub fn new() -> Self {
        Self::with_timeout(Duration::from_secs(10))
    }

    /// Create a new order tracker with a custom timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            orders: HashMap::new(),
            order_id_index: HashMap::new(),
            timeout,
        }
    }

    /// Returns the number of tracked orders.
    pub fn len(&self) -> usize {
        self.orders.len()
    }

    /// Returns true when no orders are tracked.
    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    /// Returns a tracked order by client order id.
    pub fn get(&self, cl_ord_id: &str) -> Option<&TrackedOrder> {
        self.orders.get(cl_ord_id)
    }

    /// Returns the current state for a client order id.
    pub fn state(&self, cl_ord_id: &str) -> Option<&OrderState> {
        self.orders.get(cl_ord_id).map(|order| &order.state)
    }

    /// Register a new pending order, enforcing idempotency on cl_ord_id.
    pub fn register_pending(
        &mut self,
        cl_ord_id: String,
        qty: Decimal,
        now: Instant,
    ) -> Result<(), OrderTrackerError> {
        if self.orders.contains_key(&cl_ord_id) {
            return Err(OrderTrackerError::DuplicateClOrdId { cl_ord_id });
        }

        let tracked = TrackedOrder {
            cl_ord_id: cl_ord_id.clone(),
            order_id: None,
            total_qty: qty,
            filled_qty: Decimal::ZERO,
            state: OrderState::Pending { created_at: now },
        };

        self.orders.insert(cl_ord_id, tracked);
        Ok(())
    }

    /// Mark a pending order as sent.
    pub fn mark_sent(&mut self, cl_ord_id: &str, now: Instant) -> Result<(), OrderTrackerError> {
        let tracked =
            self.orders
                .get_mut(cl_ord_id)
                .ok_or_else(|| OrderTrackerError::UnknownClOrdId {
                    cl_ord_id: cl_ord_id.to_string(),
                })?;

        match &tracked.state {
            OrderState::Pending { .. } => {
                tracked.state = OrderState::Sent {
                    sent_at: now,
                    cl_ord_id: cl_ord_id.to_string(),
                };
                Ok(())
            }
            OrderState::Sent { .. } => Ok(()),
            _ => Err(OrderTrackerError::InvalidTransition {
                cl_ord_id: cl_ord_id.to_string(),
                from: state_name(&tracked.state),
                to: "Sent",
            }),
        }
    }

    /// Acknowledge an order with the exchange order id.
    pub fn acknowledge(
        &mut self,
        cl_ord_id: &str,
        order_id: i64,
        now: Instant,
    ) -> Result<(), OrderTrackerError> {
        if !self.orders.contains_key(cl_ord_id) {
            return Err(OrderTrackerError::UnknownClOrdId {
                cl_ord_id: cl_ord_id.to_string(),
            });
        }

        self.index_order_id(order_id, cl_ord_id)?;

        let tracked =
            self.orders
                .get_mut(cl_ord_id)
                .ok_or_else(|| OrderTrackerError::UnknownClOrdId {
                    cl_ord_id: cl_ord_id.to_string(),
                })?;

        if let Some(existing_id) = tracked.order_id && existing_id != order_id {
            return Err(OrderTrackerError::OrderIdConflict {
                cl_ord_id: cl_ord_id.to_string(),
                existing_order_id: existing_id,
                new_order_id: order_id,
            });
        }

        tracked.order_id = Some(order_id);

        match &tracked.state {
            OrderState::Pending { .. }
            | OrderState::Sent { .. }
            | OrderState::Acknowledged { .. } => {
                tracked.state = OrderState::Acknowledged {
                    order_id,
                    acked_at: now,
                };
            }
            _ => {}
        }

        Ok(())
    }

    /// Mark an order as cancelling after a cancel request is sent.
    pub fn mark_cancelling(
        &mut self,
        cl_ord_id: &str,
        now: Instant,
    ) -> Result<(), OrderTrackerError> {
        let tracked =
            self.orders
                .get_mut(cl_ord_id)
                .ok_or_else(|| OrderTrackerError::UnknownClOrdId {
                    cl_ord_id: cl_ord_id.to_string(),
                })?;

        match &tracked.state {
            OrderState::Acknowledged { .. }
            | OrderState::PartiallyFilled { .. }
            | OrderState::Sent { .. } => {
                tracked.state = OrderState::Cancelling {
                    cancel_sent_at: now,
                };
                Ok(())
            }
            OrderState::Cancelling { .. } => Ok(()),
            _ => Err(OrderTrackerError::InvalidTransition {
                cl_ord_id: cl_ord_id.to_string(),
                from: state_name(&tracked.state),
                to: "Cancelling",
            }),
        }
    }

    /// Mark an order as failed with a reason.
    pub fn mark_failed(
        &mut self,
        cl_ord_id: &str,
        error: impl Into<String>,
    ) -> Result<(), OrderTrackerError> {
        let tracked =
            self.orders
                .get_mut(cl_ord_id)
                .ok_or_else(|| OrderTrackerError::UnknownClOrdId {
                    cl_ord_id: cl_ord_id.to_string(),
                })?;

        if matches!(tracked.state, OrderState::Failed { .. }) {
            return Ok(());
        }

        if tracked.state.is_terminal() {
            return Err(OrderTrackerError::InvalidTransition {
                cl_ord_id: cl_ord_id.to_string(),
                from: state_name(&tracked.state),
                to: "Failed",
            });
        }

        tracked.state = OrderState::Failed {
            error: error.into(),
        };
        Ok(())
    }

    /// Handle a WebSocket order update and update local state.
    pub fn handle_ws_update(
        &mut self,
        update: &OrderUpdateData,
        now: Instant,
    ) -> Result<OrderState, OrderTrackerError> {
        let status = parse_order_status(&update.status)?;
        let total_qty = parse_decimal("qty", &update.qty)?;
        let filled_qty = parse_decimal("fill_qty", &update.fill_qty)?;
        let cl_ord_id = self.resolve_cl_ord_id(update.id)?;

        self.index_order_id(update.id, &cl_ord_id)?;

        let tracked =
            self.orders
                .get_mut(&cl_ord_id)
                .ok_or_else(|| OrderTrackerError::UnknownClOrdId {
                    cl_ord_id: cl_ord_id.clone(),
                })?;

        if let Some(existing_id) = tracked.order_id {
            if existing_id != update.id {
                return Err(OrderTrackerError::OrderIdConflict {
                    cl_ord_id: cl_ord_id.clone(),
                    existing_order_id: existing_id,
                    new_order_id: update.id,
                });
            }
        } else {
            tracked.order_id = Some(update.id);
        }

        tracked.total_qty = total_qty;
        tracked.filled_qty = filled_qty;

        if tracked.state.is_terminal() {
            return Ok(tracked.state.clone());
        }

        let remaining_qty = remaining_qty(total_qty, filled_qty);

        let next_state = match status {
            OrderStatus::Filled => OrderState::Filled { filled_at: now },
            OrderStatus::Cancelled => OrderState::Cancelled { cancelled_at: now },
            OrderStatus::Rejected => OrderState::Failed {
                error: "rejected".to_string(),
            },
            OrderStatus::PartiallyFilled => OrderState::PartiallyFilled {
                filled_qty,
                remaining_qty,
            },
            OrderStatus::New | OrderStatus::Open | OrderStatus::Untriggered => {
                if filled_qty > Decimal::ZERO {
                    OrderState::PartiallyFilled {
                        filled_qty,
                        remaining_qty,
                    }
                } else {
                    OrderState::Acknowledged {
                        order_id: update.id,
                        acked_at: now,
                    }
                }
            }
        };

        tracked.state = next_state.clone();
        Ok(next_state)
    }

    /// Reconcile local orders with exchange snapshots.
    pub fn reconcile_with_exchange(
        &mut self,
        exchange_orders: &[Order],
        now: Instant,
    ) -> Result<ReconcileSummary, OrderTrackerError> {
        let mut summary = ReconcileSummary::default();
        let mut seen = HashSet::new();

        for order in exchange_orders {
            let cl_ord_id = order.cl_ord_id.clone();
            let total_qty = order.qty;
            let filled_qty = order.fill_qty;

            seen.insert(cl_ord_id.clone());
            self.index_order_id(order.id, &cl_ord_id)?;

            let next_state = state_from_exchange(order, now);

            match self.orders.get_mut(&cl_ord_id) {
                Some(tracked) => {
                    tracked.order_id = Some(order.id);
                    tracked.total_qty = total_qty;
                    tracked.filled_qty = filled_qty;

                    if !tracked.state.is_terminal() || next_state.is_terminal() {
                        tracked.state = next_state;
                    }

                    summary.updated += 1;
                }
                None => {
                    let tracked = TrackedOrder {
                        cl_ord_id: cl_ord_id.clone(),
                        order_id: Some(order.id),
                        total_qty,
                        filled_qty,
                        state: next_state,
                    };
                    self.orders.insert(cl_ord_id, tracked);
                    summary.inserted += 1;
                }
            }
        }

        for (cl_ord_id, tracked) in self.orders.iter_mut() {
            if !seen.contains(cl_ord_id) && !tracked.state.is_terminal() {
                tracked.state = OrderState::Failed {
                    error: "missing_on_exchange".to_string(),
                };
                summary.missing_failed += 1;
            }
        }

        Ok(summary)
    }

    /// Detect and mark timeouts for sent orders.
    pub fn check_timeouts(&mut self, now: Instant) -> Vec<String> {
        let mut timed_out = Vec::new();

        for (cl_ord_id, tracked) in self.orders.iter_mut() {
            if let OrderState::Sent { sent_at, .. } = tracked.state
                && now.saturating_duration_since(sent_at) > self.timeout {
                    tracked.state = OrderState::Failed {
                        error: "send_timeout".to_string(),
                    };
                    timed_out.push(cl_ord_id.clone());
            }
        }

        timed_out
    }

    fn index_order_id(&mut self, order_id: i64, cl_ord_id: &str) -> Result<(), OrderTrackerError> {
        if let Some(existing) = self.order_id_index.get(&order_id) {
            if existing != cl_ord_id {
                return Err(OrderTrackerError::OrderIdMismatch {
                    order_id,
                    expected_cl_ord_id: existing.clone(),
                    actual_cl_ord_id: cl_ord_id.to_string(),
                });
            }
            return Ok(());
        }

        self.order_id_index.insert(order_id, cl_ord_id.to_string());

        Ok(())
    }

    fn resolve_cl_ord_id(&mut self, order_id: i64) -> Result<String, OrderTrackerError> {
        if let Some(cl_ord_id) = self.order_id_index.get(&order_id) {
            return Ok(cl_ord_id.clone());
        }

        if let Some((cl_ord_id, _)) = self
            .orders
            .iter()
            .find(|(_, order)| order.order_id == Some(order_id))
        {
            let cl_ord_id = cl_ord_id.clone();
            self.order_id_index.insert(order_id, cl_ord_id.clone());
            return Ok(cl_ord_id);
        }

        Err(OrderTrackerError::UnknownOrderId { order_id })
    }
}

impl Default for OrderTracker {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_decimal(field: &'static str, value: &str) -> Result<Decimal, OrderTrackerError> {
    Decimal::from_str(value).map_err(|_| OrderTrackerError::InvalidDecimal {
        field,
        value: value.to_string(),
    })
}

fn parse_order_status(value: &str) -> Result<OrderStatus, OrderTrackerError> {
    match value.to_ascii_lowercase().as_str() {
        "new" => Ok(OrderStatus::New),
        "open" => Ok(OrderStatus::Open),
        "filled" => Ok(OrderStatus::Filled),
        "partially_filled" | "partial_filled" => Ok(OrderStatus::PartiallyFilled),
        "canceled" | "cancelled" => Ok(OrderStatus::Cancelled),
        "rejected" => Ok(OrderStatus::Rejected),
        "untriggered" => Ok(OrderStatus::Untriggered),
        _ => Err(OrderTrackerError::UnknownOrderStatus {
            value: value.to_string(),
        }),
    }
}

fn state_from_exchange(order: &Order, now: Instant) -> OrderState {
    let filled_qty = order.fill_qty;
    let total_qty = order.qty;
    let remaining_qty = remaining_qty(total_qty, filled_qty);

    match order.status {
        OrderStatus::Filled => OrderState::Filled { filled_at: now },
        OrderStatus::Cancelled => OrderState::Cancelled { cancelled_at: now },
        OrderStatus::Rejected => OrderState::Failed {
            error: "rejected".to_string(),
        },
        OrderStatus::PartiallyFilled => OrderState::PartiallyFilled {
            filled_qty,
            remaining_qty,
        },
        OrderStatus::New | OrderStatus::Open | OrderStatus::Untriggered => {
            if filled_qty > Decimal::ZERO {
                OrderState::PartiallyFilled {
                    filled_qty,
                    remaining_qty,
                }
            } else {
                OrderState::Acknowledged {
                    order_id: order.id,
                    acked_at: now,
                }
            }
        }
    }
}

fn remaining_qty(total_qty: Decimal, filled_qty: Decimal) -> Decimal {
    if filled_qty >= total_qty {
        Decimal::ZERO
    } else {
        total_qty - filled_qty
    }
}

fn state_name(state: &OrderState) -> &'static str {
    match state {
        OrderState::Pending { .. } => "Pending",
        OrderState::Sent { .. } => "Sent",
        OrderState::Acknowledged { .. } => "Acknowledged",
        OrderState::PartiallyFilled { .. } => "PartiallyFilled",
        OrderState::Filled { .. } => "Filled",
        OrderState::Cancelling { .. } => "Cancelling",
        OrderState::Cancelled { .. } => "Cancelled",
        OrderState::Failed { .. } => "Failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use standx_point_adapter::types::enums::{OrderStatus, OrderType, Side, TimeInForce};
    use standx_point_adapter::types::models::Order;
    use std::time::{Duration, Instant};

    fn decimal(value: &str) -> Decimal {
        Decimal::from_str(value).expect("valid decimal")
    }

    fn make_order(
        cl_ord_id: &str,
        id: i64,
        status: OrderStatus,
        qty: Decimal,
        fill_qty: Decimal,
    ) -> Order {
        Order {
            avail_locked: Decimal::ZERO,
            cl_ord_id: cl_ord_id.to_string(),
            closed_block: 0,
            created_at: "2026-02-03T00:00:00Z".to_string(),
            created_block: 0,
            fill_avg_price: Decimal::ZERO,
            fill_qty,
            id,
            leverage: Decimal::ONE,
            liq_id: 0,
            margin: Decimal::ZERO,
            order_type: OrderType::Limit,
            payload: None,
            position_id: 0,
            price: Some(decimal("1")),
            qty,
            reduce_only: false,
            remark: String::new(),
            side: Side::Buy,
            source: "test".to_string(),
            status,
            symbol: "BTCUSDT".to_string(),
            time_in_force: TimeInForce::Gtc,
            updated_at: "2026-02-03T00:00:00Z".to_string(),
            user: "tester".to_string(),
        }
    }

    #[test]
    fn rejects_duplicate_cl_ord_id() {
        let now = Instant::now();
        let mut tracker = OrderTracker::new();

        tracker
            .register_pending("order-1".to_string(), decimal("1"), now)
            .expect("register pending");

        let err = tracker
            .register_pending("order-1".to_string(), decimal("1"), now)
            .expect_err("duplicate cl_ord_id");

        assert!(matches!(err, OrderTrackerError::DuplicateClOrdId { .. }));
    }

    #[test]
    fn sent_timeout_marks_failed() {
        let now = Instant::now();
        let mut tracker = OrderTracker::with_timeout(Duration::from_secs(1));

        tracker
            .register_pending("order-1".to_string(), decimal("1"), now)
            .expect("register pending");
        tracker.mark_sent("order-1", now).expect("mark sent");

        let timed_out = tracker.check_timeouts(now + Duration::from_secs(2));

        assert_eq!(timed_out, vec!["order-1".to_string()]);
        assert!(matches!(
            tracker.state("order-1"),
            Some(OrderState::Failed { .. })
        ));
    }

    #[test]
    fn ws_update_correlates_and_updates_state() {
        let now = Instant::now();
        let mut tracker = OrderTracker::new();

        tracker
            .register_pending("order-1".to_string(), decimal("10"), now)
            .expect("register pending");
        tracker.mark_sent("order-1", now).expect("mark sent");
        tracker
            .acknowledge("order-1", 42, now)
            .expect("acknowledge");

        let update = OrderUpdateData {
            id: 42,
            symbol: "BTCUSDT".to_string(),
            side: "buy".to_string(),
            status: "partially_filled".to_string(),
            qty: "10".to_string(),
            fill_qty: "4".to_string(),
            price: "1".to_string(),
            order_type: "limit".to_string(),
        };

        tracker
            .handle_ws_update(&update, now + Duration::from_secs(1))
            .expect("handle ws update");

        match tracker.state("order-1") {
            Some(OrderState::PartiallyFilled {
                filled_qty,
                remaining_qty,
            }) => {
                assert_eq!(*filled_qty, decimal("4"));
                assert_eq!(*remaining_qty, decimal("6"));
            }
            other => panic!("unexpected state: {other:?}"),
        }
    }

    #[test]
    fn reconcile_updates_and_marks_missing_orders() {
        let now = Instant::now();
        let mut tracker = OrderTracker::new();

        tracker
            .register_pending("local-filled".to_string(), decimal("1"), now)
            .expect("register pending");
        tracker.mark_sent("local-filled", now).expect("mark sent");

        tracker
            .register_pending("missing".to_string(), decimal("1"), now)
            .expect("register pending");
        tracker.mark_sent("missing", now).expect("mark sent");

        let exchange_orders = vec![
            make_order(
                "local-filled",
                7,
                OrderStatus::Filled,
                decimal("1"),
                decimal("1"),
            ),
            make_order(
                "new-order",
                8,
                OrderStatus::Open,
                decimal("2"),
                decimal("0"),
            ),
        ];

        let summary = tracker
            .reconcile_with_exchange(&exchange_orders, now)
            .expect("reconcile");

        assert_eq!(summary.inserted, 1);
        assert_eq!(summary.updated, 1);
        assert_eq!(summary.missing_failed, 1);

        assert!(matches!(
            tracker.state("local-filled"),
            Some(OrderState::Filled { .. })
        ));
        assert!(matches!(
            tracker.state("missing"),
            Some(OrderState::Failed { .. })
        ));
        assert!(matches!(
            tracker.state("new-order"),
            Some(OrderState::Acknowledged { .. })
        ));
    }
}
