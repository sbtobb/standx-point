/*
[INPUT]:  CLI arguments, YAML configuration file, OS shutdown signals
[OUTPUT]: Running market making tasks with graceful shutdown
[POS]:    Binary entry point
[UPDATE]: When changing CLI flags, startup flow, or shutdown handling
[UPDATE]: 2026-02-05 Configure tracing to log to daily files only
*/

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use ratatui::crossterm::ExecutableCommand;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::fs;
use std::io::stdout;
use std::path::PathBuf;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tracing_appender::rolling;
use tracing_subscriber::EnvFilter;

mod app;
use crate::app::TICK_RATE;
mod cli;
mod state;
mod ui;

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
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    init_tracing(&args.log_level)?;

    if let Some(Commands::Init { output }) = args.command {
        return cli::init::run_init(output);
    }

    match args.config {
        Some(config_path) => run_cli_mode(config_path, args.dry_run).await,
        None => run_tui_mode().await,
    }
}

async fn run_cli_mode(config_path: PathBuf, dry_run: bool) -> Result<()> {
    info!(
        config_path = %config_path.display(),
        dry_run = dry_run,
        "starting standx-mm-strategy (CLI mode)"
    );

    let config = load_config(&config_path)?;
    info!(task_count = config.tasks.len(), "configuration loaded");

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

async fn run_tui_mode() -> Result<()> {
    info!("starting standx-mm-strategy (TUI mode)");
    let mut app = app::App::new().await?;

    // Check if we're in test mode and skip TUI initialization if needed
    let is_test_mode = std::env::var("STANDX_TUI_TEST_EXIT_AFTER_TICKS").is_ok();
    if is_test_mode && app.auto_exit_after_ticks.is_some() {
        // In test mode, skip TUI rendering and just run the app logic until auto-exit
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(TICK_RATE));
        while !app.should_exit {
            tokio::select! {
                _ = interval.tick() => {
                    app.handle_event(app::event::AppEvent::Tick).await?;
                    app.tick_count += 1;
                    
                    if let Some(n) = app.auto_exit_after_ticks && app.tick_count >= n {
                            app.should_exit = true;
                        }
                }
            }
        }
        return Ok(());
    }

    // Subscribe to price updates for common symbols (only in normal TUI mode)
    let symbols = vec!["BTC-USD", "ETH-USD"];
    {
        let mut hub = app.market_data.lock().await;
        for symbol in &symbols {
            hub.subscribe_price(symbol);
        }
    }
    info!(symbols = ?symbols, "subscribed to price updates for symbols");

    // Normal TUI mode
    enable_raw_mode()?;
    stdout().execute(ratatui::crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(err) => {
            disable_raw_mode()?;
            stdout().execute(ratatui::crossterm::terminal::LeaveAlternateScreen)?;
            return Err(err.into());
        }
    };

    let result = app.run(&mut terminal).await;

    disable_raw_mode()?;
    stdout().execute(ratatui::crossterm::terminal::LeaveAlternateScreen)?;

    result
}

fn init_tracing(log_level: &str) -> Result<()> {
    let filter = EnvFilter::try_new(log_level).context("invalid log level")?;
    let log_dir = std::env::current_dir()
        .context("resolve current directory")?
        .join("logs");
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("create log directory {}", log_dir.display()))?;
    let file_appender = rolling::daily(&log_dir, "standx-point-mm-strategy.log");
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file_appender)
        .with_ansi(false)
        .try_init()
        .map_err(|err| anyhow!(err))
        .context("initialize tracing subscriber")?;
    Ok(())
}

fn load_config(path: &Path) -> Result<StrategyConfig> {
    let path_str = path.to_str().context("config path must be valid utf-8")?;
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
