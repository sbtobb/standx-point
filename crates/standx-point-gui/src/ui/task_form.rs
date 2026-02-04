/// **Input**: Database handle, Task/Account models, gpui-component inputs/selects, TaskConfig.
/// **Output**: TaskForm modal UI with validation and persistence hooks.
/// **Position**: UI modal for task create/edit flows.
/// **Update**: When task fields, validation rules, or persistence logic changes.
use crate::db::Database;
use crate::state::{Account, Task, TaskStatus};
use gpui::*;
use gpui_component::input::{Input, InputState};
use gpui_component::select::{Select, SelectItem, SelectState};
use gpui_component::IndexPath;
use standx_point_mm_strategy::config::{CredentialsConfig, RiskConfig, SizingConfig, TaskConfig};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormMode {
    Create,
    Edit,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
}

const SYMBOL_OPTIONS: [&str; 4] = ["BTC-USD", "ETH-USD", "XAG-USD", "XAU-USD"];

#[derive(Clone)]
struct AccountOption {
    id: String,
    label: String,
}

impl SelectItem for AccountOption {
    type Value = String;

    fn title(&self) -> SharedString {
        self.label.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.id
    }
}

pub struct TaskForm {
    mode: FormMode,
    task_id: Option<String>,
    name: String,
    symbol: String,
    account_id: String,
    config_json: String,
    accounts: Vec<Account>,
    name_input: Option<Entity<InputState>>,
    symbol_select: Option<Entity<SelectState<Vec<&'static str>>>>,
    account_select: Option<Entity<SelectState<Vec<AccountOption>>>>,
    config_input: Option<Entity<InputState>>,
    db: Option<Arc<Database>>,
    on_save: Option<Box<dyn Fn(&mut Window, &mut Context<Self>) + 'static>>,
    on_cancel: Option<Box<dyn Fn(&mut Window, &mut Context<Self>) + 'static>>,
}

impl TaskForm {
    pub fn new_create() -> Self {
        let default_config = TaskConfig {
            id: "".to_string(),
            symbol: "".to_string(),
            credentials: CredentialsConfig {
                jwt_token: "".to_string(),
                signing_key: "".to_string(),
            },
            risk: RiskConfig {
                level: "conservative".to_string(),
                max_position_usd: "1000".to_string(),
                price_jump_threshold_bps: 10,
            },
            sizing: SizingConfig {
                base_qty: "0.01".to_string(),
                tiers: 3,
            },
        };

        Self {
            mode: FormMode::Create,
            task_id: None,
            name: String::new(),
            symbol: SYMBOL_OPTIONS[0].to_string(),
            account_id: String::new(),
            config_json: serde_json::to_string_pretty(&default_config).unwrap_or_default(),
            accounts: Vec::new(),
            name_input: None,
            symbol_select: None,
            account_select: None,
            config_input: None,
            db: None,
            on_save: None,
            on_cancel: None,
        }
    }

    pub fn new_edit(task: &Task) -> Self {
        Self {
            mode: FormMode::Edit,
            task_id: Some(task.id.clone()),
            name: task.name.clone(),
            symbol: task.symbol.clone(),
            account_id: task.account_id.clone(),
            config_json: serde_json::to_string_pretty(&task.config).unwrap_or_default(),
            accounts: Vec::new(),
            name_input: None,
            symbol_select: None,
            account_select: None,
            config_input: None,
            db: None,
            on_save: None,
            on_cancel: None,
        }
    }

    pub fn with_accounts(mut self, accounts: Vec<Account>) -> Self {
        self.apply_accounts(accounts);
        self
    }

    pub fn set_accounts(&mut self, accounts: Vec<Account>) {
        self.apply_accounts(accounts);
    }

    pub fn with_db(mut self, db: Arc<Database>) -> Self {
        self.db = Some(db);
        self
    }

    pub fn on_save(mut self, callback: impl Fn(&mut Window, &mut Context<Self>) + 'static) -> Self {
        self.on_save = Some(Box::new(callback));
        self
    }

