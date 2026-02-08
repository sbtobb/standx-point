/*
[INPUT]:  CLI arguments, YAML configuration file, OS shutdown signals
[OUTPUT]: Running market making tasks with graceful shutdown
[POS]:    Binary entry point
[UPDATE]: When changing CLI flags, startup flow, or shutdown handling
[UPDATE]: 2026-02-05 Configure tracing to log to daily files only
[UPDATE]: 2026-02-08 Remove TUI runtime and keep CLI-only entry
[UPDATE]: 2026-02-08 Add environment-variable startup path
*/

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use std::fs;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tracing_appender::rolling;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

mod cli;
mod state;

use standx_point_adapter::auth::{EvmWalletSigner, SolanaWalletSigner};
use standx_point_adapter::http::StandxClient;
use standx_point_adapter::Chain;
use standx_point_mm_strategy::{MarketDataHub, StrategyConfig, TaskManager};

#[derive(Parser, Debug)]
#[command(
    name = "standx-point-mm-strategy",
    version,
    about = "StandX market making strategy runner"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(short, long, value_name = "PATH")]
    config: Option<PathBuf>,
    #[arg(long, help = "Load configuration from environment variables")]
    env: bool,
    #[arg(short, long, value_name = "LEVEL", default_value = "info")]
    log_level: String,
    #[arg(long)]
    dry_run: bool,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    Init {
        #[arg(short, long)]
        output: PathBuf,
    },
    Migrate,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    if let Some(Commands::Init { output }) = args.command {
        init_tracing(&args.log_level)?;
        return cli::init::run_init(output);
    }

    if let Some(Commands::Migrate) = args.command {
        init_tracing(&args.log_level)?;
        return run_migrations().await;
    }

    init_tracing(&args.log_level)?;

    run_cli_mode(args.config, args.env, args.dry_run).await
}

async fn run_migrations() -> Result<()> {
    let storage = state::storage::Storage::new().await?;
    let client = StandxClient::new()
        .map_err(|err| anyhow!("create StandxClient for migration failed: {err}"))?;
    let auth = standx_point_adapter::auth::AuthManager::new(client);

    let key_count = auth.list_stored_accounts().len();
    let account_count = storage.list_accounts().await?.len();
    let task_count = storage.list_tasks().await?.len();

    info!(key_count, account_count, task_count, "migration complete");

    Ok(())
}

async fn run_cli_mode(config_path: Option<PathBuf>, env_mode: bool, dry_run: bool) -> Result<()> {
    if let Some(path) = &config_path {
        info!(
            config_path = %path.display(),
            dry_run = dry_run,
            "starting standx-mm-strategy (CLI mode)"
        );
    } else {
        info!(dry_run = dry_run, "starting standx-mm-strategy (CLI mode)");
    }

    let config = match config_path {
        Some(path) => {
            let config = load_config(&path)?;
            info!(task_count = config.tasks.len(), "configuration loaded");
            config
        }
        None => {
            if env_mode {
                match load_env_config()? {
                    Some(config) => {
                        info!(task_count = config.tasks.len(), "configuration loaded from env");
                        config
                    }
                    None => return Ok(()),
                }
            } else {
                match cli::interactive::run_interactive().await? {
                    Some(config) => config,
                    None => return Ok(()),
                }
            }
        }
    };

    validate_strategy_config(&config)?;
    log_strategy_config(&config);

    if dry_run {
        info!("dry-run requested; configuration validated");
        return Ok(());
    }

    let market_data_hub = Arc::new(Mutex::new(MarketDataHub::new()));
    let mut task_manager = TaskManager::with_market_data_hub(market_data_hub.clone());

    let shutdown = task_manager.shutdown_token();
    setup_signal_handlers(shutdown.clone());

    info!("spawning tasks");
    task_manager
        .spawn_from_config(config)
        .await
        .context("spawn tasks from config")?;
    info!("tasks started");

    shutdown.cancelled().await;
    info!("shutdown signal received");

    task_manager
        .shutdown_and_wait()
        .await
        .context("shutdown tasks")?;
    info!("tasks shutdown complete");

    let hub = market_data_hub.lock().await;
    hub.shutdown();
    info!("market data hub shutdown complete");

    Ok(())
}

