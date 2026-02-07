/*
[INPUT]:  YAML configuration file
[OUTPUT]: Parsed strategy configuration
[POS]:    Configuration layer - task setup
[UPDATE]: When adding new configuration options
*/

use serde::{Deserialize, Serialize};
use standx_point_adapter::Chain;

/// Top-level configuration for the market making bot
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StrategyConfig {
    /// Account credentials available to tasks
    #[serde(default)]
    pub accounts: Vec<AccountConfig>,
    /// List of trading tasks to run
    pub tasks: Vec<TaskConfig>,
}

/// Account credentials configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountConfig {
    /// Account identifier referenced by tasks
    pub id: String,
    /// JWT token for authentication
    pub jwt_token: String,
    /// Ed25519 private key for body signing (base64 encoded)
    pub signing_key: String,
    /// Chain used for authentication
    #[serde(default = "default_chain")]
    pub chain: Chain,
}

/// Configuration for a single trading task
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskConfig {
    /// Task identifier
    pub id: String,
    /// Trading symbol (e.g., "BTC-USD")
    pub symbol: String,
    /// Account identifier
    pub account_id: String,
    /// Risk parameters
    #[serde(default)]
    pub risk: RiskConfig,
}

/// Risk management configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskConfig {
    /// Risk level: "low", "medium", "high", "xhigh"
    #[serde(default = "default_risk_level")]
    pub level: String,
    /// Budget in USD used for quoting
    #[serde(default = "default_budget_usd", alias = "max_position_usd")]
    pub budget_usd: String,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            level: default_risk_level(),
            budget_usd: default_budget_usd(),
        }
    }
}

fn default_risk_level() -> String {
    "low".to_string()
}

fn default_budget_usd() -> String {
    "50000".to_string()
}

fn default_chain() -> Chain {
    Chain::Bsc
}

impl StrategyConfig {
    /// Load configuration from YAML file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
