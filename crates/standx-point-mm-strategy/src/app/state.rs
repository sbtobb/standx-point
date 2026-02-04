use crate::app::event::{AppEvent, TaskState};
use crate::state::storage::{Storage, Account, Task};
use anyhow::Result;
use ratatui::crossterm::event::KeyEvent;
use std::sync::Arc;

/// Application mode - determines how keyboard input is interpreted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Normal mode - vi-like navigation commands
    Normal,
    /// Insert mode - typing into input fields
    Insert,
    /// Dialog mode - modal dialog is open
    Dialog,
}

/// Which pane has focus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    /// Left sidebar (account/task list)
    Sidebar,
    /// Right detail view
    Detail,
}

/// What the sidebar is currently displaying
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMode {
    /// Showing accounts
    Accounts,
    /// Showing tasks
    Tasks,
}

/// Modal types that can be displayed
#[derive(Debug, Clone)]
pub enum ModalType {
    /// Help overlay
    Help,
    /// Confirmation dialog with message
    Confirm { title: String, message: String },
}

/// Main application state
#[derive(Debug)]
pub struct AppState {
    /// Current input mode
    pub mode: AppMode,
    /// Which pane has focus
    pub focused_pane: Pane,
    /// What the sidebar is displaying
    pub sidebar_mode: SidebarMode,
    /// Currently selected index in the sidebar
    pub selected_index: usize,
    /// Status message to display to user
    pub status_message: Option<String>,
    /// Whether help overlay is shown
    pub show_help: bool,
    /// Current modal (if any)
    pub modal: Option<ModalType>,
    /// Reference to storage for data access
    storage: Arc<Storage>,
    /// Cached accounts for synchronous access in render
    pub accounts: Vec<Account>,
    /// Cached tasks for synchronous access in render
    pub tasks: Vec<Task>,
}

impl AppState {
    /// Create new application state
    pub async fn new(storage: Arc<Storage>) -> Result<Self> {
        let accounts = storage.list_accounts().await?;
        let tasks = storage.list_tasks().await?;
        Ok(Self {
            mode: AppMode::Normal,
            focused_pane: Pane::Sidebar,
            sidebar_mode: SidebarMode::Accounts,
            selected_index: 0,
            status_message: Some("Press F1 for help".to_string()),
            show_help: false,
            modal: None,
            storage,
            accounts,
            tasks,
        })
    }

    /// Move to next item in sidebar
    pub async fn next_item(&mut self) -> Result<()> {
        let count = match self.sidebar_mode {
            SidebarMode::Accounts => self.storage.list_accounts().await?.len(),
            SidebarMode::Tasks => self.storage.list_tasks().await?.len(),
        };
        if count > 0 && self.selected_index < count - 1 {
            self.selected_index += 1;
        }
        Ok(())
    }

    /// Move to previous item in sidebar
    pub async fn previous_item(&mut self) -> Result<()> {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
        Ok(())
    }

    /// Change focused pane
    pub async fn focus_pane(&mut self, pane: Pane) -> Result<()> {
        self.focused_pane = pane;
        Ok(())
    }

    /// Switch sidebar mode (Accounts/Tasks)
    pub async fn switch_sidebar_mode(&mut self, mode: SidebarMode) -> Result<()> {
        self.sidebar_mode = mode;
        self.selected_index = 0;
        Ok(())
    }

    /// Update on tick (called periodically)
    pub async fn update_tick(&mut self) -> Result<()> {
        // Placeholder for periodic updates
        Ok(())
    }

    /// Show help
    pub async fn show_help(&mut self) -> Result<()> {
        self.show_help = true;
        Ok(())
    }

    /// Create new item (account or task)
    pub async fn create_new_item(&mut self) -> Result<()> {
        self.mode = AppMode::Dialog;
        self.status_message = Some(format!("Creating new {:?}...", self.sidebar_mode));
        Ok(())
    }

    /// Edit selected item
    pub async fn edit_selected_item(&mut self) -> Result<()> {
        self.mode = AppMode::Dialog;
        self.status_message = Some("Editing selected item...".to_string());
        Ok(())
    }

    /// Delete selected item
    pub async fn delete_selected_item(&mut self) -> Result<()> {
        self.mode = AppMode::Dialog;
        self.status_message = Some("Confirm deletion...".to_string());
        Ok(())
    }

    /// Start selected task
    pub async fn start_selected_task(&mut self) -> Result<()> {
        if self.sidebar_mode == SidebarMode::Tasks {
            self.status_message = Some("Starting task...".to_string());
        }
        Ok(())
    }

    /// Stop selected task
    pub async fn stop_selected_task(&mut self) -> Result<()> {
        if self.sidebar_mode == SidebarMode::Tasks {
            self.status_message = Some("Stopping task...".to_string());
        }
        Ok(())
    }

    /// Handle form input (called when in Insert mode)
    pub async fn handle_form_input(&mut self, _key: KeyEvent) -> Result<()> {
        Ok(())
    }

    /// Close dialog/modal
    pub async fn close_dialog(&mut self, _confirmed: bool) -> Result<()> {
        self.mode = AppMode::Normal;
        Ok(())
    }
}