fn init_tracing(log_level: &str) -> Result<()> {
    let filter = EnvFilter::try_new(log_level).context("invalid log level")?;
    let log_dir = std::env::current_dir()
        .context("resolve current directory")?
        .join("logs");
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("create log directory {}", log_dir.display()))?;
    let file_appender = rolling::daily(&log_dir, "standx-point-mm-strategy.log");
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_filter(filter.clone());
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_filter(filter);
    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .try_init()
        .map_err(|err| anyhow!(err))
        .context("initialize tracing subscriber")?;
    Ok(())
}

fn load_config(path: &Path) -> Result<StrategyConfig> {
    let path_str = path.to_str().context("config path must be valid utf-8")?;
    StrategyConfig::from_file(path_str).context("load config")
}

fn validate_strategy_config(config: &StrategyConfig) -> Result<()> {
    if config.accounts.is_empty() {
        return Err(anyhow!("strategy config must contain at least one account"));
    }
    if config.tasks.is_empty() {
        return Err(anyhow!("strategy config must contain at least one task"));
    }

    let mut seen_accounts = std::collections::HashSet::new();
    let mut account_ids = std::collections::HashSet::new();
    for account in &config.accounts {
        if account.id.trim().is_empty() {
            return Err(anyhow!("account id cannot be empty"));
        }
        let private_key = account
            .private_key
            .as_deref()
            .unwrap_or("")
            .trim();
        let jwt_token = account.jwt_token.as_deref().unwrap_or("").trim();
        let signing_key = account.signing_key.as_deref().unwrap_or("").trim();

        let has_private_key = !private_key.is_empty();
        let has_jwt = !jwt_token.is_empty();
        let has_signing = !signing_key.is_empty();

        if !has_private_key && (!has_jwt || !has_signing) {
            return Err(anyhow!(
                "account must provide private_key or jwt_token+signing_key"
            ));
        }
        if has_jwt && !has_signing {
            return Err(anyhow!("account signing_key cannot be empty when jwt_token is set"));
        }
        if has_signing && !has_jwt {
            return Err(anyhow!("account jwt_token cannot be empty when signing_key is set"));
        }
        if !seen_accounts.insert(account.id.clone()) {
            return Err(anyhow!("duplicate account id in config: {}", account.id));
        }
        account_ids.insert(account.id.clone());
    }

    let mut seen_ids = std::collections::HashSet::new();
    for task in &config.tasks {
        if task.id.trim().is_empty() {
            return Err(anyhow!("task id cannot be empty"));
        }
        if task.symbol.trim().is_empty() {
            return Err(anyhow!("task symbol cannot be empty"));
        }
        if task.account_id.trim().is_empty() {
            return Err(anyhow!("task account_id cannot be empty"));
        }
        if !account_ids.contains(&task.account_id) {
            return Err(anyhow!("task account_id not found: {}", task.account_id));
        }
        if task.risk.level.trim().is_empty() {
            return Err(anyhow!("task risk.level cannot be empty"));
        }
        if task.risk.budget_usd.trim().is_empty() {
            return Err(anyhow!("task risk.budget_usd cannot be empty"));
        }
        if !seen_ids.insert(task.id.clone()) {
            return Err(anyhow!("duplicate task id in config: {}", task.id));
        }
    }
    Ok(())
}

