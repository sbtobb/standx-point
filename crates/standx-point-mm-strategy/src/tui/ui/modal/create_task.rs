/*
[INPUT]:  Task creation form state (placeholder)
[OUTPUT]: Task creation modal rendering (placeholder)
[POS]:    TUI UI modal create task placeholder
[UPDATE]: 2026-02-09 Add placeholder module for TUI refactor
[UPDATE]: 2026-02-10 Implement create task modal structure
[UPDATE]: 2026-02-10 Widen CreateTaskModal visibility for crate::tui re-export
[UPDATE]: 2026-02-10 Add modal input handling and option storage
[UPDATE]: 2026-02-10 Rename apply_key to handle_key for consistency
*/

use crossterm::event::KeyCode;

use super::{Field, Modal, ModalAction, handle_modal_key};

#[allow(dead_code)]
pub(in crate::tui) struct CreateTaskModal {
    id: String,
    symbol_index: usize,
    account_index: usize,
    risk_index: usize,
    budget_usd: String,
    focus_index: usize,
    symbols: Vec<String>,
    account_ids: Vec<String>,
    account_labels: Vec<String>,
}

#[allow(dead_code)]
impl CreateTaskModal {
    pub(in crate::tui) fn new(
        id: String,
        symbols: Vec<String>,
        accounts: Vec<(String, String)>,
    ) -> Self {
        let (account_ids, account_labels): (Vec<String>, Vec<String>) =
            accounts.into_iter().unzip();
        Self {
            id,
            symbol_index: 0,
            account_index: 0,
            risk_index: 0,
            budget_usd: String::from("50000"),
            focus_index: 0,
            symbols,
            account_ids,
            account_labels,
        }
    }

    pub(in crate::tui) fn to_modal(&self) -> Modal {
        let risk_options = Self::risk_options();

        Modal {
            title: String::from("Create Task"),
            focus_index: self.focus_index,
            fields: vec![
                Field::TextInput {
                    label: String::from("ID"),
                    value: self.id.clone(),
                },
                Field::Select {
                    label: String::from("Symbol"),
                    options: self.symbols.clone(),
                    selected: self.symbol_index,
                },
                Field::Select {
                    label: String::from("Account"),
                    options: self.account_labels.clone(),
                    selected: self.account_index,
                },
                Field::Select {
                    label: String::from("Risk"),
                    options: risk_options,
                    selected: self.risk_index,
                },
                Field::TextInput {
                    label: String::from("Budget USD"),
                    value: self.budget_usd.clone(),
                },
                Field::Button {
                    label: String::from("Create"),
                    action: ModalAction::Submit,
                },
                Field::Button {
                    label: String::from("Cancel"),
                    action: ModalAction::Cancel,
                },
            ],
        }
    }

    pub(in crate::tui) fn handle_key(&mut self, key: KeyCode) -> ModalAction {
        let mut modal = self.to_modal();
        let action = handle_modal_key(&mut modal, key);
        self.apply_modal_state(&modal);
        action
    }

    pub(in crate::tui) fn id(&self) -> &str {
        self.id.as_str()
    }

    pub(in crate::tui) fn budget_usd(&self) -> &str {
        self.budget_usd.as_str()
    }

    pub(in crate::tui) fn selected_symbol(&self) -> Option<&str> {
        self.symbols.get(self.symbol_index).map(String::as_str)
    }

    pub(in crate::tui) fn selected_account_id(&self) -> Option<&str> {
        self.account_ids.get(self.account_index).map(String::as_str)
    }

    pub(in crate::tui) fn selected_risk_level(&self) -> &str {
        match self.risk_index {
            1 => "medium",
            2 => "high",
            3 => "xhigh",
            _ => "low",
        }
    }

    fn apply_modal_state(&mut self, modal: &Modal) {
        self.focus_index = modal.focus_index;
        if let Some(Field::TextInput { value, .. }) = modal.fields.get(0) {
            self.id = value.clone();
        }
        if let Some(Field::Select { selected, .. }) = modal.fields.get(1) {
            self.symbol_index = *selected;
        }
        if let Some(Field::Select { selected, .. }) = modal.fields.get(2) {
            self.account_index = *selected;
        }
        if let Some(Field::Select { selected, .. }) = modal.fields.get(3) {
            self.risk_index = *selected;
        }
        if let Some(Field::TextInput { value, .. }) = modal.fields.get(4) {
            self.budget_usd = value.clone();
        }
    }

    fn risk_options() -> Vec<String> {
        vec![
            String::from("low"),
            String::from("medium"),
            String::from("high"),
            String::from("xhigh"),
        ]
    }
}
