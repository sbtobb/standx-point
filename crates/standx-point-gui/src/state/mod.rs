/*
[INPUT]:  Core domain types from adapter and strategy config
[OUTPUT]: Global application state and runtime models for GUI
[POS]:    State layer - central data store for GPUI components
[UPDATE]: When new state fields or task status transitions are needed
*/

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use standx_point_adapter::types::{Chain, SymbolPrice};
use standx_point_mm_strategy::config::TaskConfig;
use std::collections::HashMap;

/// Global application state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    /// Connected accounts (Max 5 in MVP)
    pub accounts: Vec<Account>,
    /// Configured trading tasks
    pub tasks: Vec<Task>,
    /// Real-time price information indexed by symbol
    pub prices: HashMap<String, PriceData>,
}

/// Account with identification and chain info.
/// Credentials are stored encrypted elsewhere; this struct holds metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account {
    pub id: String,
    pub address: String,
    pub alias: String,
    pub chain: Chain,
}

/// Task configuration and runtime status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub symbol: String,
    pub config: TaskConfig,
    pub status: TaskStatus,
}

/// Runtime status of a trading task.
///
/// Transitions:
/// - Draft -> Pending (on save)
/// - Pending -> Running (on start)
/// - Running -> Paused/Stopped/Failed
/// - Paused -> Running/Stopped
/// - Any -> Draft (on edit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Draft,
    Pending,
    Running,
    Paused,
    Stopped,
    Failed,
}

/// Real-time price information for a symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub symbol: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub mark_price: Decimal,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub last_price: Option<Decimal>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub index_price: Option<Decimal>,
    pub updated_at: i64,
}

impl From<SymbolPrice> for PriceData {
    fn from(sp: SymbolPrice) -> Self {
        Self {
            symbol: sp.symbol,
            mark_price: sp.mark_price,
            last_price: sp.last_price,
            index_price: Some(sp.index_price),
            // We'll need a way to parse the time string from SymbolPrice if needed,
            // but for now we'll use 0 or handle it in the state update logic.
            updated_at: 0,
        }
    }
}

pub use standx_point_adapter::types::{
    Balance, MarginMode, Order, OrderStatus, OrderType, Position, Side, Trade,
};
