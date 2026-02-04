pub mod event;
pub mod state;

use crate::app::event::AppEvent;
use crate::app::state::{AppMode, AppState, Pane, SidebarMode};
use crate::state::storage::Storage;
use anyhow::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self as crossterm_event, Event, KeyCode, KeyEvent, KeyEventKind};
use std::io;
use std::sync::Arc;
use standx_point_mm_strategy::{config, market_data::MarketDataHub, task::TaskManager};
use tokio::sync::{RwLock, mpsc};

const TICK_RATE: u64 = 250;

pub struct App {
    pub state: Arc<RwLock<AppState>>,
    pub storage: Arc<Storage>,
    pub task_manager: TaskManager,
    pub market_data: MarketDataHub,
    pub event_tx: mpsc::Sender<AppEvent>,
    pub event_rx: mpsc::Receiver<AppEvent>,
    pub should_exit: bool,
}

impl App {
    pub async fn new() -> Result<Self> {
        let storage = Arc::new(Storage::new().await?);
        let state = Arc::new(RwLock::new(AppState::new(storage.clone()).await?));
        let (event_tx, event_rx) = mpsc::channel(100);
        let task_manager = TaskManager::new();
        let market_data = MarketDataHub::new();

        Ok(Self {
            state,
            storage,
            task_manager,
            market_data,
            event_tx,
            event_rx,
            should_exit: false,
        })
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(TICK_RATE));
        let event_tx = self.event_tx.clone();
        let _input_task = tokio::task::spawn_blocking(move || {
            loop {
                match crossterm_event::read() {
                    Ok(Event::Key(key)) => {
                        if key.kind == KeyEventKind::Press {
                            if event_tx.blocking_send(AppEvent::Key(key)).is_err() {
                                break;
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });

        while !self.should_exit {
            tokio::select! {
                _ = interval.tick() => {
                    self.handle_event(AppEvent::Tick).await?;
                }
                Some(event) = self.event_rx.recv() => {
                    self.handle_event(event).await?;
                }
            }

            self.draw(terminal).await?;
        }

        Ok(())
    }

    async fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Key(key) => {
                if key.code == KeyCode::Char('x') {
                    // Handle stop command separately to avoid borrow issues
                    self.task_manager.shutdown_and_wait().await?;
                    let mut state = self.state.write().await;
                    state.status_message = Some("All tasks stopped".to_string());
                } else {
                    let mut state = self.state.write().await;
                    
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
                                } else {
                                    let mut state = self.state.write().await;
                                    state.status_message = Some("Account not found for task".to_string());
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
            }
            Char('k') | Up => {
                state.previous_item().await?;
            }
            Char('h') | Left => {
                state.focus_pane(Pane::Sidebar).await?;
            }
            Char('l') | Right | Enter => {
                state.focus_pane(Pane::Detail).await?;
            }
            Char('n') => {
                state.create_new_item().await?;
            }
            Char('e') => {
                state.edit_selected_item().await?;
            }
            Char('d') => {
                state.delete_selected_item().await?;
            }
            F(1) => {
                state.show_help().await?;
            }
            F(2) => {
                state
                    .switch_sidebar_mode(crate::app::state::SidebarMode::Accounts)
                    .await?;
            }
            F(3) => {
                state
                    .switch_sidebar_mode(crate::app::state::SidebarMode::Tasks)
                    .await?;
            }
            _ => {}
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
