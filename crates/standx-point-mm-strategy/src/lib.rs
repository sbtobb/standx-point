/*
[INPUT]:  Public API exports for standx-mm-strategy crate
[OUTPUT]: Module declarations and public re-exports
[POS]:    Crate root - library entry point
[UPDATE]: When adding new modules or public exports
*/

pub mod config;
pub mod market_data;
pub mod order_state;
pub mod risk;
pub mod strategy;
pub mod task;

// Re-export main types for convenience
pub use config::StrategyConfig;
pub use market_data::MarketDataHub;
pub use task::TaskManager;
