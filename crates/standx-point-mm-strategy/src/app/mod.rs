/// **Input**: App events, key inputs, storage access, shared app state.
/// **Output**: Event dispatch, state mutations, and synchronous UI draw calls.
/// **Position**: TUI application runtime and event dispatcher.
/// **Update**: Wire account form modal input and cancel handling.
/// **Update**: Close task form modal on cancel.
/// **Update**: Persist per-task start/stop state and refresh task list.
pub mod event;
pub mod state;

use crate::app::event::AppEvent;
use crate::app::state::{AccountDetail, AppMode, AppState, ModalType, Pane, SidebarMode};
use crate::state::storage::{Storage, TaskState};
use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self as crossterm_event, Event, KeyCode, KeyEvent};
use standx_point_adapter::auth::{EvmWalletSigner, SolanaWalletSigner};
use standx_point_adapter::{AuthManager, Chain, Credentials, StandxClient, WalletSigner};
use standx_point_mm_strategy::{config, market_data::MarketDataHub, task::TaskManager};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, info};

pub const TICK_RATE: u64 = 250;
const ACCOUNT_DETAIL_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

pub struct App {
    pub state: Arc<RwLock<AppState>>,
    pub storage: Arc<Storage>,
    pub task_manager: TaskManager,
    pub market_data: Arc<Mutex<MarketDataHub>>,
    pub event_tx: mpsc::Sender<AppEvent>,
    pub event_rx: mpsc::Receiver<AppEvent>,
    pub should_exit: bool,
    pub auto_exit_after_ticks: Option<u64>,
    pub tick_count: u64,
}

