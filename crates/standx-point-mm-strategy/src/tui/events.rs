/*
[INPUT]:  Crossterm events and internal TUI signals (placeholder)
[OUTPUT]: TUI event routing (placeholder)
[POS]:    TUI event module placeholder
[UPDATE]: 2026-02-09 Add placeholder module for TUI refactor
[UPDATE]: 2026-02-09 Extract key handling match logic from run_tui_with_log
[UPDATE]: 2026-02-09 Add tab switching hotkeys
[UPDATE]: 2026-02-10 Wire modal input handling and submission
*/

use crossterm::event::KeyCode;
use standx_point_adapter::Chain;

use super::app::{ActiveModal, AppState, Tab};
use super::ui::modal::ModalAction;

enum ModalSubmit {
    CreateAccount {
        name: String,
        private_key: String,
        chain: Chain,
    },
    CreateTask {
        id: String,
        symbol: String,
        account_id: String,
        risk_level: String,
        budget_usd: String,
    },
}

/// Handles key events for the TUI.
///
/// Returns `true` if quit is requested, `false` otherwise.
pub(super) async fn handle_key_event(app: &mut AppState, key: KeyCode) -> bool {
    if app.active_modal.is_some() {
        return handle_modal_key_event(app, key).await;
    }

    match key {
        KeyCode::Char('q') => true,
        KeyCode::Char('r') => {
            if let Err(err) = app.refresh_tasks().await {
                app.status_message = format!("refresh tasks failed: {err}");
            }
            false
        }
        KeyCode::Char('s') => {
            if let Err(err) = app.start_selected_task().await {
                app.status_message = format!("start task failed: {err}");
            }
            false
        }
        KeyCode::Char('x') => {
            if let Err(err) = app.stop_selected_task().await {
                app.status_message = format!("stop task failed: {err}");
            }
            false
        }
        KeyCode::Tab | KeyCode::Char('l') => {
            app.next_tab();
            false
        }
        KeyCode::Char('1') => {
            app.set_tab(Tab::Dashboard);
            false
        }
        KeyCode::Char('2') => {
            app.set_tab(Tab::Logs);
            false
        }
        KeyCode::Char('3') => {
            app.set_tab(Tab::Create);
            false
        }
        KeyCode::Char('a') => {
            app.open_create_account();
            false
        }
        KeyCode::Char('t') => {
            if let Err(err) = app.open_create_task().await {
                app.status_message = format!("open create task failed: {err}");
            }
            false
        }
        KeyCode::Up => {
            app.move_selection(-1);
            false
        }
        KeyCode::Down => {
            app.move_selection(1);
            false
        }
        _ => false,
    }
}

async fn handle_modal_key_event(app: &mut AppState, key: KeyCode) -> bool {
    let mut status_update = None;
    let (action, submit) = match app.active_modal_mut() {
        Some(ActiveModal::CreateAccount(modal)) => {
            let action = modal.handle_key(key);
            let submit = if action == ModalAction::Submit {
                Some(ModalSubmit::CreateAccount {
                    name: modal.name().to_string(),
                    private_key: modal.private_key().to_string(),
                    chain: modal.selected_chain(),
                })
            } else {
                None
            };
            (action, submit)
        }
        Some(ActiveModal::CreateTask(modal)) => {
            let action = modal.handle_key(key);
            let mut submit = None;
            if action == ModalAction::Submit {
                if let Some(symbol) = modal.selected_symbol() {
                    if let Some(account_id) = modal.selected_account_id() {
                        submit = Some(ModalSubmit::CreateTask {
                            id: modal.id().to_string(),
                            symbol: symbol.to_string(),
                            account_id: account_id.to_string(),
                            risk_level: modal.selected_risk_level().to_string(),
                            budget_usd: modal.budget_usd().to_string(),
                        });
                    } else {
                        status_update = Some("select an account".to_string());
                    }
                } else {
                    status_update = Some("select a symbol".to_string());
                }
            }
            (action, submit)
        }
        None => return false,
    };

    if let Some(status) = status_update {
        app.status_message = status;
    }

    if action == ModalAction::Cancel {
        app.close_modal();
        return false;
    }

    if let Some(submit) = submit {
        let result = match submit {
            ModalSubmit::CreateAccount {
                name,
                private_key,
                chain,
            } => app.submit_create_account(name, private_key, chain).await,
            ModalSubmit::CreateTask {
                id,
                symbol,
                account_id,
                risk_level,
                budget_usd,
            } => {
                app.submit_create_task(id, symbol, account_id, risk_level, budget_usd)
                    .await
            }
        };

        match result {
            Ok(()) => app.close_modal(),
            Err(err) => app.status_message = format!("modal submit failed: {err}"),
        }
    }

    false
}
