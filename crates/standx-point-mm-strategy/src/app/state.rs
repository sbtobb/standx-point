/// **Input**: Storage access, modal/form input events, sidebar selections.
/// **Output**: Mutated AppState for UI rendering and persisted accounts/tasks.
/// **Position**: TUI application state and input-handling coordinator.
/// **Update**: Add account-form modal state and input handling.
/// **Update**: Implement account edit flow with read-only ID handling.
/// **Update**: Add confirm action context for account deletion.
/// **Update**: Implement task form create flow with validation.
/// **Update**: Implement task edit flow with storage updates.
/// **Update**: Add task deletion confirmation and storage removal.
/// **Update**: Track quit confirmations and exit requests.
/// **Update**: Restrict account selection to single-select in task form.
/// **Update**: Fix pending task account updates and account selection matching.
use crate::state::storage::{Account, Storage, Task};
use standx_point_mm_strategy::task::TaskRuntimeStatus;
use crate::ui::components::account_form::{
    AccountChain, AccountField, AccountForm, AccountSeed,
};
use crate::ui::components::task_form::{AccountOption, TaskField, TaskForm};
use anyhow::{Result, anyhow};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use standx_point_adapter::auth::{
    AuthManager, EvmWalletSigner, SolanaWalletSigner, WalletSigner,
};
use standx_point_adapter::StandxClient;
use standx_point_adapter::types::{Balance, Chain, Position};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;

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
    /// Quit the application after graceful shutdown
    #[allow(dead_code)]
    Quit,
}

/// Modal types that can be displayed
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub(crate) struct PendingTaskForm {
    form: TaskForm,
    is_edit: bool,
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
    /// Pending task form when creating an account mid-flow
    pub pending_task_form: Option<PendingTaskForm>,
    /// Exit requested via UI (graceful shutdown path)
    pub exit_requested: bool,
    /// Reference to storage for data access
    storage: Arc<Storage>,
    /// Cached accounts for synchronous access in render
    pub accounts: Vec<Account>,
    /// Cached tasks for synchronous access in render
    pub tasks: Vec<Task>,
    /// Runtime task status snapshot from TaskManager
    pub runtime_status: HashMap<String, TaskRuntimeStatus>,
    /// Cached account detail data fetched from StandX
    pub account_details: HashMap<String, AccountDetail>,
    /// Pending 'g' key prefix state (number of ticks remaining before timeout)
    pub pending_g_ticks: u8,
    /// Whether to show full credentials (jwt_token, signing_key) or mask them
    pub show_credentials: bool,
    /// Spinner animation ticks remaining
    pub spinner_ticks: u8,
    /// Current spinner frame index
    pub spinner_frame: u8,
    /// Whether next character input replaces entire field (select-all semantics for forms)
    pub replace_on_next_input: bool,
}

#[derive(Debug, Clone)]
pub struct AccountDetail {
    pub balance: Option<Balance>,
    pub positions: Vec<Position>,
    pub last_updated: Option<Instant>,
    pub last_error: Option<String>,
}

impl AccountDetail {
    pub fn empty() -> Self {
        Self {
            balance: None,
            positions: Vec::new(),
            last_updated: None,
            last_error: None,
        }
    }
}