impl App {
    pub async fn new() -> Result<Self> {
        let storage = Arc::new(Storage::new().await?);
        let state = Arc::new(RwLock::new(AppState::new(storage.clone()).await?));
        let (event_tx, event_rx) = mpsc::channel(100);
        let market_data = Arc::new(Mutex::new(MarketDataHub::new()));
        let task_manager = TaskManager::with_market_data_hub(Arc::clone(&market_data));

        // Parse auto-exit configuration from environment variable
        let auto_exit_after_ticks = std::env::var("STANDX_TUI_TEST_EXIT_AFTER_TICKS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .and_then(|n| if n > 0 { Some(n) } else { None });

        Ok(Self {
            state,
            storage,
            task_manager,
            market_data,
            event_tx,
            event_rx,
            should_exit: false,
            auto_exit_after_ticks,
            tick_count: 0,
        })
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(TICK_RATE));
        let event_tx = self.event_tx.clone();

        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        let input_task = tokio::task::spawn_blocking(move || {
            let poll_timeout = Duration::from_millis(100);
            loop {
                if shutdown_clone.load(Ordering::Relaxed) {
                    break;
                }

                if crossterm_event::poll(poll_timeout).unwrap_or(false) {
                    match crossterm_event::read() {
                        Ok(Event::Key(key)) => {
                            // No filtering on KeyEventKind::Press to allow tmux/repeat
                            if let Err(_e) = event_tx.try_send(AppEvent::Key(key)) {
                                // Channel full or closed - if closed we should exit, but we rely on shutdown flag
                            }
                        }
                        Ok(Event::Resize(w, h)) => {
                            let _ = event_tx.try_send(AppEvent::Resize(w, h));
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
            }
        });

        while !self.should_exit {
            tokio::select! {
                _ = interval.tick() => {
                    self.handle_event(AppEvent::Tick).await?;
                    self.tick_count += 1;

                    // Check if we need to auto-exit after N ticks
                    if let Some(n) = self.auto_exit_after_ticks && self.tick_count >= n {
                            self.should_exit = true;
                        }
                }
                Some(event) = self.event_rx.recv() => {
                    self.handle_event(event).await?;
                }
            }

            self.draw(terminal).await?;
        }

        // Signal input loop to exit and wait for it
        info!(action = "shutdown", "shutdown started");
        shutdown.store(true, Ordering::Relaxed);
        let _ = input_task.await;
        info!(action = "shutdown_complete", "shutdown completed");

        Ok(())
    }

    pub async fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Key(key) => {
                debug!(key = ?key, "key pressed");

                // Handle Ctrl+C force quit immediately (bypasses all other logic)
                if key.code == KeyCode::Char('c')
                    && key
                        .modifiers
                        .contains(crossterm_event::KeyModifiers::CONTROL)
                {
                    self.should_exit = true;
                    info!(
                        action = "force_quit",
                        "user requested force quit via Ctrl+C"
                    );
                    return Ok(());
                }

                let mut state = self.state.write().await;

                // Handle help overlay interaction
                if state.show_help {
                    match key.code {
                        KeyCode::Esc => {
                            state.close_help().await?;
                            info!(action = "close_help", sidebar_mode = ?state.sidebar_mode, focused_pane = ?state.focused_pane, "help overlay closed");
                        }
                        _ => {
                            // Consume all other keys while help is shown
                        }
                    }
                    return Ok(());
                }

                // Set keypress flash message (2 ticks = ~500ms at 250ms per tick)
                state.keypress_flash = Some((format!("Key pressed: {:?}", key), 2));

                match state.mode {
                    AppMode::Normal => {
                        // Extract necessary information before dropping the state lock
                        let sidebar_mode = state.sidebar_mode;
                        let selected_index = state.selected_index;
                        let tasks = state.tasks.clone();
                        let focused_pane = state.focused_pane;

                        drop(state); // Drop the mutable state borrow

                        if sidebar_mode == SidebarMode::Tasks && key.code == KeyCode::Char('s') {
                            if selected_index >= tasks.len() {
                                let mut state = self.state.write().await;
                                state.status_message = Some("No task selected".to_string());
                                return Ok(());
                            }

                            let selected_task = tasks[selected_index].clone();
                            {
                                let mut state = self.state.write().await;
                                state.start_selected_task().await?;
                            }

                            let account = self.storage.get_account(&selected_task.account_id).await;
                            if let Some(account) = account {
                                let account = match refresh_account_credentials(
                                    &self.storage,
                                    &account,
                                )
                                .await
                                {
                                    Ok(account) => account,
                                    Err(err) => {
                                        let mut state = self.state.write().await;
                                        state.status_message = Some(format!(
                                            "Failed to refresh account credentials: {}",
                                            err
                                        ));
                                        state.stop_spinner().await?;
                                        return Ok(());
                                    }
                                };
                                let account_chain = account.chain.unwrap_or(Chain::Bsc);
                                // Convert storage task and account to StrategyConfig
                                let task_config = config::TaskConfig {
                                    id: selected_task.id.clone(),
                                    symbol: selected_task.symbol.clone(),
                                    account_id: account.id.clone(),
                                    risk: config::RiskConfig {
                                        level: selected_task.risk_level.clone(),
                                        budget_usd: selected_task.budget_usd.clone(),
                                    },
                                };

                                let strategy_config = config::StrategyConfig {
                                    accounts: vec![config::AccountConfig {
                                        id: account.id.clone(),
                                        jwt_token: account.jwt_token.clone(),
                                        signing_key: account.signing_key.clone(),
                                        chain: account_chain,
                                    }],
                                    tasks: vec![task_config],
                                };

                                // Spawn task using TaskManager
                                match self.task_manager.spawn_from_config(strategy_config).await {
                                    Ok(()) => {
                                        if let Err(err) = self
                                            .storage
                                            .update_task(&selected_task.id, |task| {
                                                task.state = TaskState::Running;
                                            })
                                            .await
                                        {
                                            let mut state = self.state.write().await;
                                            state.status_message = Some(format!(
                                                "Failed to persist task state: {}",
                                                err
                                            ));
                                            state.stop_spinner().await?;
                                            return Ok(());
                                        }

                                        if let Err(err) = self.refresh_tasks_and_clamp().await {
                                            let mut state = self.state.write().await;
                                            state.status_message = Some(format!(
                                                "Task started but refresh failed: {}",
                                                err
                                            ));
                                            state.stop_spinner().await?;
                                            return Ok(());
                                        }

                                        let mut state = self.state.write().await;
                                        state.status_message =
                                            Some(format!("Started task: {}", selected_task.id));
                                        state.stop_spinner().await?;
                                        info!(action = "start_task", task_id = %selected_task.id, symbol = %selected_task.symbol, sidebar_mode = ?sidebar_mode, focused_pane = ?focused_pane, "task started");
                                    }
                                    Err(err) => {
                                        let failure_message = err.to_string();
                                        let update_message = failure_message.clone();
                                        let update_result = self
                                            .storage
                                            .update_task(&selected_task.id, move |task| {
                                                task.state = TaskState::Failed(update_message);
                                            })
                                            .await;

                                        if update_result.is_ok() {
                                            let _ = self.refresh_tasks_and_clamp().await;
                                        }

                                        let mut state = self.state.write().await;
                                        state.status_message = Some(format!(
                                            "Failed to start task {}: {}",
                                            selected_task.id, failure_message
                                        ));
                                        state.stop_spinner().await?;
                                        info!(action = "start_task_failed", task_id = %selected_task.id, symbol = %selected_task.symbol, sidebar_mode = ?sidebar_mode, focused_pane = ?focused_pane, error = %failure_message, "task start failed");
                                    }
                                }
                            } else {
                                let mut state = self.state.write().await;
                                state.status_message =
                                    Some("Account not found for task".to_string());
                                state.stop_spinner().await?;
                            }
                        } else if sidebar_mode == SidebarMode::Tasks
                            && key.code == KeyCode::Char('x')
                        {
                            if selected_index >= tasks.len() {
                                let mut state = self.state.write().await;
                                state.status_message = Some("No task selected".to_string());
                                return Ok(());
                            }

                            let selected_task = tasks[selected_index].clone();
                            {
                                let mut state = self.state.write().await;
                                state.stop_selected_task().await?;
                            }

                            match self.task_manager.stop_task(&selected_task.id).await {
                                Ok(()) => {
                                    if let Err(err) = self
                                        .storage
                                        .update_task(&selected_task.id, |task| {
                                            task.state = TaskState::Stopped;
                                        })
                                        .await
                                    {
                                        let mut state = self.state.write().await;
                                        state.status_message =
                                            Some(format!("Failed to persist task state: {}", err));
                                        state.stop_spinner().await?;
                                        return Ok(());
                                    }

                                    if let Err(err) = self.refresh_tasks_and_clamp().await {
                                        let mut state = self.state.write().await;
                                        state.status_message = Some(format!(
                                            "Task stopped but refresh failed: {}",
                                            err
                                        ));
                                        state.stop_spinner().await?;
                                        return Ok(());
                                    }

                                    let mut state = self.state.write().await;
                                    state.status_message =
                                        Some(format!("Stopped task: {}", selected_task.id));
                                    state.stop_spinner().await?;
                                    info!(action = "stop_task", task_id = %selected_task.id, symbol = %selected_task.symbol, sidebar_mode = ?sidebar_mode, focused_pane = ?focused_pane, "task stopped");
                                }
                                Err(err) => {
                                    let mut state = self.state.write().await;
                                    state.status_message = Some(format!(
                                        "Failed to stop task {}: {}",
                                        selected_task.id, err
                                    ));
                                    state.stop_spinner().await?;
                                    info!(action = "stop_task_failed", task_id = %selected_task.id, symbol = %selected_task.symbol, sidebar_mode = ?sidebar_mode, focused_pane = ?focused_pane, error = %err, "task stop failed");
                                }
                            }
                        } else {
                            // Handle other normal mode keys
                            let mut state = self.state.write().await;
                            Self::handle_normal_mode(&mut state, key, &mut self.should_exit)
                                .await?;
                        }
                    }
                    AppMode::Insert => Self::handle_insert_mode(&mut state, key).await?,
                    AppMode::Dialog => Self::handle_dialog_mode(&mut state, key).await?,
                }
            }
            AppEvent::Tick => {
                {
                    let mut state = self.state.write().await;
                    state.update_tick().await?;
                }
                self.refresh_account_detail().await?;
                self.refresh_task_runtime_status().await?;
            }
            AppEvent::Resize(w, h) => {
                debug!(width = w, height = h, "terminal resized");
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_normal_mode(
        state: &mut AppState,
        key: KeyEvent,
        should_exit: &mut bool,
    ) -> Result<()> {
        use KeyCode::*;

        match key.code {
            Char('q') | Esc => {
                *should_exit = true;
                info!(action = "quit", sidebar_mode = ?state.sidebar_mode, focused_pane = ?state.focused_pane, "user requested quit");
            }
            Char('j') | Down => {
                state.next_item().await?;
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('k') | Up => {
                state.previous_item().await?;
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('h') | Left => {
                state.focus_pane(Pane::Sidebar).await?;
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('l') | Right => {
                state.focus_pane(Pane::Detail).await?;
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('v') | Enter => {
                // Check if there's a selectable item
                let has_selectable_item = match state.sidebar_mode {
                    SidebarMode::Accounts => state.selected_index < state.accounts.len(),
                    SidebarMode::Tasks => state.selected_index < state.tasks.len(),
                };

                if has_selectable_item {
                    state.focus_pane(Pane::Detail).await?;
                } else {
                    let message = match state.sidebar_mode {
                        SidebarMode::Accounts => "No account selected",
                        SidebarMode::Tasks => "No task selected",
                    };
                    state.status_message = Some(message.to_string());
                }
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Tab => {
                // Cycle focus: Sidebar -> Detail -> Menu -> Sidebar
                let next_pane = match state.focused_pane {
                    Pane::Sidebar => Pane::Detail,
                    Pane::Detail => Pane::Menu,
                    Pane::Menu => Pane::Sidebar,
                };
                state.focus_pane(next_pane).await?;
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('n') => {
                state.create_new_item().await?;
                info!(action = "create_new_item", sidebar_mode = ?state.sidebar_mode, focused_pane = ?state.focused_pane, "create new item dialog opened");
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('e') => {
                state.edit_selected_item().await?;
                info!(action = "edit_selected_item", sidebar_mode = ?state.sidebar_mode, focused_pane = ?state.focused_pane, selected_index = state.selected_index, "edit selected item dialog opened");
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('d') => {
                state.delete_selected_item().await?;
                info!(action = "delete_selected_item", sidebar_mode = ?state.sidebar_mode, focused_pane = ?state.focused_pane, selected_index = state.selected_index, "delete selected item dialog opened");
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('s') => {
                if state.sidebar_mode == SidebarMode::Tasks {
                    state.start_selected_task().await?;
                    let selected_task_id = if state.selected_index < state.tasks.len() {
                        Some(&state.tasks[state.selected_index].id)
                    } else {
                        None
                    };
                    info!(action = "start_selected_task", sidebar_mode = ?state.sidebar_mode, focused_pane = ?state.focused_pane, selected_index = state.selected_index, task_id = ?selected_task_id, "start selected task requested");
                }
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('g') => {
                if state.pending_g_ticks > 0 {
                    // Second 'g' pressed - jump to first item
                    state.jump_to_first_item().await?;
                } else {
                    // First 'g' pressed - arm the prefix state (4 ticks = 1 second at 250ms per tick)
                    state.pending_g_ticks = 4;
                }
            }
            Char('G') => {
                state.jump_to_last_item().await?;
            }
            Char('1')
                if key
                    .modifiers
                    .contains(crossterm_event::KeyModifiers::CONTROL) =>
            {
                state.show_help().await?;
                info!(action = "show_help", sidebar_mode = ?state.sidebar_mode, focused_pane = ?state.focused_pane, "help overlay opened");
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('2')
                if key
                    .modifiers
                    .contains(crossterm_event::KeyModifiers::CONTROL) =>
            {
                state
                    .switch_sidebar_mode(crate::app::state::SidebarMode::Accounts)
                    .await?;
                info!(action = "switch_sidebar_mode", sidebar_mode = ?SidebarMode::Accounts, focused_pane = ?state.focused_pane, "sidebar mode switched to accounts");
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('3')
                if key
                    .modifiers
                    .contains(crossterm_event::KeyModifiers::CONTROL) =>
            {
                state
                    .switch_sidebar_mode(crate::app::state::SidebarMode::Tasks)
                    .await?;
                info!(action = "switch_sidebar_mode", sidebar_mode = ?SidebarMode::Tasks, focused_pane = ?state.focused_pane, "sidebar mode switched to tasks");
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('4')
                if key
                    .modifiers
                    .contains(crossterm_event::KeyModifiers::CONTROL) =>
            {
                state.toggle_credentials().await?;
                // Show flash message indicating current state
                let msg = if state.show_credentials {
                    "Credentials: shown"
                } else {
                    "Credentials: hidden"
                };
                state.keypress_flash = Some((msg.to_string(), 3)); // Show for ~750ms
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            _ => {
                // Any other key clears the pending 'g' prefix
                state.pending_g_ticks = 0;
            }
        }

        Ok(())
    }

    async fn handle_insert_mode(state: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                if matches!(state.modal.as_ref(), Some(ModalType::AccountForm { .. })) {
                    state.close_account_form_modal();
                } else if matches!(state.modal.as_ref(), Some(ModalType::TaskForm { .. })) {
                    state.close_task_form_modal();
                } else {
                    state.mode = AppMode::Normal;
                }
            }
            _ => {
                state.handle_form_input(key).await?;
            }
        }
        Ok(())
    }

    async fn handle_dialog_mode(state: &mut AppState, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') => {
                state.close_dialog(false).await?;
            }
            KeyCode::Char('y') | KeyCode::Enter => {
                state.close_dialog(true).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn draw(&self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let state = self.state.read().await;
        let storage = self.storage.clone();
        let market_data = self.market_data.lock().await;
        terminal.draw(|frame| {
            crate::ui::render(frame, &state, &storage, &market_data);
        })?;
        Ok(())
    }

    async fn refresh_account_detail(&mut self) -> Result<()> {
        let account = {
            let state = self.state.read().await;
            if state.sidebar_mode != SidebarMode::Accounts || state.focused_pane != Pane::Detail {
                return Ok(());
            }

            let Some(account) = state.accounts.get(state.selected_index).cloned() else {
                return Ok(());
            };

            let refresh_due = match state
                .account_details
                .get(&account.id)
                .and_then(|detail| detail.last_updated)
            {
                Some(last) => last.elapsed() >= ACCOUNT_DETAIL_REFRESH_INTERVAL,
                None => true,
            };

            if !refresh_due {
                return Ok(());
            }

            account
        };

        let detail_result = self.fetch_account_detail(&account).await;
        let mut state = self.state.write().await;
        let entry = state
            .account_details
            .entry(account.id.clone())
            .or_insert_with(AccountDetail::empty);

        match detail_result {
            Ok(detail) => {
                *entry = detail;
            }
            Err(err) => {
                entry.last_error = Some(err.to_string());
                entry.last_updated = Some(Instant::now());
            }
        }

        Ok(())
    }

    async fn fetch_account_detail(
        &self,
        account: &crate::state::storage::Account,
    ) -> Result<AccountDetail> {
        let chain = account
            .chain
            .ok_or_else(|| anyhow::anyhow!("account chain not set"))?;

        let mut client = StandxClient::new()
            .map_err(|err| anyhow::anyhow!("create StandxClient failed: {err}"))?;
        let credentials = Credentials {
            jwt_token: account.jwt_token.clone(),
            wallet_address: account.id.clone(),
            chain,
        };
        client.set_credentials(credentials);

        let balance = client.query_balance().await?;
        let positions = client.query_positions(None).await?;

        Ok(AccountDetail {
            balance: Some(balance),
            positions,
            last_updated: Some(Instant::now()),
            last_error: None,
        })
    }

    async fn refresh_tasks_and_clamp(&self) -> Result<()> {
        let tasks = self.storage.list_tasks().await?;
        let mut state = self.state.write().await;
        state.tasks = tasks;
        if state.tasks.is_empty() {
            state.selected_index = 0;
        } else if state.selected_index >= state.tasks.len() {
            state.selected_index = state.tasks.len() - 1;
        }
        Ok(())
    }

    async fn refresh_task_runtime_status(&self) -> Result<()> {
        let runtime_status = self.task_manager.runtime_status_snapshot();
        let mut state = self.state.write().await;
        state.runtime_status = runtime_status;
        Ok(())
    }
}

async fn refresh_account_credentials(
    storage: &Storage,
    account: &crate::state::storage::Account,
) -> Result<crate::state::storage::Account> {
    let chain = account
        .chain
        .ok_or_else(|| anyhow::anyhow!("account chain not set"))?;
    if account.private_key.trim().is_empty() {
        return Err(anyhow::anyhow!("account private key is missing"));
    }

    info!(
        account_id = %account.id,
        chain = ?chain,
        "refreshing account credentials"
    );

    let client =
        StandxClient::new().map_err(|err| anyhow::anyhow!("create StandxClient failed: {err}"))?;
    let auth = AuthManager::new(client);
    let (wallet_address, login_response): (String, _) = match chain {
        Chain::Bsc => {
            let wallet = EvmWalletSigner::new(&account.private_key)
                .map_err(|err| anyhow::anyhow!("invalid EVM private key: {err}"))?;
            let address = wallet.address().to_string();
            let login = auth
                .authenticate(&wallet, 7 * 24 * 60 * 60)
                .await
                .map_err(|err| anyhow::anyhow!("authenticate failed: {err}"))?;
            (address, login)
        }
        Chain::Solana => {
            let wallet = SolanaWalletSigner::new(&account.private_key)
                .map_err(|err| anyhow::anyhow!("invalid Solana private key: {err}"))?;
            let address = wallet.address().to_string();
            let login = auth
                .authenticate(&wallet, 7 * 24 * 60 * 60)
                .await
                .map_err(|err| anyhow::anyhow!("authenticate failed: {err}"))?;
            (address, login)
        }
    };

    if wallet_address != account.id {
        return Err(anyhow::anyhow!(
            "wallet address mismatch: stored={} derived={}",
            account.id,
            wallet_address
        ));
    }

    let signer = auth
        .key_manager()
        .get_or_create_signer(&wallet_address)
        .map_err(|err| anyhow::anyhow!("load ed25519 signer failed: {err}"))?;
    let signing_key = STANDARD.encode(signer.secret_key_bytes());

    storage
        .update_account(&account.id, |stored| {
            stored.jwt_token = login_response.token.clone();
            stored.signing_key = signing_key.clone();
        })
        .await
        .map_err(|err| anyhow::anyhow!("update account credentials failed: {err}"))?;

    info!(
        account_id = %account.id,
        chain = ?chain,
        "account credentials refreshed"
    );

    let mut refreshed = account.clone();
    refreshed.jwt_token = login_response.token;
    refreshed.signing_key = signing_key;
    Ok(refreshed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{AppMode, AppState};
    use crate::state::storage::Storage;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::env;
    use std::sync::Arc;
    use std::time::SystemTime;

    /// Creates a unique temporary directory for testing
    fn create_unique_temp_dir() -> std::path::PathBuf {
        let temp_dir = env::temp_dir();
        let pid = std::process::id();
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        temp_dir.join(format!("standx-mm-test-{}-{}", pid, timestamp))
    }

    /// Creates a minimal test App instance
    async fn create_test_app() -> Result<App> {
        let temp_dir = create_unique_temp_dir();
        let storage = Arc::new(Storage::new_in_dir(&temp_dir).await.unwrap());
        let state = Arc::new(RwLock::new(AppState::new(storage.clone()).await?));
        let (event_tx, event_rx) = mpsc::channel(100);
        let market_data = Arc::new(Mutex::new(MarketDataHub::new()));
        let task_manager = TaskManager::with_market_data_hub(Arc::clone(&market_data));

        Ok(App {
            state,
            storage,
            task_manager,
            market_data,
            event_tx,
            event_rx,
            should_exit: false,
            auto_exit_after_ticks: None,
            tick_count: 0,
        })
    }

    /// Creates a Ctrl+C key event
    fn create_ctrl_c_event() -> AppEvent {
        AppEvent::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: ratatui::crossterm::event::KeyEventKind::Press,
            state: ratatui::crossterm::event::KeyEventState::NONE,
        })
    }

    #[tokio::test]
    async fn test_ctrl_c_force_quit_normal_mode() {
        // Create test app
        let mut app = create_test_app().await.unwrap();

        // Verify initial state
        assert!(!app.should_exit);

        // Send Ctrl+C event
        app.handle_event(create_ctrl_c_event()).await.unwrap();

        // Verify app should exit
        assert!(app.should_exit);
    }

    #[tokio::test]
    async fn test_ctrl_c_force_quit_help_mode() {
        // Create test app
        let mut app = create_test_app().await.unwrap();

        // Open help overlay
        {
            let mut state = app.state.write().await;
            state.show_help = true;
            assert!(state.show_help);
        }

        // Verify initial state
        assert!(!app.should_exit);

        // Send Ctrl+C event
        app.handle_event(create_ctrl_c_event()).await.unwrap();

        // Verify app should exit
        assert!(app.should_exit);
    }

    #[tokio::test]
    async fn test_ctrl_c_force_quit_dialog_mode() {
        // Create test app
        let mut app = create_test_app().await.unwrap();

        // Enter dialog mode
        {
            let mut state = app.state.write().await;
            state.mode = AppMode::Dialog;
            assert_eq!(state.mode, AppMode::Dialog);
        }

        // Verify initial state
        assert!(!app.should_exit);

        // Send Ctrl+C event
        app.handle_event(create_ctrl_c_event()).await.unwrap();

        // Verify app should exit
        assert!(app.should_exit);
    }

    #[tokio::test]
    async fn test_ctrl_c_force_quit_insert_mode() {
        // Create test app
        let mut app = create_test_app().await.unwrap();

        // Enter insert mode
        {
            let mut state = app.state.write().await;
            state.mode = AppMode::Insert;
            assert_eq!(state.mode, AppMode::Insert);
        }

        // Verify initial state
        assert!(!app.should_exit);

        // Send Ctrl+C event
        app.handle_event(create_ctrl_c_event()).await.unwrap();

        // Verify app should exit
        assert!(app.should_exit);
    }
}
