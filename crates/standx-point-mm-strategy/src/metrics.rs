/*
[INPUT]:  Runtime task updates (orders, positions, heartbeat, price)
[OUTPUT]: Snapshot-friendly task metrics for UI display
[POS]:    Shared runtime metrics between task loops and UI
[UPDATE]: When adding/removing task-level runtime signals
*/

use rust_decimal::Decimal;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct TaskMetricsSnapshot {
    pub open_orders: usize,
    pub position_qty: Decimal,
    pub last_heartbeat: Option<Instant>,
    pub last_price: Option<Decimal>,
    pub last_update: Option<Instant>,
}

#[derive(Debug, Default)]
pub struct TaskMetrics {
    open_orders: usize,
    position_qty: Decimal,
    last_heartbeat: Option<Instant>,
    last_price: Option<Decimal>,
    last_update: Option<Instant>,
}

impl TaskMetrics {
    pub fn snapshot(&self) -> TaskMetricsSnapshot {
        TaskMetricsSnapshot {
            open_orders: self.open_orders,
            position_qty: self.position_qty,
            last_heartbeat: self.last_heartbeat,
            last_price: self.last_price,
            last_update: self.last_update,
        }
    }

    pub fn record_open_orders(&mut self, open_orders: usize) {
        self.open_orders = open_orders;
        self.last_update = Some(Instant::now());
    }

    pub fn record_position_qty(&mut self, position_qty: Decimal) {
        self.position_qty = position_qty;
        self.last_update = Some(Instant::now());
    }

    pub fn record_heartbeat(&mut self) {
        self.last_heartbeat = Some(Instant::now());
        self.last_update = Some(Instant::now());
    }

    pub fn record_price(&mut self, price: Decimal) {
        self.last_price = Some(price);
        self.last_update = Some(Instant::now());
    }
}
