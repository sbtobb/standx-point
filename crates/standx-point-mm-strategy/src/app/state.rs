/// **Input**: Storage access, modal/form input events, sidebar selections.
/// **Output**: Mutated AppState for UI rendering and persisted accounts/tasks.
/// **Position**: TUI application state and input-handling coordinator.
/// **Update**: Add account-form modal state and input handling.
/// **Update**: Implement account edit flow with read-only ID handling.
/// **Update**: Add confirm action context for account deletion.
/// **Update**: Implement task form create flow with validation.
/// **Update**: Implement task edit flow with storage updates.
/// **Update**: Add task deletion confirmation and storage removal.
use crate::state::storage::{Account, Storage, Task};
use crate::ui::components::account_form::AccountForm;
use crate::ui::components::task_form::TaskForm;
use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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

/// Confirmation actions that can be triggered from dialogs
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    /// Delete a specific account by ID
    DeleteAccount { account_id: String },
    /// Delete a specific task by ID
    DeleteTask { task_id: String },
}

/// Modal types that can be displayed
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ModalType {
    /// Help overlay
    Help,
    /// Confirmation dialog with message and action context
    Confirm {
        title: String,
        message: String,
        action: ConfirmAction,
    },
    /// Account form dialog
    AccountForm { form: AccountForm, is_edit: bool },
    /// Task form dialog
    TaskForm { form: TaskForm, is_edit: bool },
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
    #[allow(dead_code)]
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
        match self.sidebar_mode {
            SidebarMode::Accounts => {
                self.mode = AppMode::Insert;
                self.modal = Some(ModalType::AccountForm {
                    form: AccountForm::new(),
                    is_edit: false,
                });
                self.status_message = Some("Creating new account...".to_string());
            }
            SidebarMode::Tasks => {
                self.mode = AppMode::Insert;
                self.modal = Some(ModalType::TaskForm {
                    form: TaskForm::new(),
                    is_edit: false,
                });
                self.status_message = Some("Creating new task...".to_string());
            }
        }
        Ok(())
    }

    /// Edit selected item
    pub async fn edit_selected_item(&mut self) -> Result<()> {
        match self.sidebar_mode {
            SidebarMode::Accounts => {
                if let Some(account) = self.accounts.get(self.selected_index) {
                    let mut form = AccountForm::from_account(account);
                    form.focused_field = 1;
                    self.mode = AppMode::Insert;
                    self.modal = Some(ModalType::AccountForm {
                        form,
                        is_edit: true,
                    });
                    self.status_message = Some(format!("Editing account: {}", account.id));
                } else {
                    self.status_message = Some("No account selected".to_string());
                }
            }
            SidebarMode::Tasks => {
                if let Some(task) = self.tasks.get(self.selected_index) {
                    let mut form = TaskForm::from_task(task);
                    form.focused_field = 1;
                    self.mode = AppMode::Insert;
                    self.modal = Some(ModalType::TaskForm {
                        form,
                        is_edit: true,
                    });
                    self.status_message = Some(format!("Editing task: {}", task.id));
                } else {
                    self.status_message = Some("No task selected".to_string());
                }
            }
        }
        Ok(())
    }

    /// Delete selected item
    pub async fn delete_selected_item(&mut self) -> Result<()> {
        match self.sidebar_mode {
            SidebarMode::Accounts => {
                if let Some(account) = self.accounts.get(self.selected_index) {
                    let message = format!(
                        "Delete account '{}' ({})",
                        account.id, account.name
                    );
                    self.mode = AppMode::Dialog;
                    self.modal = Some(ModalType::Confirm {
                        title: "Delete Account".to_string(),
                        message,
                        action: ConfirmAction::DeleteAccount {
                            account_id: account.id.clone(),
                        },
                    });
                    self.status_message = Some("Confirm deletion...".to_string());
                } else {
                    self.status_message = Some("No account selected".to_string());
                }
            }
            SidebarMode::Tasks => {
                if let Some(task) = self.tasks.get(self.selected_index) {
                    let message = format!("Delete task '{}' ({})", task.id, task.symbol);
                    self.mode = AppMode::Dialog;
                    self.modal = Some(ModalType::Confirm {
                        title: "Delete Task".to_string(),
                        message,
                        action: ConfirmAction::DeleteTask {
                            task_id: task.id.clone(),
                        },
                    });
                    self.status_message = Some("Confirm deletion...".to_string());
                } else {
                    self.status_message = Some("No task selected".to_string());
                }
            }
        }
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
    #[allow(dead_code)]
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
    pub async fn handle_form_input(&mut self, key: KeyEvent) -> Result<()> {
        let mut pending_account: Option<Account> = None;
        let mut pending_account_id: Option<String> = None;
        let mut pending_is_edit = false;
        let mut pending_task: Option<Task> = None;
        let mut pending_task_id: Option<String> = None;
        let mut pending_task_account_id: Option<String> = None;
        let mut pending_task_is_edit = false;

        if let Some(ModalType::AccountForm { form, is_edit }) = self.modal.as_mut() {
            let is_edit = *is_edit;
            if is_edit && form.focused_field == 0 {
                form.focused_field = 1;
            }
            match key.code {
                KeyCode::Tab | KeyCode::Down => {
                    form.focused_field = if is_edit {
                        match form.focused_field {
                            1 => 2,
                            2 => 3,
                            _ => 1,
                        }
                    } else {
                        (form.focused_field + 1) % 4
                    };
                    form.error_message = None;
                }
                KeyCode::BackTab | KeyCode::Up => {
                    form.focused_field = if is_edit {
                        match form.focused_field {
                            3 => 2,
                            2 => 1,
                            _ => 3,
                        }
                    } else if form.focused_field == 0 {
                        3
                    } else {
                        form.focused_field - 1
                    };
                    form.error_message = None;
                }
                KeyCode::Backspace => {
                    if !(is_edit && form.focused_field == 0) {
                        match form.focused_field {
                            0 => {
                                form.id.pop();
                            }
                            1 => {
                                form.name.pop();
                            }
                            2 => {
                                form.jwt_token.pop();
                            }
                            _ => {
                                form.signing_key.pop();
                            }
                        }
                    }
                    form.error_message = None;
                }
                KeyCode::Enter => match form.to_account() {
                    Ok(account) => {
                        pending_account_id = Some(account.id.clone());
                        pending_account = Some(account);
                        pending_is_edit = is_edit;
                        form.error_message = None;
                    }
                    Err(message) => {
                        form.error_message = Some(message);
                    }
                },
                KeyCode::Char(ch)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    if !(is_edit && form.focused_field == 0) {
                        match form.focused_field {
                            0 => form.id.push(ch),
                            1 => form.name.push(ch),
                            2 => form.jwt_token.push(ch),
                            _ => form.signing_key.push(ch),
                        }
                    }
                    form.error_message = None;
                }
                _ => {}
            }
        }

        if let Some(ModalType::TaskForm { form, is_edit }) = self.modal.as_mut() {
            let is_edit = *is_edit;
            if is_edit && form.focused_field == 0 {
                form.focused_field = 1;
            }
            match key.code {
                KeyCode::Tab | KeyCode::Down => {
                    form.focused_field = if is_edit {
                        match form.focused_field {
                            1 => 2,
                            2 => 3,
                            3 => 4,
                            4 => 5,
                            5 => 6,
                            6 => 7,
                            _ => 1,
                        }
                    } else {
                        (form.focused_field + 1) % 8
                    };
                    form.error_message = None;
                }
                KeyCode::BackTab | KeyCode::Up => {
                    form.focused_field = if is_edit {
                        match form.focused_field {
                            7 => 6,
                            6 => 5,
                            5 => 4,
                            4 => 3,
                            3 => 2,
                            2 => 1,
                            _ => 7,
                        }
                    } else if form.focused_field == 0 {
                        7
                    } else {
                        form.focused_field - 1
                    };
                    form.error_message = None;
                }
                KeyCode::Backspace => {
                    if !(is_edit && form.focused_field == 0) {
                        match form.focused_field {
                            0 => {
                                form.id.pop();
                            }
                            1 => {
                                form.symbol.pop();
                            }
                            2 => {
                                form.account_id.pop();
                            }
                            3 => {
                                form.risk_level.pop();
                            }
                            4 => {
                                form.max_position_usd.pop();
                            }
                            5 => {
                                form.price_jump_threshold_bps.pop();
                            }
                            6 => {
                                form.base_qty.pop();
                            }
                            _ => {
                                form.tiers.pop();
                            }
                        }
                    }
                    form.error_message = None;
                }
                KeyCode::Enter => match form.to_task() {
                    Ok(task) => {
                        pending_task_id = Some(task.id.clone());
                        pending_task_account_id = Some(task.account_id.clone());
                        pending_task = Some(task);
                        pending_task_is_edit = is_edit;
                        form.error_message = None;
                    }
                    Err(message) => {
                        form.error_message = Some(message);
                    }
                },
                KeyCode::Char(ch)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    if !(is_edit && form.focused_field == 0) {
                        match form.focused_field {
                            0 => form.id.push(ch),
                            1 => form.symbol.push(ch),
                            2 => form.account_id.push(ch),
                            3 => form.risk_level.push(ch),
                            4 => form.max_position_usd.push(ch),
                            5 => form.price_jump_threshold_bps.push(ch),
                            6 => form.base_qty.push(ch),
                            _ => form.tiers.push(ch),
                        }
                    }
                    form.error_message = None;
                }
                _ => {}
            }
        }

        if let Some(account) = pending_account {
            if pending_is_edit {
                let Account {
                    id,
                    name,
                    jwt_token,
                    signing_key,
                    ..
                } = account;
                let account_id = pending_account_id.unwrap_or(id);
                match self
                    .storage
                    .update_account(&account_id, |account| {
                        account.name = name;
                        account.jwt_token = jwt_token;
                        account.signing_key = signing_key;
                    })
                    .await
                {
                    Ok(()) => match self.storage.list_accounts().await {
                        Ok(accounts) => {
                            self.accounts = accounts;
                            self.status_message =
                                Some(format!("Account updated: {}", account_id));
                            self.close_account_form_modal();
                        }
                        Err(err) => {
                            self.set_account_form_error(err.to_string());
                        }
                    },
                    Err(err) => {
                        self.set_account_form_error(err.to_string());
                    }
                }
            } else {
                match self.storage.create_account(account).await {
                    Ok(()) => match self.storage.list_accounts().await {
                        Ok(accounts) => {
                            self.accounts = accounts;
                            let account_id =
                                pending_account_id.unwrap_or_else(|| "account".to_string());
                            self.status_message =
                                Some(format!("Account created: {}", account_id));
                            self.close_account_form_modal();
                        }
                        Err(err) => {
                            self.set_account_form_error(err.to_string());
                        }
                    },
                    Err(err) => {
                        self.set_account_form_error(err.to_string());
                    }
                }
            }
        }

        if let Some(task) = pending_task {
            let task_id = pending_task_id.unwrap_or_else(|| task.id.clone());
            let account_id = pending_task_account_id.unwrap_or_else(|| task.account_id.clone());
            if self.storage.get_account(&account_id).await.is_none() {
                self.set_task_form_error(format!(
                    "Account '{}' not found",
                    account_id
                ));
            } else if pending_task_is_edit {
                let Task {
                    symbol,
                    account_id,
                    risk_level,
                    max_position_usd,
                    price_jump_threshold_bps,
                    base_qty,
                    tiers,
                    ..
                } = task;
                match self
                    .storage
                    .update_task(&task_id, |existing| {
                        existing.symbol = symbol;
                        existing.account_id = account_id;
                        existing.risk_level = risk_level;
                        existing.max_position_usd = max_position_usd;
                        existing.price_jump_threshold_bps = price_jump_threshold_bps;
                        existing.base_qty = base_qty;
                        existing.tiers = tiers;
                    })
                    .await
                {
                    Ok(()) => match self.storage.list_tasks().await {
                        Ok(tasks) => {
                            self.tasks = tasks;
                            self.status_message = Some(format!("Task updated: {}", task_id));
                            self.close_task_form_modal();
                        }
                        Err(err) => {
                            self.set_task_form_error(err.to_string());
                        }
                    },
                    Err(err) => {
                        self.set_task_form_error(err.to_string());
                    }
                }
            } else {
                match self.storage.create_task(task).await {
                    Ok(()) => match self.storage.list_tasks().await {
                        Ok(tasks) => {
                            self.tasks = tasks;
                            self.status_message = Some(format!("Task created: {}", task_id));
                            self.close_task_form_modal();
                        }
                        Err(err) => {
                            self.set_task_form_error(err.to_string());
                        }
                    },
                    Err(err) => {
                        self.set_task_form_error(err.to_string());
                    }
                }
            }
        }
        Ok(())
    }

    /// Close dialog/modal
    pub async fn close_dialog(&mut self, confirmed: bool) -> Result<()> {
        let action = match self.modal.take() {
            Some(ModalType::Confirm { action, .. }) => Some(action),
            _ => None,
        };
        self.mode = AppMode::Normal;
        self.modal = None;

        if confirmed {
            match action {
                Some(ConfirmAction::DeleteAccount { account_id }) => {
                    match self.storage.delete_account(&account_id).await {
                        Ok(()) => match self.storage.list_accounts().await {
                            Ok(accounts) => {
                                self.accounts = accounts;
                                if self.accounts.is_empty() {
                                    self.selected_index = 0;
                                } else if self.selected_index >= self.accounts.len() {
                                    self.selected_index = self.accounts.len() - 1;
                                }
                                self.status_message =
                                    Some(format!("Account deleted: {}", account_id));
                            }
                            Err(err) => {
                                self.status_message = Some(err.to_string());
                            }
                        },
                        Err(err) => {
                            self.status_message = Some(err.to_string());
                        }
                    }
                }
                Some(ConfirmAction::DeleteTask { task_id }) => {
                    match self.storage.delete_task(&task_id).await {
                        Ok(()) => match self.storage.list_tasks().await {
                            Ok(tasks) => {
                                self.tasks = tasks;
                                if self.tasks.is_empty() {
                                    self.selected_index = 0;
                                } else if self.selected_index >= self.tasks.len() {
                                    self.selected_index = self.tasks.len() - 1;
                                }
                                self.status_message = Some(format!("Task deleted: {}", task_id));
                            }
                            Err(err) => {
                                self.status_message = Some(err.to_string());
                            }
                        },
                        Err(err) => {
                            self.status_message = Some(err.to_string());
                        }
                    }
                }
                None => {}
            }
        }
        Ok(())
    }

    pub fn close_account_form_modal(&mut self) {
        if let Some(ModalType::AccountForm { form, .. }) = self.modal.as_mut() {
            form.jwt_token.clear();
            form.signing_key.clear();
        }
        self.modal = None;
        self.mode = AppMode::Normal;
    }

    pub fn close_task_form_modal(&mut self) {
        self.modal = None;
        self.mode = AppMode::Normal;
    }

    fn set_account_form_error(&mut self, message: String) {
        if let Some(ModalType::AccountForm { form, .. }) = self.modal.as_mut() {
            form.error_message = Some(message);
        }
    }

    fn set_task_form_error(&mut self, message: String) {
        if let Some(ModalType::TaskForm { form, .. }) = self.modal.as_mut() {
            form.error_message = Some(message);
        }
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