impl AppState {
    /// Create new application state
    pub async fn new(storage: Arc<Storage>) -> Result<Self> {
        let accounts = storage.list_accounts().await?;
        let tasks = storage.list_tasks().await?;
        Ok(Self {
            mode: AppMode::Normal,
            focused_pane: Pane::Sidebar,
            sidebar_mode: SidebarMode::Tasks,
            selected_index: 0,
            status_message: Some("Press F1 for help".to_string()),
            keypress_flash: None,
            show_help: false,
            modal: None,
            pending_task_form: None,
            exit_requested: false,
            storage,
            accounts,
            tasks,
            runtime_status: HashMap::new(),
            account_details: HashMap::new(),
            pending_g_ticks: 0,
            show_credentials: false, // Default to masking credentials
            spinner_ticks: 0,
            spinner_frame: 0,
            replace_on_next_input: false,
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
                let mut form = TaskForm::new();
                self.update_task_form_accounts(&mut form);
                self.modal = Some(ModalType::TaskForm {
                    form,
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
                    form.focused_field = AccountField::Name;
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
                    form.focused_field = TaskField::Symbol;
                    self.update_task_form_accounts(&mut form);
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
                    let message = format!("Delete account '{}' ({})", account.id, account.name);
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
        let mut pending_account_seed: Option<AccountSeed> = None;
        let mut pending_account_update: Option<(String, String, AccountChain)> = None;
        let mut pending_task: Option<Task> = None;
        let mut pending_task_id: Option<String> = None;
        let mut pending_task_account_id: Option<String> = None;
        let mut pending_task_is_edit = false;
        let mut open_account_form_from_task = false;
        let mut pending_task_form_for_account: Option<PendingTaskForm> = None;

        if let Some(ModalType::AccountForm { form, is_edit }) = self.modal.as_mut() {
            let is_edit = *is_edit;
            match key.code {
                KeyCode::Tab => {
                    if is_edit {
                        form.focused_field = match form.focused_field {
                            AccountField::Chain => AccountField::Name,
                            _ => AccountField::Chain,
                        };
                    } else {
                        form.focused_field = form.focused_field.next();
                    }
                    form.error_message = None;
                    self.replace_on_next_input = false;
                }
                KeyCode::BackTab => {
                    if is_edit {
                        form.focused_field = match form.focused_field {
                            AccountField::Chain => AccountField::Name,
                            _ => AccountField::Chain,
                        };
                    } else {
                        form.focused_field = form.focused_field.prev();
                    }
                    form.error_message = None;
                    self.replace_on_next_input = false;
                }
                KeyCode::Down | KeyCode::Up => {
                    if form.focused_field == AccountField::Chain {
                        let selected = form.chain_select.handle_key(key);
                        if let Some(value) = selected {
                            form.chain = value;
                        } else if let Some(option) = form
                            .chain_select
                            .options()
                            .get(form.chain_select.cursor_index())
                        {
                            form.chain = *option;
                        }
                    } else if is_edit {
                        form.focused_field = match form.focused_field {
                            AccountField::Chain => AccountField::Name,
                            _ => AccountField::Chain,
                        };
                    } else {
                        if matches!(key.code, KeyCode::Down) {
                            form.focused_field = form.focused_field.next();
                        } else {
                            form.focused_field = form.focused_field.prev();
                        }
                    }
                    form.error_message = None;
                    self.replace_on_next_input = false;
                }
                KeyCode::Backspace => {
                    if matches!(
                        form.focused_field,
                        AccountField::Name | AccountField::PrivateKey
                    ) {
                        if self.replace_on_next_input {
                            match form.focused_field {
                                AccountField::Name => form.name.clear(),
                                AccountField::PrivateKey => form.private_key.clear(),
                                _ => {}
                            }
                            self.replace_on_next_input = false;
                        } else {
                            match form.focused_field {
                                AccountField::Name => {
                                    form.name.pop();
                                }
                                AccountField::PrivateKey => {
                                    form.private_key.pop();
                                }
                                _ => {}
                            }
                        }
                    }
                    form.error_message = None;
                }
                KeyCode::Enter => {
                    if form.focused_field == AccountField::Chain {
                        if let Some(value) = form.chain_select.handle_key(key) {
                            form.chain = value;
                        }
                        form.error_message = None;
                    } else if is_edit {
                        match form.validate_name() {
                            Ok(()) => {
                                if let Some(account_id) = form.account_id.clone() {
                                    pending_account_update =
                                        Some((account_id, form.name.clone(), form.chain));
                                    form.error_message = None;
                                } else {
                                    form.error_message = Some("Account ID missing".to_string());
                                }
                            }
                            Err(message) => {
                                form.error_message = Some(message);
                            }
                        }
                    } else {
                        match form.to_account_seed() {
                            Ok(seed) => {
                                pending_account_seed = Some(seed);
                                form.error_message = None;
                            }
                            Err(message) => {
                                form.error_message = Some(message);
                            }
                        }
                    }
                }
                KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    match form.focused_field {
                        AccountField::Name => form.name.clear(),
                        AccountField::PrivateKey => form.private_key.clear(),
                        _ => {}
                    }
                    self.replace_on_next_input = false;
                    form.error_message = None;
                }
                KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if matches!(
                        form.focused_field,
                        AccountField::Name | AccountField::PrivateKey
                    ) {
                        self.replace_on_next_input = true;
                    }
                    form.error_message = None;
                }
                KeyCode::Char('j') | KeyCode::Char('k') => {
                    if form.focused_field == AccountField::Chain {
                        let selected = form.chain_select.handle_key(key);
                        if let Some(value) = selected {
                            form.chain = value;
                        } else if let Some(option) = form
                            .chain_select
                            .options()
                            .get(form.chain_select.cursor_index())
                        {
                            form.chain = *option;
                        }
                    } else if is_edit {
                        form.focused_field = match form.focused_field {
                            AccountField::Chain => AccountField::Name,
                            _ => AccountField::Chain,
                        };
                    }
                    form.error_message = None;
                }
                KeyCode::Char(ch)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    if matches!(
                        form.focused_field,
                        AccountField::Name | AccountField::PrivateKey
                    ) {
                        if self.replace_on_next_input {
                            match form.focused_field {
                                AccountField::Name => {
                                    form.name.clear();
                                    form.name.push(ch);
                                }
                                AccountField::PrivateKey => {
                                    form.private_key.clear();
                                    form.private_key.push(ch);
                                }
                                _ => {}
                            }
                            self.replace_on_next_input = false;
                        } else {
                            match form.focused_field {
                                AccountField::Name => form.name.push(ch),
                                AccountField::PrivateKey => form.private_key.push(ch),
                                _ => {}
                            }
                        }
                    }
                    form.error_message = None;
                }
                _ => {}
            }
        }

        if let Some(ModalType::TaskForm { form, is_edit }) = self.modal.as_mut() {
            let is_edit = *is_edit;
            match key.code {
                KeyCode::Tab => {
                    form.focused_field = form.focused_field.next();
                    form.error_message = None;
                }
                KeyCode::BackTab => {
                    form.focused_field = form.focused_field.prev();
                    form.error_message = None;
                }
                KeyCode::Down | KeyCode::Up => {
                    match form.focused_field {
                        TaskField::Symbol => {
                            let selected = form.symbol_select.handle_key(key);
                            if let Some(value) = selected {
                                form.symbol = value;
                            } else if let Some(option) = form
                                .symbol_select
                                .options()
                                .get(form.symbol_select.cursor_index())
                            {
                                form.symbol = option.clone();
                            }
                        }
                        TaskField::RiskLevel => {
                            let selected = form.risk_level_select.handle_key(key);
                            if let Some(value) = selected {
                                form.risk_level = value;
                            } else if let Some(option) = form
                                .risk_level_select
                                .options()
                                .get(form.risk_level_select.cursor_index())
                            {
                                form.risk_level = option.clone();
                            }
                        }
                        TaskField::AccountId => {
                            let selected = form.account_select.handle_key(key);
                            if let Some(value) = selected {
                                match value {
                                    AccountOption::Existing { id, .. } => {
                                        form.account_id = id;
                                    }
                                    AccountOption::CreateNew => {
                                        form.account_id.clear();
                                    }
                                }
                            } else if let Some(option) = form
                                .account_select
                                .options()
                                .get(form.account_select.cursor_index())
                            {
                                match option {
                                    AccountOption::Existing { id, .. } => {
                                        form.account_id = id.clone();
                                    }
                                    AccountOption::CreateNew => {
                                        form.account_id.clear();
                                    }
                                }
                            }
                        }
                        _ => {
                            if matches!(key.code, KeyCode::Down) {
                                form.focused_field = form.focused_field.next();
                            } else {
                                form.focused_field = form.focused_field.prev();
                            }
                        }
                    }
                    form.error_message = None;
                }
                KeyCode::Backspace => {
                    match form.focused_field {
                        TaskField::MaxPositionUsd => {
                            form.max_position_usd.pop();
                        }
                        TaskField::PriceJumpThresholdBps => {
                            form.price_jump_threshold_bps.pop();
                        }
                        _ => {}
                    }
                    form.error_message = None;
                }
                KeyCode::Enter => match form.focused_field {
                    TaskField::Symbol => {
                        if let Some(value) = form.symbol_select.handle_key(key) {
                            form.symbol = value;
                        }
                    }
                    TaskField::RiskLevel => {
                        if let Some(value) = form.risk_level_select.handle_key(key) {
                            form.risk_level = value;
                        }
                    }
                    TaskField::AccountId => {
                        if let Some(value) = form.account_select.handle_key(key) {
                            match value {
                                AccountOption::Existing { id, .. } => {
                                    form.account_id = id;
                                }
                                AccountOption::CreateNew => {
                                    pending_task_form_for_account = Some(PendingTaskForm {
                                        form: form.clone(),
                                        is_edit,
                                    });
                                    open_account_form_from_task = true;
                                }
                            }
                        }
                    }
                    _ => match form.to_task() {
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
                },
                KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    match form.focused_field {
                        TaskField::MaxPositionUsd => form.max_position_usd.clear(),
                        TaskField::PriceJumpThresholdBps => form.price_jump_threshold_bps.clear(),
                        _ => {}
                    }
                    form.error_message = None;
                }
                KeyCode::Char('j') | KeyCode::Char('k')
                    if matches!(
                        form.focused_field,
                        TaskField::Symbol | TaskField::RiskLevel | TaskField::AccountId
                    ) =>
                {
                    match form.focused_field {
                        TaskField::Symbol => {
                            let selected = form.symbol_select.handle_key(key);
                            if let Some(value) = selected {
                                form.symbol = value;
                            } else if let Some(option) = form
                                .symbol_select
                                .options()
                                .get(form.symbol_select.cursor_index())
                            {
                                form.symbol = option.clone();
                            }
                        }
                        TaskField::RiskLevel => {
                            let selected = form.risk_level_select.handle_key(key);
                            if let Some(value) = selected {
                                form.risk_level = value;
                            } else if let Some(option) = form
                                .risk_level_select
                                .options()
                                .get(form.risk_level_select.cursor_index())
                            {
                                form.risk_level = option.clone();
                            }
                        }
                        TaskField::AccountId => {
                            let selected = form.account_select.handle_key(key);
                            if let Some(value) = selected {
                                match value {
                                    AccountOption::Existing { id, .. } => {
                                        form.account_id = id;
                                    }
                                    AccountOption::CreateNew => {
                                        form.account_id.clear();
                                    }
                                }
                            } else if let Some(option) = form
                                .account_select
                                .options()
                                .get(form.account_select.cursor_index())
                            {
                                match option {
                                    AccountOption::Existing { id, .. } => {
                                        form.account_id = id.clone();
                                    }
                                    AccountOption::CreateNew => {
                                        form.account_id.clear();
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    form.error_message = None;
                }
                KeyCode::Char(ch)
                    if !key.modifiers.contains(KeyModifiers::CONTROL)
                        && !key.modifiers.contains(KeyModifiers::ALT) =>
                {
                    match form.focused_field {
                        TaskField::MaxPositionUsd => form.max_position_usd.push(ch),
                        TaskField::PriceJumpThresholdBps => form.price_jump_threshold_bps.push(ch),
                        _ => {}
                    }
                    form.error_message = None;
                }
                _ => {}
            }
        }

        if let Some((account_id, name, chain)) = pending_account_update {
            let chain = match chain {
                AccountChain::Bsc => Chain::Bsc,
                AccountChain::Solana => Chain::Solana,
            };
            match self
                .storage
                .update_account(&account_id, |account| {
                    account.name = name;
                    account.chain = Some(chain);
                })
                .await
            {
                Ok(()) => match self.storage.list_accounts().await {
                    Ok(accounts) => {
                        self.accounts = accounts;
                        self.status_message = Some(format!("Account updated: {}", account_id));
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

        if let Some(seed) = pending_account_seed {
            match self.build_account_from_seed(seed).await {
                Ok(account) => match self.storage.create_account(account.clone()).await {
                    Ok(()) => match self.storage.list_accounts().await {
                        Ok(accounts) => {
                            self.accounts = accounts;
                            self.set_pending_task_account(account.id.clone());
                            self.status_message =
                                Some(format!("Account created: {}", account.id));
                            self.close_account_form_modal();
                        }
                        Err(err) => {
                            self.set_account_form_error(err.to_string());
                        }
                    },
                    Err(err) => {
                        self.set_account_form_error(err.to_string());
                    }
                },
                Err(err) => {
                    self.set_account_form_error(err.to_string());
                }
            }
        }

        if let Some(task) = pending_task {
            let task_id = pending_task_id.unwrap_or_else(|| task.id.clone());
            let account_id = pending_task_account_id.unwrap_or_else(|| task.account_id.clone());
            if self.storage.get_account(&account_id).await.is_none() {
                self.set_task_form_error(format!("Account '{}' not found", account_id));
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

        if open_account_form_from_task {
            self.pending_task_form = pending_task_form_for_account;
            self.modal = Some(ModalType::AccountForm {
                form: AccountForm::new(),
                is_edit: false,
            });
            self.mode = AppMode::Insert;
            self.status_message = Some("Creating new account...".to_string());
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
                Some(ConfirmAction::Quit) => {
                    self.exit_requested = true;
                    self.status_message = Some("Shutting down...".to_string());
                }
                None => {}
            }
        }
        Ok(())
    }

    pub fn close_account_form_modal(&mut self) {
        if let Some(ModalType::AccountForm { form, .. }) = self.modal.as_mut() {
            form.private_key.clear();
        }
        if let Some(mut pending) = self.pending_task_form.take() {
            self.update_task_form_accounts(&mut pending.form);
            self.modal = Some(ModalType::TaskForm {
                form: pending.form,
                is_edit: pending.is_edit,
            });
            self.mode = AppMode::Insert;
        } else {
            self.modal = None;
            self.mode = AppMode::Normal;
        }
        self.replace_on_next_input = false;
    }

    pub fn close_task_form_modal(&mut self) {
        self.modal = None;
        self.mode = AppMode::Normal;
        self.replace_on_next_input = false;
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

    fn update_task_form_accounts(&self, form: &mut TaskForm) {
        let mut options: Vec<AccountOption> = self
            .accounts
            .iter()
            .map(|account| AccountOption::Existing {
                id: account.id.clone(),
                name: account.name.clone(),
            })
            .collect();
        options.push(AccountOption::CreateNew);
        form.account_select.set_options(options);

        if let Some(index) = form.account_select.options().iter().position(|option| {
            matches!(
                option,
                AccountOption::Existing { id, .. } if id.as_str() == form.account_id.as_str()
            )
        }) {
            form.account_select.set_selected_index(index);
        } else if form.account_id.is_empty() {
            if let Some((index, option)) = form
                .account_select
                .options()
                .iter()
                .enumerate()
                .find(|(_, option)| matches!(option, AccountOption::Existing { .. }))
            {
                if let AccountOption::Existing { id, .. } = option {
                    form.account_id = id.clone();
                }
                form.account_select.set_selected_index(index);
            }
        }
    }

    fn set_pending_task_account(&mut self, account_id: String) {
        if let Some(mut pending) = self.pending_task_form.take() {
            pending.form.account_id = account_id;
            self.update_task_form_accounts(&mut pending.form);
            self.pending_task_form = Some(pending);
        }
    }

    async fn build_account_from_seed(&self, seed: AccountSeed) -> Result<Account> {
        let client = StandxClient::new()
            .map_err(|err| anyhow!("create StandxClient failed: {err}"))?;
        let auth = AuthManager::new(client);
        let (wallet_address, login_response) = match seed.chain {
            AccountChain::Bsc => {
                let wallet = EvmWalletSigner::new(&seed.private_key)
                    .map_err(|err| anyhow!("invalid EVM private key: {err}"))?;
                let address = wallet.address().to_string();
                let login = auth
                    .authenticate(&wallet, 7 * 24 * 60 * 60)
                    .await
                    .map_err(|err| anyhow!("authenticate failed: {err}"))?;
                (address, login)
            }
            AccountChain::Solana => {
                let wallet = SolanaWalletSigner::new(&seed.private_key)
                    .map_err(|err| anyhow!("invalid Solana private key: {err}"))?;
                let address = wallet.address().to_string();
                let login = auth
                    .authenticate(&wallet, 7 * 24 * 60 * 60)
                    .await
                    .map_err(|err| anyhow!("authenticate failed: {err}"))?;
                (address, login)
            }
        };

        let signer = auth
            .key_manager()
            .get_or_create_signer(&wallet_address)
            .map_err(|err| anyhow!("load ed25519 signer failed: {err}"))?;
        let signing_key = STANDARD.encode(signer.secret_key_bytes());

        let chain = match seed.chain {
            AccountChain::Bsc => Chain::Bsc,
            AccountChain::Solana => Chain::Solana,
        };

        Ok(Account::new(
            wallet_address,
            seed.name,
            login_response.token,
            signing_key,
            Some(chain),
        ))
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

    #[tokio::test]
    async fn test_ctrl_a_select_all_replace() {
        let temp_dir = create_unique_temp_dir();
        let storage = Arc::new(Storage::new_in_dir(&temp_dir).await.unwrap());
        let mut state = AppState::new(storage.clone()).await.unwrap();

        // Open account form
        state.modal = Some(ModalType::AccountForm {
            form: AccountForm::new(),
            is_edit: false,
        });

        // Simulate typing "abc" into name field
        if let Some(ModalType::AccountForm { form, is_edit: _ }) = state.modal.as_mut() {
            form.name = "abc".to_string();
            form.focused_field = AccountField::Name;
        }

        // Press Ctrl+A to select all
        let ctrl_a_event = KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::CONTROL,
            kind: ratatui::crossterm::event::KeyEventKind::Press,
            state: ratatui::crossterm::event::KeyEventState::NONE,
        };
        state.handle_form_input(ctrl_a_event).await.unwrap();

        // Verify replace_on_next_input is true
        assert!(state.replace_on_next_input);

        // Press 'x' to replace
        let x_event = KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
            kind: ratatui::crossterm::event::KeyEventKind::Press,
            state: ratatui::crossterm::event::KeyEventState::NONE,
        };
        state.handle_form_input(x_event).await.unwrap();

        // Verify name is "x" and flag is false
        if let Some(ModalType::AccountForm { form, .. }) = state.modal.as_ref() {
            assert_eq!(form.name, "x");
        }
        assert!(!state.replace_on_next_input);

        cleanup_temp_dir(&temp_dir);
    }

    #[tokio::test]
    async fn test_ctrl_a_select_all_backspace() {
        let temp_dir = create_unique_temp_dir();
        let storage = Arc::new(Storage::new_in_dir(&temp_dir).await.unwrap());
        let mut state = AppState::new(storage.clone()).await.unwrap();

        // Open account form
        state.modal = Some(ModalType::AccountForm {
            form: AccountForm::new(),
            is_edit: false,
        });

        // Simulate typing "abc" into name field
        if let Some(ModalType::AccountForm { form, is_edit: _ }) = state.modal.as_mut() {
            form.name = "abc".to_string();
            form.focused_field = AccountField::Name;
        }

        // Press Ctrl+A to select all
        let ctrl_a_event = KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::CONTROL,
            kind: ratatui::crossterm::event::KeyEventKind::Press,
            state: ratatui::crossterm::event::KeyEventState::NONE,
        };
        state.handle_form_input(ctrl_a_event).await.unwrap();

        // Verify replace_on_next_input is true
        assert!(state.replace_on_next_input);

        // Press Backspace to clear
        let backspace_event = KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::NONE,
            kind: ratatui::crossterm::event::KeyEventKind::Press,
            state: ratatui::crossterm::event::KeyEventState::NONE,
        };
        state.handle_form_input(backspace_event).await.unwrap();

        // Verify name is empty and flag is false
        if let Some(ModalType::AccountForm { form, .. }) = state.modal.as_ref() {
            assert!(form.name.is_empty());
        }
        assert!(!state.replace_on_next_input);

        cleanup_temp_dir(&temp_dir);
    }

    #[tokio::test]
    async fn test_ctrl_a_edit_mode_id_field_no_op() {
        let temp_dir = create_unique_temp_dir();
        let storage = Arc::new(Storage::new_in_dir(&temp_dir).await.unwrap());
        let mut state = AppState::new(storage.clone()).await.unwrap();

        // Open account form in edit mode
        let account = Account::new(
            "acc-1".to_string(),
            "Test Account".to_string(),
            "jwt".to_string(),
            "signing".to_string(),
            None,
        );
        state.modal = Some(ModalType::AccountForm {
            form: AccountForm::from_account(&account),
            is_edit: true,
        });

        // Press Ctrl+A to select all for the name field
        let ctrl_a_event = KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::CONTROL,
            kind: ratatui::crossterm::event::KeyEventKind::Press,
            state: ratatui::crossterm::event::KeyEventState::NONE,
        };
        state.handle_form_input(ctrl_a_event).await.unwrap();
        assert!(state.replace_on_next_input);

        cleanup_temp_dir(&temp_dir);
    }
}
