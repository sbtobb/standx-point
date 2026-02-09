/*
[INPUT]:  Account creation form state (placeholder)
[OUTPUT]: Account creation modal rendering (placeholder)
[POS]:    TUI UI modal create account placeholder
[UPDATE]: 2026-02-09 Add placeholder module for TUI refactor
[UPDATE]: 2026-02-10 Define create account modal structure
[UPDATE]: 2026-02-10 Widen CreateAccountModal visibility for re-export
[UPDATE]: 2026-02-10 Add modal input handling and value accessors
[UPDATE]: 2026-02-10 Rename apply_key to handle_key, chain to selected_chain
*/

use crossterm::event::KeyCode;
use standx_point_adapter::Chain;

use super::{handle_modal_key, Field, Modal, ModalAction};

#[allow(dead_code)]
pub(in crate::tui) struct CreateAccountModal {
    name: String,
    private_key: String,
    chain_index: usize,
    focus_index: usize,
}

#[allow(dead_code)]
impl CreateAccountModal {
    pub(in crate::tui) fn new() -> Self {
        Self {
            name: String::new(),
            private_key: String::new(),
            chain_index: 0,
            focus_index: 0,
        }
    }

    pub(in crate::tui) fn to_modal(&self) -> Modal {
        Modal {
            title: "Create Account".to_string(),
            focus_index: self.focus_index,
            fields: vec![
                Field::TextInput {
                    label: "Name".to_string(),
                    value: self.name.clone(),
                },
                Field::TextInput {
                    label: "Private Key".to_string(),
                    value: self.private_key.clone(),
                },
                Field::Select {
                    label: "Chain".to_string(),
                    options: vec!["BSC".to_string(), "Solana".to_string()],
                    selected: self.chain_index,
                },
                Field::Button {
                    label: "Create".to_string(),
                    action: ModalAction::Submit,
                },
                Field::Button {
                    label: "Cancel".to_string(),
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

    pub(in crate::tui) fn name(&self) -> String {
        self.name.clone()
    }

    pub(in crate::tui) fn private_key(&self) -> String {
        self.private_key.clone()
    }

    pub(in crate::tui) fn selected_chain(&self) -> Chain {
        if self.chain_index == 1 {
            Chain::Solana
        } else {
            Chain::Bsc
        }
    }

    fn apply_modal_state(&mut self, modal: &Modal) {
        self.focus_index = modal.focus_index;
        if let Some(Field::TextInput { value, .. }) = modal.fields.get(0) {
            self.name = value.clone();
        }
        if let Some(Field::TextInput { value, .. }) = modal.fields.get(1) {
            self.private_key = value.clone();
        }
        if let Some(Field::Select { selected, .. }) = modal.fields.get(2) {
            self.chain_index = *selected;
        }
    }
}