    pub fn on_cancel(
        mut self,
        callback: impl Fn(&mut Window, &mut Context<Self>) + 'static,
    ) -> Self {
        self.on_cancel = Some(Box::new(callback));
        self
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.name.trim().is_empty() {
            return Err(ValidationError {
                message: "Task name cannot be empty".to_string(),
            });
        }
        if self.symbol.trim().is_empty() {
            return Err(ValidationError {
                message: "Symbol cannot be empty".to_string(),
            });
        }
        if self.account_id.trim().is_empty() {
            return Err(ValidationError {
                message: "Account must be selected".to_string(),
            });
        }
        if let Err(e) = serde_json::from_str::<TaskConfig>(&self.config_json) {
            return Err(ValidationError {
                message: format!("Invalid configuration JSON: {}", e),
            });
        }
        Ok(())
    }

    pub fn to_task_config(&self) -> TaskConfig {
        let mut config: TaskConfig =
            serde_json::from_str(&self.config_json).unwrap_or_else(|_| TaskConfig {
                id: self.name.clone(),
                symbol: self.symbol.clone(),
                credentials: CredentialsConfig {
                    jwt_token: "".to_string(),
                    signing_key: "".to_string(),
                },
                risk: RiskConfig {
                    level: "conservative".to_string(),
                    max_position_usd: "1000".to_string(),
                    price_jump_threshold_bps: 10,
                },
                sizing: SizingConfig {
                    base_qty: "0.01".to_string(),
                    tiers: 3,
                },
            });

        config.id = self.name.clone();
        config.symbol = self.symbol.clone();

        config
    }

    fn ensure_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.name_input.is_none() {
            let state = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Enter task name")
                    .default_value(self.name.clone())
            });
            self.name_input = Some(state);
        }

        if self.symbol_select.is_none() {
            let selected_index = symbol_index(&self.symbol).or(Some(IndexPath::default()));
            let state =
                cx.new(|cx| SelectState::new(SYMBOL_OPTIONS.to_vec(), selected_index, window, cx));
            if self.symbol.is_empty() {
                self.symbol = SYMBOL_OPTIONS[0].to_string();
            }
            self.symbol_select = Some(state);
        }

        if self.account_select.is_none() {
            let options = self
                .accounts
                .iter()
                .map(|account| AccountOption {
                    id: account.id.clone(),
                    label: account.alias.clone(),
                })
                .collect::<Vec<_>>();
            let selected_index = if self.account_id.is_empty() {
                if let Some(first) = options.first() {
                    self.account_id = first.id.clone();
                    Some(IndexPath::default())
                } else {
                    None
                }
            } else {
                account_index(&options, &self.account_id)
            };
            let state = cx.new(|cx| SelectState::new(options, selected_index, window, cx));
            self.account_select = Some(state);
        }

        if self.config_input.is_none() {
            let state = cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .rows(6)
                    .placeholder("{}")
                    .default_value(self.config_json.clone())
            });
            self.config_input = Some(state);
        }
    }

    fn refresh_values(&mut self, cx: &App) {
        if let Some(state) = &self.name_input {
            self.name = state.read(cx).value().to_string();
        }
        if let Some(state) = &self.config_input {
            self.config_json = state.read(cx).value().to_string();
        }
        if let Some(state) = &self.symbol_select {
            if let Some(value) = state.read(cx).selected_value() {
                self.symbol = value.to_string();
            }
        }
        if let Some(state) = &self.account_select {
            if let Some(value) = state.read(cx).selected_value() {
                self.account_id = value.to_string();
            }
        }
    }

    fn render_input(&self, label: &str, state: &Entity<InputState>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .mb_4()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xa3a3a3))
                    .mb_1()
                    .child(label.to_string()),
            )
            .child(Input::new(state))
    }

    fn render_multiline_input(&self, label: &str, state: &Entity<InputState>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .mb_4()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xa3a3a3))
                    .mb_1()
                    .child(label.to_string()),
            )
            .child(Input::new(state).h(px(160.0)))
    }

    fn render_symbol_select(
        &self,
        state: &Entity<SelectState<Vec<&'static str>>>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .mb_4()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xa3a3a3))
                    .mb_1()
                    .child("Symbol"),
            )
            .child(Select::new(state).placeholder("Select Symbol"))
    }

    fn render_account_select(
        &self,
        state: &Entity<SelectState<Vec<AccountOption>>>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .mb_4()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xa3a3a3))
                    .mb_1()
                    .child("Account"),
            )
            .child(
                Select::new(state)
                    .placeholder("Select Account")
                    .disabled(self.accounts.is_empty()),
            )
    }

    fn handle_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_values(cx);
        if let Err(e) = self.validate() {
            println!("Validation Error: {}", e.message);
            return;
        }

        let task = Task {
            id: self.task_id.clone().unwrap_or_default(),
            account_id: self.account_id.clone(),
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            config: self.to_task_config(),
            status: TaskStatus::Draft,
        };

        if let Some(db) = &self.db {
            let result = match self.mode {
                FormMode::Create => db.create_task(&task),
                FormMode::Edit => db.update_task(&task).map(|_| task.id.clone()),
            };

            match result {
                Ok(_) => {
                    println!("Task saved successfully");
                    if let Some(callback) = &self.on_save {
                        (callback)(window, cx);
                    }
                }
                Err(e) => {
                    println!("Failed to save task: {:?}", e);
                }
            }
        } else {
            println!("Database not connected");
        }
    }

    fn apply_accounts(&mut self, accounts: Vec<Account>) {
        self.accounts = accounts;
        let has_selected = self
            .accounts
            .iter()
            .any(|account| account.id == self.account_id);
        if self.account_id.is_empty() || !has_selected {
            self.account_id = self
                .accounts
                .first()
                .map(|account| account.id.clone())
                .unwrap_or_default();
        }
        self.account_select = None;
    }
}

