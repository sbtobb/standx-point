/*
[INPUT]:  Modal state, fields, and key events
[OUTPUT]: Modal rendering output and modal action results
[POS]:    TUI UI modal module root
[UPDATE]: 2026-02-09 Add modal module tree for refactor
[UPDATE]: 2026-02-10 Implement modal framework rendering and input handling
[UPDATE]: 2026-02-10 Expose CreateAccountModal and CreateTaskModal for AppState usage
[UPDATE]: 2026-02-10 Fix modal exports to avoid duplicate structs
[UPDATE]: 2026-02-10 Expand modal visibility for tui modules
[UPDATE]: 2026-02-10 Add text input editing for modal fields
*/

mod create_account;
mod create_task;

pub(in crate::tui) use create_account::CreateAccountModal;
pub(in crate::tui) use create_task::CreateTaskModal;

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

#[allow(dead_code)]
pub(in crate::tui) struct Modal {
    pub(super) title: String,
    pub(super) focus_index: usize,
    pub(super) fields: Vec<Field>,
}

#[allow(dead_code)]
pub(in crate::tui) enum Field {
    TextInput {
        label: String,
        value: String,
    },
    Select {
        label: String,
        options: Vec<String>,
        selected: usize,
    },
    Button {
        label: String,
        action: ModalAction,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(in crate::tui) enum ModalAction {
    Submit,
    Cancel,
    None,
}

#[allow(dead_code)]
pub(in crate::tui) fn draw_modal(frame: &mut ratatui::Frame, area: Rect, modal: &Modal) {
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(modal.title.as_str());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = modal
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let content = match field {
                Field::TextInput { label, value } => format!("{label}: {value}"),
                Field::Select {
                    label,
                    options,
                    selected,
                } => {
                    let selected_value = options.get(*selected).map(String::as_str).unwrap_or("-");
                    format!("{label}: {selected_value}")
                }
                Field::Button { label, .. } => format!("[{label}]"),
            };
            let style = if index == modal.focus_index {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            Line::from(Span::styled(content, style))
        })
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

#[allow(dead_code)]
pub(in crate::tui) fn handle_modal_key(modal: &mut Modal, key: KeyCode) -> ModalAction {
    match key {
        KeyCode::Esc => ModalAction::Cancel,
        KeyCode::Tab => {
            if !modal.fields.is_empty() {
                modal.focus_index = (modal.focus_index + 1) % modal.fields.len();
            }
            ModalAction::None
        }
        KeyCode::Up => {
            if let Some(Field::Select {
                selected, options, ..
            }) = modal.fields.get_mut(modal.focus_index)
            {
                if !options.is_empty() {
                    *selected = selected.saturating_sub(1);
                }
            }
            ModalAction::None
        }
        KeyCode::Down => {
            if let Some(Field::Select {
                selected, options, ..
            }) = modal.fields.get_mut(modal.focus_index)
            {
                if *selected + 1 < options.len() {
                    *selected += 1;
                }
            }
            ModalAction::None
        }
        KeyCode::Backspace => {
            if let Some(Field::TextInput { value, .. }) = modal.fields.get_mut(modal.focus_index) {
                value.pop();
            }
            ModalAction::None
        }
        KeyCode::Char(ch) => {
            if let Some(Field::TextInput { value, .. }) = modal.fields.get_mut(modal.focus_index) {
                value.push(ch);
            }
            ModalAction::None
        }
        KeyCode::Enter => {
            if let Some(Field::Button { action, .. }) = modal.fields.get(modal.focus_index) {
                return *action;
            }
            ModalAction::None
        }
        _ => ModalAction::None,
    }
}
