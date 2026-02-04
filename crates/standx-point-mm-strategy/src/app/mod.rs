pub mod event;
pub mod state;

use crate::app::event::AppEvent;
use crate::app::state::{AppMode, AppState, Pane, SidebarMode};
use crate::state::storage::Storage;
use anyhow::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self as crossterm_event, Event, KeyCode, KeyEvent};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use standx_point_mm_strategy::{config, market_data::MarketDataHub, task::TaskManager};
use tokio::sync::{RwLock, mpsc};

pub const TICK_RATE: u64 = 250;

pub struct App {
    pub state: Arc<RwLock<AppState>>,
    pub storage: Arc<Storage>,
    pub task_manager: TaskManager,
    pub market_data: MarketDataHub,
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
        let task_manager = TaskManager::new();
        let market_data = MarketDataHub::new();

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
                    if let Some(n) = self.auto_exit_after_ticks {
                        if self.tick_count >= n {
                            self.should_exit = true;
                        }
                    }
                }
                Some(event) = self.event_rx.recv() => {
                    self.handle_event(event).await?;
                }
            }

            self.draw(terminal).await?;
        }

        // Signal input loop to exit and wait for it
        shutdown.store(true, Ordering::Relaxed);
        let _ = input_task.await;

        Ok(())
    }

    pub async fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
             AppEvent::Key(key) => {
                if key.code == KeyCode::Char('x') {
                    // Handle stop command separately to avoid borrow issues
                    let mut state = self.state.write().await;
                    state.stop_all_tasks().await?;
                    drop(state);
                    self.task_manager.shutdown_and_wait().await?;
                    let mut state = self.state.write().await;
                    state.status_message = Some("All tasks stopped".to_string());
                    state.stop_spinner().await?;
                } else {
                    let mut state = self.state.write().await;

                    // Handle help overlay interaction
                    if state.show_help {
                        match key.code {
                            KeyCode::F(1) | KeyCode::Esc => {
                                state.close_help().await?;
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
                            
                            drop(state); // Drop the mutable state borrow
                            
                            if sidebar_mode == SidebarMode::Tasks && selected_index < tasks.len() && key.code == KeyCode::Char('s') {
                                let selected_task = &tasks[selected_index];
                                let account = self.storage.get_account(&selected_task.account_id).await;
                                
                                if let Some(account) = account {
                                    // Convert storage task and account to StrategyConfig
                                    let task_config = config::TaskConfig {
                                        id: selected_task.id.clone(),
                                        symbol: selected_task.symbol.clone(),
                                        credentials: config::CredentialsConfig {
                                            jwt_token: account.jwt_token.clone(),
                                            signing_key: account.signing_key.clone(),
                                        },
                                        risk: config::RiskConfig {
                                            level: selected_task.risk_level.clone(),
                                            max_position_usd: selected_task.max_position_usd.clone(),
                                            price_jump_threshold_bps: selected_task.price_jump_threshold_bps,
                                        },
                                        sizing: config::SizingConfig {
                                            base_qty: selected_task.base_qty.clone(),
                                            tiers: selected_task.tiers,
                                        },
                                    };
                                    
                                    let strategy_config = config::StrategyConfig {
                                        tasks: vec![task_config],
                                    };
                                    
                                    // Spawn task using TaskManager
                                    self.task_manager.spawn_from_config(strategy_config).await?;
                                    
                                    // Update status message
                                    let mut state = self.state.write().await;
                                    state.status_message = Some(format!("Started task: {}", selected_task.id));
                                    state.stop_spinner().await?;
                                } else {
                                    let mut state = self.state.write().await;
                                    state.status_message = Some("Account not found for task".to_string());
                                    state.stop_spinner().await?;
                                }
                            } else if sidebar_mode == SidebarMode::Tasks && key.code == KeyCode::Char('s') {
                                let mut state = self.state.write().await;
                                state.status_message = Some("No task selected".to_string());
                            } else {
                                // Handle other normal mode keys
                                let mut state = self.state.write().await;
                                Self::handle_normal_mode(&mut state, key, &mut self.should_exit).await?;
                            }
                        }
                        AppMode::Insert => Self::handle_insert_mode(&mut state, key).await?,
                        AppMode::Dialog => Self::handle_dialog_mode(&mut state, key).await?,
                    }
                }
            }
            AppEvent::Tick => {
                let mut state = self.state.write().await;
                state.update_tick().await?;
            }
            AppEvent::Resize(_w, _h) => {
                // Just consume the event to trigger a redraw loop
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
             Char('l') | Right | Enter => {
                 state.focus_pane(Pane::Detail).await?;
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
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('e') => {
                state.edit_selected_item().await?;
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('d') => {
                state.delete_selected_item().await?;
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            Char('s') => {
                if state.sidebar_mode == SidebarMode::Tasks {
                    state.start_selected_task().await?;
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
            F(1) => {
                state.show_help().await?;
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            F(2) => {
                state
                    .switch_sidebar_mode(crate::app::state::SidebarMode::Accounts)
                    .await?;
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            F(3) => {
                state
                    .switch_sidebar_mode(crate::app::state::SidebarMode::Tasks)
                    .await?;
                state.pending_g_ticks = 0; // Clear pending 'g' prefix
            }
            F(4) => {
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
                state.mode = AppMode::Normal;
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
        let market_data = &self.market_data;
        terminal.draw(|frame| {
            crate::ui::render(frame, &state, &storage, market_data);
        })?;
        Ok(())
    }
}
