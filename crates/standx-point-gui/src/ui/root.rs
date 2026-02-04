/// **Input**: AppState entity, Database handle, UI components (StatusBar, TaskCard, TaskDetailPanel, TaskForm, AccountForm).
/// **Output**: Root layout tree with selection and modal state updates.
/// **Position**: UI root layout and interaction wiring.
/// **Update**: When layout composition, modal/selection flow, or DB wiring changes.
use super::{AccountForm, StatusBar, TaskCard, TaskDetailPanel, TaskForm};
use crate::db::Database;
use crate::state::AppState;
use gpui::*;
use gpui_component::resizable::h_resizable;
use std::path::PathBuf;
use std::sync::Arc;

pub struct RootView {
    state: Entity<AppState>,
    db: Option<Arc<Database>>,
    status_bar: Entity<StatusBar>,
    detail_panel: Entity<TaskDetailPanel>,
    selected_task_id: Option<String>,
    task_form: Option<Entity<TaskForm>>,
    account_form: Option<Entity<AccountForm>>,
}

impl RootView {
    pub fn build(cx: &mut App) -> Self {
        let db = match Database::new(resolve_db_path()) {
            Ok(db) => Some(Arc::new(db)),
            Err(err) => {
                println!("Failed to open database: {:?}", err);
                None
            }
        };
        let state = cx.new(|_| AppState::default());
        let mut accounts = Vec::new();
        let mut tasks = Vec::new();
        if let Some(db) = &db {
            match db.list_accounts() {
                Ok(records) => {
                    accounts = records.into_iter().map(|record| record.account).collect();
                }
                Err(err) => {
                    println!("Failed to load accounts: {:?}", err);
                }
            }

            match db.list_tasks() {
                Ok(records) => {
                    tasks = records;
                }
                Err(err) => {
                    println!("Failed to load tasks: {:?}", err);
                }
            }
        }

        let mut selected_task_id = None;
        let mut detail_panel = TaskDetailPanel::new();
        if let Some(task) = tasks.first().cloned() {
            selected_task_id = Some(task.id.clone());
            detail_panel.set_task(task.clone());
            if let Some(account) = accounts
                .iter()
                .find(|account| account.id == task.account_id)
                .cloned()
            {
                detail_panel.set_account(account);
            }
        }

        if !accounts.is_empty() || !tasks.is_empty() {
            let state_accounts = accounts.clone();
            let state_tasks = tasks.clone();
            let _ = state.update(cx, |state, _| {
                state.accounts = state_accounts;
                state.tasks = state_tasks;
            });
        }

        let status_bar = cx.new(|_| StatusBar::new());
        let detail_panel = cx.new(|_| detail_panel);
        Self {
            state,
            db,
            status_bar,
            detail_panel,
            selected_task_id,
            task_form: None,
            account_form: None,
        }
    }
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(_cx);
        let weak_root = _cx.weak_entity();
        let weak_root_for_task = weak_root.clone();
        let weak_root_for_account = weak_root.clone();