fn load_env_config() -> Result<Option<StrategyConfig>> {
    let private_key = env::var("STANDX_MM_PRIVATE_KEY").ok();
    let symbol = env::var("STANDX_MM_SYMBOL").ok();
    let risk_level = env::var("STANDX_MM_RISK_LEVEL").ok();
    let budget_usd = env::var("STANDX_MM_BUDGET_USD").ok();

    let any_set = private_key.is_some() || symbol.is_some() || risk_level.is_some() || budget_usd.is_some();
    if !any_set {
        return Ok(None);
    }

    let private_key = private_key.ok_or_else(|| anyhow!("STANDX_MM_PRIVATE_KEY is required"))?;
    let symbol = symbol.ok_or_else(|| anyhow!("STANDX_MM_SYMBOL is required"))?;
    let risk_level = risk_level.ok_or_else(|| anyhow!("STANDX_MM_RISK_LEVEL is required"))?;
    let budget_usd = budget_usd.ok_or_else(|| anyhow!("STANDX_MM_BUDGET_USD is required"))?;

    let chain = parse_chain(env::var("STANDX_MM_CHAIN").ok())?;
    let wallet_address = derive_wallet_address(&private_key, chain)?;

    let account_id = env::var("STANDX_MM_ACCOUNT_ID").unwrap_or(wallet_address);
    let task_id = env::var("STANDX_MM_TASK_ID").unwrap_or_else(|_| format!(
        "task-{}",
        slugify_symbol(&symbol)
    ));

    let config = StrategyConfig {
        accounts: vec![standx_point_mm_strategy::config::AccountConfig {
            id: account_id.clone(),
            private_key: Some(private_key),
            jwt_token: None,
            signing_key: None,
            chain,
        }],
        tasks: vec![standx_point_mm_strategy::config::TaskConfig {
            id: task_id,
            symbol,
            account_id,
            risk: standx_point_mm_strategy::config::RiskConfig {
                level: risk_level,
                budget_usd,
            },
        }],
    };

    Ok(Some(config))
}

fn parse_chain(raw: Option<String>) -> Result<Chain> {
    let raw = raw.unwrap_or_else(|| "bsc".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "bsc" => Ok(Chain::Bsc),
        "solana" => Ok(Chain::Solana),
        other => Err(anyhow!("invalid STANDX_MM_CHAIN: {other} (use bsc or solana)")),
    }
}

fn derive_wallet_address(private_key: &str, chain: Chain) -> Result<String> {
    match chain {
        Chain::Bsc => {
            let wallet = EvmWalletSigner::new(private_key)
                .map_err(|err| anyhow!("invalid EVM private key: {err}"))?;
            Ok(wallet.address().to_string())
        }
        Chain::Solana => {
            let wallet = SolanaWalletSigner::new(private_key)
                .map_err(|err| anyhow!("invalid Solana private key: {err}"))?;
            Ok(wallet.address().to_string())
        }
    }
}

fn slugify_symbol(symbol: &str) -> String {
    let mut slug = String::with_capacity(symbol.len());
    for ch in symbol.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            slug.push('-');
        }
    }
    if slug.is_empty() {
        "task".to_string()
    } else {
        slug
    }
}

fn log_strategy_config(config: &StrategyConfig) {
    info!(
        task_count = config.tasks.len(),
        "strategy configuration confirmed"
    );
    for task in &config.tasks {
        info!(
            task_id = %task.id,
            symbol = %task.symbol,
            account_id = %task.account_id,
            risk_level = %task.risk.level,
            budget_usd = %task.risk.budget_usd,
            "strategy task parameters"
        );
    }
}

fn setup_signal_handlers(shutdown: CancellationToken) {
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        if let Err(err) = tokio::signal::ctrl_c().await {
            warn!(error = %err, "failed to install SIGINT handler");
            return;
        }
        info!("received SIGINT");
        shutdown_clone.cancel();
    });

    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let shutdown_clone = shutdown.clone();
        tokio::spawn(async move {
            match signal(SignalKind::terminate()) {
                Ok(mut stream) => {
                    stream.recv().await;
                    info!("received SIGTERM");
                    shutdown_clone.cancel();
                }
                Err(err) => {
                    warn!(error = %err, "failed to install SIGTERM handler");
                }
            }
        });
    }
}
