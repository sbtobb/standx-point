/*
[INPUT]:  Interactive user input via CLI
[OUTPUT]: Generated YAML configuration file
[POS]:    CLI initialization layer
[UPDATE]: When StrategyConfig schema changes
[UPDATE]: 2026-02-08 Collect wallet private key for auth
*/

use anyhow::{Context, Result};
use console::style;
use dialoguer::{Input, Select, theme::ColorfulTheme};
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

    let chains = vec!["bsc", "solana"];
    let chain_index = Select::with_theme(&theme)
        .with_prompt("Chain")
        .items(&chains)
        .default(0)
        .interact()?;
    let chain = if chain_index == 0 {
        standx_point_adapter::Chain::Bsc
    } else {
        standx_point_adapter::Chain::Solana
    };

    let private_key: String = Input::with_theme(&theme)
        .with_prompt("Wallet Private Key")
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
            private_key: Some(private_key),
            jwt_token: None,
            signing_key: None,
            chain,
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
