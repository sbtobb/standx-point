/*
[INPUT]:  YAML configuration file
[OUTPUT]: Parsed strategy configuration
[POS]:    Configuration layer - task setup
[UPDATE]: When adding new configuration options
*/

use serde::{Deserialize, Serialize};

/// Top-level configuration for the market making bot
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StrategyConfig {
    /// List of trading tasks to run
    pub tasks: Vec<TaskConfig>,
}

/// Configuration for a single trading task
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskConfig {
    /// Task identifier
    pub id: String,
    /// Trading symbol (e.g., "BTC-USD")
    pub symbol: String,
    /// Account credentials
    pub credentials: CredentialsConfig,
    /// Risk parameters
    pub risk: RiskConfig,
    /// Order sizing
    pub sizing: SizingConfig,
}

/// Account credentials configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialsConfig {
    /// JWT token for authentication
    pub jwt_token: String,
    /// Ed25519 private key for body signing (base64 encoded)
    pub signing_key: String,
}

/// Risk management configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskConfig {
    /// Risk level: "conservative", "moderate", "aggressive"
    pub level: String,
    /// Maximum position size in USD
    pub max_position_usd: String,
    /// Price jump threshold in bps/second
    pub price_jump_threshold_bps: u32,
}

/// Order sizing configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SizingConfig {
    /// Base order quantity
    pub base_qty: String,
    /// Number of order tiers (1-3)
    pub tiers: u8,
}

impl StrategyConfig {
    /// Load configuration from YAML file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
