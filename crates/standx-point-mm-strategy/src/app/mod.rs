pub mod event;
pub mod state;

use crate::app::event::AppEvent;
use crate::app::state::{AppMode, AppState, Pane};
use crate::state::storage::Storage;
use anyhow::Result;
use ratatui::crossterm::event::{self as crossterm_event, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::DefaultTerminal;
use std::io;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

const TICK_RATE: u64 = 250;

pub struct App {
    pub state: Arc<RwLock<AppState>>,
    pub storage: Arc<Storage>,
    pub event_tx: mpsc::Sender<AppEvent>,
    pub event_rx: mpsc::Receiver<AppEvent>,
    pub should_exit: bool,
}

impl App {
    pub async fn new() -> Result<Self> {
        let storage = Arc::new(Storage::new().await?);
        let state = Arc::new(RwLock::new(AppState::new(storage.clone()).await?));
        let (event_tx, event_rx) = mpsc::channel(100);

        Ok(Self {
            state,
            storage,
            event_tx,
            event_rx,
            should_exit: false,
        })
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(TICK_RATE));

        while !self.should_exit {
            tokio::select! {
                _ = interval.tick() => {
                    self.handle_event(AppEvent::Tick).await?;
                }
                Some(event) = self.event_rx.recv() => {
                    self.handle_event(event).await?;
                }
                Ok(event) = tokio::task::spawn_blocking(|| crossterm_event::read()) => {
                    if let Ok(Event::Key(key)) = event {
                        if key.kind == KeyEventKind::Press {
                            self.handle_event(AppEvent::Key(key)).await?;
                        }
                    }
                }
            }

            self.draw(terminal).await?;
        }

        Ok(())
    }

    async fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        let mut state = self.state.write().await;

        match event {
            AppEvent::Key(key) => match state.mode {
                AppMode::Normal => Self::handle_normal_mode(&mut state, key, &mut self.should_exit).await?,
                AppMode::Insert => Self::handle_insert_mode(&mut state, key).await?,
                AppMode::Dialog => Self::handle_dialog_mode(&mut state, key).await?,
            },
            AppEvent::Tick => {
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
            Char('s') => {
                state.start_selected_task().await?;
            }
            Char('x') => {
                state.stop_selected_task().await?;
            }
            F(1) => {
                state.show_help().await?;
            }
            F(2) => {
                state.switch_sidebar_mode(crate::app::state::SidebarMode::Accounts).await?;
            }
            F(3) => {
                state.switch_sidebar_mode(crate::app::state::SidebarMode::Tasks).await?;
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
        terminal.draw(|frame| {
            crate::ui::render(frame, &state);
        })?;
        Ok(())
    }
}
