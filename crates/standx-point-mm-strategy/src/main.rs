/*
[INPUT]:  CLI arguments, YAML configuration file, OS shutdown signals
[OUTPUT]: Running market making tasks with graceful shutdown
[POS]:    Binary entry point
[UPDATE]: When changing CLI flags, startup flow, or shutdown handling
[UPDATE]: 2026-02-05 Configure tracing to log to daily files only
[UPDATE]: 2026-02-08 Remove TUI runtime and keep CLI-only entry
*/

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use std::fs;
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

use standx_point_adapter::http::StandxClient;
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

    run_cli_mode(args.config, args.dry_run).await
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

async fn run_cli_mode(config_path: Option<PathBuf>, dry_run: bool) -> Result<()> {
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
        None => match cli::interactive::run_interactive().await? {
            Some(config) => config,
            None => return Ok(()),
        },
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
        if account.jwt_token.trim().is_empty() {
            return Err(anyhow!("account jwt_token cannot be empty"));
        }
        if account.signing_key.trim().is_empty() {
            return Err(anyhow!("account signing_key cannot be empty"));
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
