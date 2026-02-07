/*
[INPUT]:  Interactive user input via CLI
[OUTPUT]: Generated YAML configuration file
[POS]:    CLI initialization layer
[UPDATE]: When StrategyConfig schema changes
*/

use anyhow::{Context, Result};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use std::path::PathBuf;

use standx_point_mm_strategy::config::{AccountConfig, RiskConfig, StrategyConfig, TaskConfig};

pub fn run_init(output: PathBuf) -> Result<()> {
    println!(
        "{}",
        style("Welcome to StandX MM Strategy Init").bold().cyan()
    );
    println!(
        "{}",
        style("This will guide you through creating a new strategy configuration.").dim()
    );

    let theme = ColorfulTheme::default();

    let id: String = Input::with_theme(&theme)
        .with_prompt("Task ID (e.g., btc-mm)")
        .default("btc-mm".to_string())
        .interact_text()?;

    let symbol: String = Input::with_theme(&theme)
        .with_prompt("Trading Symbol (e.g., BTC-USD)")
        .default("BTC-USD".to_string())
        .interact_text()?;

    println!("\n{}", style("--- Account ---").bold());
    let account_id: String = Input::with_theme(&theme)
        .with_prompt("Account ID")
        .default("account-1".to_string())
        .interact_text()?;

    let jwt_token: String = Input::with_theme(&theme)
        .with_prompt("JWT Token")
        .interact_text()?;

    let signing_key: String = Input::with_theme(&theme)
        .with_prompt("Signing Key (Base64 Ed25519 private key)")
        .interact_text()?;

    println!("\n{}", style("--- Risk Management ---").bold());
    let risk_levels = vec!["low", "medium", "high", "xhigh"];
    let risk_selection = Select::with_theme(&theme)
        .with_prompt("Risk Level")
        .items(&risk_levels)
        .default(0)
        .interact()?;
    let risk_level = risk_levels[risk_selection].to_string();

    let budget_usd: String = Input::with_theme(&theme)
        .with_prompt("Budget (USD)")
        .default("50000".to_string())
        .interact_text()?;

    let config = StrategyConfig {
        accounts: vec![AccountConfig {
            id: account_id.clone(),
            jwt_token,
            signing_key,
            chain: standx_point_adapter::Chain::Bsc,
        }],
        tasks: vec![TaskConfig {
            id,
            symbol,
            account_id,
            risk: RiskConfig {
                level: risk_level,
                budget_usd,
            },
        }],
    };

    let yaml = serde_yaml::to_string(&config).context("failed to serialize config to YAML")?;

    std::fs::write(&output, yaml)
        .context(format!("failed to write config to {}", output.display()))?;

    println!("\n{}", style("SUCCESS!").bold().green());
    println!(
        "Configuration written to: {}",
        style(output.display()).cyan()
    );

    Ok(())
}