        let sidebar = div()
            .flex()
            .flex_col()
            .w(px(280.0))
            .h_full()
            .bg(rgb(0x252526))
            .child(
                div()
                    .p_4()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xcccccc))
                    .child("Tasks"),
            )
            .child(div().flex().flex_col().gap_2().p_2().flex_1().children(
                state.tasks.iter().map(|task| {
                    let is_selected = self.selected_task_id.as_ref() == Some(&task.id);
                    let task = task.clone();
                    let account = state
                        .accounts
                        .iter()
                        .find(|account| account.id == task.account_id)
                        .cloned();
                    let task_id = task.id.clone();

                    div()
                        .child(TaskCard::new(task.clone(), is_selected))
                        .on_mouse_down(
                            MouseButton::Left,
                            _cx.listener(move |this, _, _, cx| {
                                this.selected_task_id = Some(task_id.clone());

                                let mut panel = TaskDetailPanel::new();
                                panel.set_task(task.clone());
                                if let Some(account) = account.clone() {
                                    panel.set_account(account);
                                }
                                let _ = this.detail_panel.write(cx, panel);

                                cx.notify();
                            }),
                        )
                }),
            ));

        let sidebar = sidebar.child(
            div()
                .p_2()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .w_full()
                        .px_3()
                        .py_2()
                        .bg(rgb(0x333333))
                        .rounded_md()
                        .text_sm()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0xe5e5e5))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0x3a3a3a)))
                        .child("New Task")
                        .on_mouse_down(
                            MouseButton::Left,
                            _cx.listener(move |this, _, _, cx| {
                                let accounts = this.state.read(cx).accounts.clone();
                                let db = this.db.clone();
                                let form = cx.new(|_| {
                                    let weak_root = weak_root_for_task.clone();
                                    let mut form = TaskForm::new_create().with_accounts(accounts);
                                    if let Some(db) = db {
                                        form = form.with_db(db);
                                    }
                                    form.on_save({
                                        let weak_root = weak_root.clone();
                                        move |_, cx| {
                                            if let Some(root) = weak_root.upgrade() {
                                                let _ = root.update(cx, |root, cx| {
                                                    root.task_form = None;
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    })
                                    .on_cancel({
                                        let weak_root = weak_root.clone();
                                        move |_, cx| {
                                            if let Some(root) = weak_root.upgrade() {
                                                let _ = root.update(cx, |root, cx| {
                                                    root.task_form = None;
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    })
                                });

                                this.task_form = Some(form);
                                cx.notify();
                            }),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .px_3()
                        .py_2()
                        .bg(rgb(0x333333))
                        .rounded_md()
                        .text_sm()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0xe5e5e5))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0x3a3a3a)))
                        .child("New Account")
                        .on_mouse_down(
                            MouseButton::Left,
                            _cx.listener(move |this, _, _, cx| {
                                let state = this.state.clone();
                                let db = this.db.clone();
                                let form = cx.new(|_| {
                                    let weak_root = weak_root_for_account.clone();
                                    let mut form = AccountForm::new(state);
                                    if let Some(db) = db {
                                        form = form.with_db(db);
                                    }
                                    form.on_save({
                                        let weak_root = weak_root.clone();
                                        move |_, cx| {
                                            if let Some(root) = weak_root.upgrade() {
                                                let _ = root.update(cx, |root, cx| {
                                                    root.account_form = None;
                                                    if let Some(form) = root.task_form.clone() {
                                                        let accounts =
                                                            root.state.read(cx).accounts.clone();
                                                        let _ = form.update(cx, |form, _| {
                                                            form.set_accounts(accounts);
                                                        });
                                                    }
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    })
                                    .on_cancel({
                                        let weak_root = weak_root.clone();
                                        move |_, cx| {
                                            if let Some(root) = weak_root.upgrade() {
                                                let _ = root.update(cx, |root, cx| {
                                                    root.account_form = None;
                                                    cx.notify();
                                                });
                                            }
                                        }
                                    })
                                });

                                this.account_form = Some(form);
                                cx.notify();
                            }),
                        ),
                ),
        );

        let mut root = div()
            .size_full()
            .font_family(".SystemUI")
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xe5e5e5))
            .flex()
            .flex_col()
            .child(self.status_bar.clone())
            .child(
                div().flex_1().child(
                    h_resizable("main_split")
                        .child(sidebar.into_any_element())
                        .child(
                            div()
                                .flex()
                                .size_full()
                                .bg(rgb(0x1e1e1e))
                                .child(self.detail_panel.clone())
                                .into_any_element(),
                        ),
                ),
            );

        if let Some(form) = self.task_form.clone() {
            root = root.child(form);
        }

        if let Some(form) = self.account_form.clone() {
            root = root.child(form);
        }

        root
    }
}

fn resolve_db_path() -> PathBuf {
    if let Ok(path) = std::env::var("STANDX_GUI_DB_PATH") {
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }
    PathBuf::from("standx-point-gui.db")
}
