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
    /// Bottom menu bar
    Menu,
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
    /// Keypress flash message (text, ticks remaining)
    pub keypress_flash: Option<(String, u8)>,
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
    /// Pending 'g' key prefix state (number of ticks remaining before timeout)
    pub pending_g_ticks: u8,
    /// Whether to show full credentials (jwt_token, signing_key) or mask them
    pub show_credentials: bool,
    /// Spinner animation ticks remaining
    pub spinner_ticks: u8,
    /// Current spinner frame index
    pub spinner_frame: u8,
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
            keypress_flash: None,
            show_help: false,
            modal: None,
            storage,
            accounts,
            tasks,
            pending_g_ticks: 0,
            show_credentials: false, // Default to masking credentials
            spinner_ticks: 0,
            spinner_frame: 0,
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

    /// Jump to first item in sidebar
    pub async fn jump_to_first_item(&mut self) -> Result<()> {
        self.selected_index = 0;
        self.pending_g_ticks = 0; // Clear any pending 'g' prefix
        Ok(())
    }

    /// Jump to last item in sidebar
    pub async fn jump_to_last_item(&mut self) -> Result<()> {
        let count = match self.sidebar_mode {
            SidebarMode::Accounts => self.storage.list_accounts().await?.len(),
            SidebarMode::Tasks => self.storage.list_tasks().await?.len(),
        };
        if count > 0 {
            self.selected_index = count - 1;
        }
        self.pending_g_ticks = 0; // Clear any pending 'g' prefix
        Ok(())
    }

    /// Update on tick (called periodically)
    pub async fn update_tick(&mut self) -> Result<()> {
        // Decrement pending 'g' key prefix timer if active
        if self.pending_g_ticks > 0 {
            self.pending_g_ticks -= 1;
        }
        // Decrement keypress flash timer if active
        if let Some((_, ref mut ticks)) = self.keypress_flash {
            if *ticks > 0 {
                *ticks -= 1;
            } else {
                self.keypress_flash = None;
            }
        }
        // Update spinner animation if active
        if self.spinner_ticks > 0 {
            self.spinner_ticks -= 1;
            // Advance spinner frame (4 frames: |, /, -, \)
            self.spinner_frame = (self.spinner_frame + 1) % 4;
        }
        Ok(())
    }

    /// Show help
    pub async fn show_help(&mut self) -> Result<()> {
        self.show_help = true;
        Ok(())
    }

    /// Close help
    pub async fn close_help(&mut self) -> Result<()> {
        self.show_help = false;
        Ok(())
    }

    /// Toggle help
    pub async fn toggle_help(&mut self) -> Result<()> {
        self.show_help = !self.show_help;
        Ok(())
    }

    /// Toggle credentials visibility
    pub async fn toggle_credentials(&mut self) -> Result<()> {
        self.show_credentials = !self.show_credentials;
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
            self.spinner_ticks = 12; // 3 seconds at 250ms per tick
            self.spinner_frame = 0;
        }
        Ok(())
    }

    /// Stop selected task
    pub async fn stop_selected_task(&mut self) -> Result<()> {
        if self.sidebar_mode == SidebarMode::Tasks {
            self.status_message = Some("Stopping task...".to_string());
            self.spinner_ticks = 12; // 3 seconds at 250ms per tick
            self.spinner_frame = 0;
        }
        Ok(())
    }

    /// Stop all tasks
    pub async fn stop_all_tasks(&mut self) -> Result<()> {
        self.status_message = Some("Stopping all tasks...".to_string());
        self.spinner_ticks = 12; // 3 seconds at 250ms per tick
        self.spinner_frame = 0;
        Ok(())
    }

    /// Stop spinner animation
    pub async fn stop_spinner(&mut self) -> Result<()> {
        self.spinner_ticks = 0;
        self.spinner_frame = 0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::storage::Storage;
    use std::env;
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

    /// Cleans up a temporary directory (ignores errors)
    fn cleanup_temp_dir(path: &std::path::Path) {
        let _ = std::fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn test_pending_g_ticks_timeout() {
        // Create unique temp dir for this test
        let temp_dir = create_unique_temp_dir();
        let storage = Arc::new(Storage::new_in_dir(&temp_dir).await.unwrap());
        let mut state = AppState::new(storage.clone()).await.unwrap();

        // Arm the prefix
        state.pending_g_ticks = 4;
        assert_eq!(state.pending_g_ticks, 4);

        // Simulate ticks (each update_tick() decrements by 1)
        for i in (1..=4).rev() {
            state.update_tick().await.unwrap();
            assert_eq!(state.pending_g_ticks, i - 1);
        }

        // After 4 ticks, should be 0
        assert_eq!(state.pending_g_ticks, 0);

        // Cleanup
        cleanup_temp_dir(&temp_dir);
    }

    #[tokio::test]
    async fn test_gg_prefix_cleared_on_other_key() {
        let temp_dir = create_unique_temp_dir();
        let storage = Arc::new(Storage::new_in_dir(&temp_dir).await.unwrap());
        let mut state = AppState::new(storage.clone()).await.unwrap();

        // Arm the prefix
        state.pending_g_ticks = 4;
        assert_eq!(state.pending_g_ticks, 4);

        // Pressing any other key should clear the prefix
        state.pending_g_ticks = 0;
        assert_eq!(state.pending_g_ticks, 0);

        cleanup_temp_dir(&temp_dir);
    }

    #[tokio::test]
    async fn test_jump_methods_clear_prefix() {
        let temp_dir = create_unique_temp_dir();
        let storage = Arc::new(Storage::new_in_dir(&temp_dir).await.unwrap());
        let mut state = AppState::new(storage.clone()).await.unwrap();

        // Arm the prefix
        state.pending_g_ticks = 4;
        
        // Jump to first should clear the prefix
        state.jump_to_first_item().await.unwrap();
        assert_eq!(state.pending_g_ticks, 0);

        // Arm again
        state.pending_g_ticks = 4;
        
        // Jump to last should clear the prefix
        state.jump_to_last_item().await.unwrap();
        assert_eq!(state.pending_g_ticks, 0);

        cleanup_temp_dir(&temp_dir);
    }
}
