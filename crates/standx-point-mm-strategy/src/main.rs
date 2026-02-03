/*
[INPUT]:  CLI arguments, YAML configuration file, OS shutdown signals
[OUTPUT]: Running market making tasks with graceful shutdown
[POS]:    Binary entry point
[UPDATE]: When changing CLI flags, startup flow, or shutdown handling
*/

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use standx_point_mm_strategy::{MarketDataHub, StrategyConfig, TaskManager};

#[derive(Parser, Debug)]
#[command(name = "standx-point-mm-strategy", version, about = "StandX market making strategy runner")]
struct Cli {
    #[arg(long = "config", value_name = "PATH")]
    config_path: PathBuf,
    #[arg(long = "log-level", value_name = "LEVEL", default_value = "info")]
    log_level: String,
    #[arg(long = "dry-run")]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    init_tracing(&args.log_level)?;

    info!(
        config_path = %args.config_path.display(),
        dry_run = args.dry_run,
        "starting standx-mm-strategy"
    );

    let config = load_config(&args.config_path)?;
    info!(task_count = config.tasks.len(), "configuration loaded");

    if args.dry_run {
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
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|err| anyhow!(err))
        .context("initialize tracing subscriber")?;
    Ok(())
}

fn load_config(path: &PathBuf) -> Result<StrategyConfig> {
    let path_str = path
        .to_str()
        .context("config path must be valid utf-8")?;
    StrategyConfig::from_file(path_str).context("load config")
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
        use tokio::signal::unix::{signal, SignalKind};

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