impl Render for TaskForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = match self.mode {
            FormMode::Create => "Create New Task",
            FormMode::Edit => "Edit Task",
        };

        self.ensure_inputs(window, cx);
        let name_input = self.name_input.as_ref().expect("name_input missing");
        let symbol_select = self.symbol_select.as_ref().expect("symbol_select missing");
        let account_select = self
            .account_select
            .as_ref()
            .expect("account_select missing");
        let config_input = self.config_input.as_ref().expect("config_input missing");

        div()
            .absolute()
            .inset_0()
            .bg(black().opacity(0.8))
            .flex()
            .justify_center()
            .items_center()
            .child(
                div()
                    .w(px(500.0))
                    .bg(rgb(0x2d2d2d))
                    .border_1()
                    .border_color(rgb(0x404040))
                    .rounded_lg()
                    .shadow_lg()
                    .child(
                        div().p_4().border_b_1().border_color(rgb(0x404040)).child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0xffffff))
                                .child(title),
                        ),
                    )
                    .child(
                        div()
                            .p_4()
                            .child(self.render_input("Task Name", name_input))
                            .child(self.render_symbol_select(symbol_select))
                            .child(self.render_account_select(account_select))
                            .child(
                                self.render_multiline_input("Configuration (JSON)", config_input),
                            ),
                    )
                    .child(
                        div()
                            .p_4()
                            .border_t_1()
                            .border_color(rgb(0x404040))
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                div()
                                    .px_4()
                                    .py_2()
                                    .bg(rgb(0x444444))
                                    .rounded_md()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x555555)))
                                    .child("Cancel")
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|view, _, window, cx| {
                                            if let Some(callback) = &view.on_cancel {
                                                (callback)(window, cx);
                                            }
                                        }),
                                    ),
                            )
                            .child(
                                div()
                                    .px_4()
                                    .py_2()
                                    .bg(rgb(0x2563eb))
                                    .rounded_md()
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x1d4ed8)))
                                    .child("Save Task")
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|view, _, window, cx| {
                                            view.handle_save(window, cx);
                                        }),
                                    ),
                            ),
                    ),
            )
    }
}

fn symbol_index(symbol: &str) -> Option<IndexPath> {
    SYMBOL_OPTIONS
        .iter()
        .position(|item| *item == symbol)
        .map(|ix| IndexPath::default().row(ix))
}

fn account_index(options: &[AccountOption], account_id: &str) -> Option<IndexPath> {
    options
        .iter()
        .position(|option| option.id == account_id)
        .map(|ix| IndexPath::default().row(ix))
}
