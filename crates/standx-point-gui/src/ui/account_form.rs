/// **Input**: AppState entity, Database handle, chain selection, private key string, optional alias.
/// **Output**: Persisted account record plus AppState accounts updated and form callbacks invoked.
/// **Position**: UI account creation form.
/// **Update**: When account fields, wallet derivation logic, or persistence flow changes.
use crate::db::Database;
use crate::state::{Account, AppState};
use gpui::*;
use gpui_component::input::{Input, InputState};
use gpui_component::select::{Select, SelectState};
use gpui_component::IndexPath;
use standx_point_adapter::auth::{EvmWalletSigner, SolanaWalletSigner};
use standx_point_adapter::types::Chain;
use standx_point_adapter::WalletSigner;
use std::sync::Arc;

const CHAIN_OPTIONS: [&str; 2] = ["BSC", "Solana"];

#[derive(Debug, Clone)]
pub struct AccountValidationError {
    pub message: String,
}

pub struct AccountForm {
    state: Entity<AppState>,
    db: Option<Arc<Database>>,
    alias: String,
    private_key: String,
    chain: Chain,
    alias_input: Option<Entity<InputState>>,
    key_input: Option<Entity<InputState>>,
    chain_select: Option<Entity<SelectState<Vec<&'static str>>>>,
    on_save: Option<Box<dyn Fn(&mut Window, &mut Context<Self>) + 'static>>,
    on_cancel: Option<Box<dyn Fn(&mut Window, &mut Context<Self>) + 'static>>,
}

impl AccountForm {
    pub fn new(state: Entity<AppState>) -> Self {
        Self {
            state,
            db: None,
            alias: String::new(),
            private_key: String::new(),
            chain: Chain::Bsc,
            alias_input: None,
            key_input: None,
            chain_select: None,
            on_save: None,
            on_cancel: None,
        }
    }

    pub fn on_save(mut self, callback: impl Fn(&mut Window, &mut Context<Self>) + 'static) -> Self {
        self.on_save = Some(Box::new(callback));
        self
    }

    pub fn with_db(mut self, db: Arc<Database>) -> Self {
        self.db = Some(db);
        self
    }

    pub fn on_cancel(
        mut self,
        callback: impl Fn(&mut Window, &mut Context<Self>) + 'static,
    ) -> Self {
        self.on_cancel = Some(Box::new(callback));
        self
    }

    fn ensure_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.alias_input.is_none() {
            let state = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Alias (optional)")
                    .default_value(self.alias.clone())
            });
            self.alias_input = Some(state);
        }

        if self.key_input.is_none() {
            let state = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Private key")
                    .default_value(self.private_key.clone())
            });
            self.key_input = Some(state);
        }

        if self.chain_select.is_none() {
            let selected_index = Some(IndexPath::default().row(chain_index(self.chain)));
            let state =
                cx.new(|cx| SelectState::new(CHAIN_OPTIONS.to_vec(), selected_index, window, cx));
            self.chain_select = Some(state);
        }
    }

    fn refresh_values(&mut self, cx: &App) {
        if let Some(state) = &self.alias_input {
            self.alias = state.read(cx).value().to_string();
        }
        if let Some(state) = &self.key_input {
            self.private_key = state.read(cx).value().to_string();
        }
        if let Some(state) = &self.chain_select {
            if let Some(value) = state.read(cx).selected_value() {
                self.chain = chain_from_value(value);
            }
        }
    }

    fn validate(&self) -> Result<(), AccountValidationError> {
        if self.private_key.trim().is_empty() {
            return Err(AccountValidationError {
                message: "Private key cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    fn handle_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_values(cx);
        if let Err(e) = self.validate() {
            println!("Validation Error: {}", e.message);
            return;
        }

        let private_key = self.private_key.trim();
        let address_result = match self.chain {
            Chain::Bsc => {
                EvmWalletSigner::new(private_key).map(|signer| signer.address().to_string())
            }
            Chain::Solana => {
                SolanaWalletSigner::new(private_key).map(|signer| signer.address().to_string())
            }
        };

        let address = match address_result {
            Ok(address) => address,
            Err(err) => {
                println!("Invalid private key: {}", err);
                return;
            }
        };

        let alias = if self.alias.trim().is_empty() {
            format!("acct-{}", address_prefix(&address))
        } else {
            self.alias.trim().to_string()
        };

        let chain = self.chain;
        let account = Account {
            id: String::new(),
            address: address.clone(),
            alias,
            chain,
        };

        let account_id = match &self.db {
            Some(db) => match db.create_account(&account, None) {
                Ok(id) => id,
                Err(err) => {
                    println!("Failed to save account: {:?}", err);
                    return;
                }
            },
            None => {
                println!("Database not connected");
                return;
            }
        };

        let account = Account {
            id: account_id.clone(),
            ..account
        };

        self.state.update(cx, |state, cx| {
            state.accounts.push(account);
            cx.notify();
        });

        println!("Account created: {}", account_id);
        if let Some(callback) = &self.on_save {
            (callback)(window, cx);
        }
    }
}

impl Render for AccountForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_inputs(window, cx);
        let alias_input = self.alias_input.as_ref().expect("alias_input missing");
        let key_input = self.key_input.as_ref().expect("key_input missing");
        let chain_select = self.chain_select.as_ref().expect("chain_select missing");

        div()
            .absolute()
            .inset_0()
            .bg(black().opacity(0.8))
            .flex()
            .justify_center()
            .items_center()
            .child(
                div()
                    .w(px(460.0))
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
                                .child("Create Account"),
                        ),
                    )
                    .child(
                        div()
                            .p_4()
                            .child(render_input("Alias", alias_input))
                            .child(render_chain_select(chain_select))
                            .child(render_input("Private Key", key_input)),
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
                                    .child("Save Account")
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

fn render_input(label: &str, state: &Entity<InputState>) -> impl IntoElement {
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

fn render_chain_select(state: &Entity<SelectState<Vec<&'static str>>>) -> impl IntoElement {
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
                .child("Chain"),
        )
        .child(Select::new(state).placeholder("Select Chain"))
}

fn chain_index(chain: Chain) -> usize {
    match chain {
        Chain::Solana => 1,
        Chain::Bsc => 0,
    }
}

fn chain_from_value(value: &str) -> Chain {
    if value.eq_ignore_ascii_case("solana") {
        Chain::Solana
    } else {
        Chain::Bsc
    }
}

fn address_prefix(address: &str) -> String {
    let trimmed = address.trim_start_matches("0x");
    trimmed.chars().take(6).collect::<String>()
}
