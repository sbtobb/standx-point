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

use standx_point_mm_strategy::config::{
    CredentialsConfig, RiskConfig, SizingConfig, StrategyConfig, TaskConfig,
};

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

    println!("\n{}", style("--- Credentials ---").bold());
    let jwt_token: String = Input::with_theme(&theme)
        .with_prompt("JWT Token")
        .interact_text()?;

    let signing_key: String = Input::with_theme(&theme)
        .with_prompt("Signing Key (Base64 Ed25519 private key)")
        .interact_text()?;

    println!("\n{}", style("--- Risk Management ---").bold());
    let risk_levels = vec!["conservative", "moderate", "aggressive"];
    let risk_selection = Select::with_theme(&theme)
        .with_prompt("Risk Level")
        .items(&risk_levels)
        .default(0)
        .interact()?;
    let risk_level = risk_levels[risk_selection].to_string();

    let max_position_usd: String = Input::with_theme(&theme)
        .with_prompt("Max Position (USD)")
        .default("50000".to_string())
        .interact_text()?;

    let price_jump_threshold_bps: u32 = Input::with_theme(&theme)
        .with_prompt("Price Jump Threshold (bps/sec)")
        .default(5)
        .interact_text()?;

    println!("\n{}", style("--- Order Sizing ---").bold());
    let base_qty: String = Input::with_theme(&theme)
        .with_prompt("Base Order Quantity")
        .default("0.1".to_string())
        .interact_text()?;

    let tiers: u8 = Input::with_theme(&theme)
        .with_prompt("Number of Tiers (1-3)")
        .default(2)
        .validate_with(|input: &u8| -> Result<(), &str> {
            if *input >= 1 && *input <= 3 {
                Ok(())
            } else {
                Err("Tiers must be between 1 and 3")
            }
        })
        .interact_text()?;

    let config = StrategyConfig {
        tasks: vec![TaskConfig {
            id,
            symbol,
            credentials: CredentialsConfig {
                jwt_token,
                signing_key,
            },
            risk: RiskConfig {
                level: risk_level,
                max_position_usd,
                price_jump_threshold_bps,
            },
            sizing: SizingConfig { base_qty, tiers },
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
