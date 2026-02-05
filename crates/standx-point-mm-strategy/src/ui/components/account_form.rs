/// **Input**: `Account` domain model and ratatui layout/style primitives.
/// **Output**: `AccountForm` state and `render` function for the account dialog.
/// **Position**: TUI component for account create/edit dialog rendering.
/// **Update**: Revisit when form fields or validation rules change.
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::state::storage::Account;

/// Account form data for creating/editing accounts
#[derive(Debug, Clone, PartialEq)]
pub struct AccountForm {
    pub id: String,
    pub name: String,
    pub jwt_token: String,
    pub signing_key: String,
    pub error_message: Option<String>,
    pub focused_field: usize,        // 0=id, 1=name, 2=jwt, 3=signing_key
    pub replace_on_next_input: bool, // Whether next character input replaces entire field (select-all semantics)
}

impl AccountForm {
    pub fn new() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            jwt_token: String::new(),
            signing_key: String::new(),
            error_message: None,
            focused_field: 0,
            replace_on_next_input: false,
        }
    }

    pub fn from_account(account: &Account) -> Self {
        Self {
            id: account.id.clone(),
            name: account.name.clone(),
            jwt_token: account.jwt_token.clone(),
            signing_key: account.signing_key.clone(),
            error_message: None,
            focused_field: 0,
            replace_on_next_input: false,
        }
    }

    pub fn to_account(&self) -> Result<Account, String> {
        if self.id.is_empty() {
            return Err("Account ID is required".to_string());
        }
        if self.name.is_empty() {
            return Err("Account name is required".to_string());
        }
        if self.jwt_token.is_empty() {
            return Err("JWT token is required".to_string());
        }
        if self.signing_key.is_empty() {
            return Err("Signing key is required".to_string());
        }

        let account = Account::new(
            self.id.clone(),
            self.name.clone(),
            self.jwt_token.clone(),
            self.signing_key.clone(),
        );

        account.validate().map_err(|e| e.to_string())?;
        Ok(account)
    }
}

/// Render the account form dialog
pub fn render(frame: &mut Frame, area: Rect, form: &AccountForm, is_edit: bool) {
    // Create centered popup
    let popup_area = centered_rect(80, 70, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let title = if is_edit {
        format!(" Edit Account: {} ", form.id)
    } else {
        " Create New Account ".to_string()
    };

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    // Create the form content
    let mut content = vec![Line::from("")];

    // ID field
    let id_style = if form.focused_field == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    content.push(Line::from(vec![
        Span::styled("ID:        ", id_style),
        Span::raw(&form.id),
        if form.focused_field == 0 {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Name field
    let name_style = if form.focused_field == 1 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    content.push(Line::from(vec![
        Span::styled("Name:      ", name_style),
        Span::raw(&form.name),
        if form.focused_field == 1 {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // JWT Token field (masked for security)
    let jwt_style = if form.focused_field == 2 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let jwt_display = if form.jwt_token.is_empty() {
        String::new()
    } else {
        format!("{}...", &form.jwt_token[..form.jwt_token.len().min(20)])
    };
    content.push(Line::from(vec![
        Span::styled("JWT Token: ", jwt_style),
        Span::raw(&jwt_display),
        if form.focused_field == 2 {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Signing Key field (masked for security)
    let key_style = if form.focused_field == 3 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let key_display = if form.signing_key.is_empty() {
        String::new()
    } else {
        format!("{}...", &form.signing_key[..form.signing_key.len().min(20)])
    };
    content.push(Line::from(vec![
        Span::styled("Sign Key:  ", key_style),
        Span::raw(&key_display),
        if form.focused_field == 3 {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Error message if any
    if let Some(ref error) = form.error_message {
        content.push(Line::from(vec![
            Span::styled(
                "Error: ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(error, Style::default().fg(Color::Red)),
        ]));
        content.push(Line::from(""));
    }

    // Instructions
    content.push(Line::from(vec![
        Span::styled("Tab/↑↓ ", Style::default().fg(Color::Cyan)),
        Span::styled("switch fields  ", Style::default().fg(Color::Gray)),
        Span::styled("Enter ", Style::default().fg(Color::Cyan)),
        Span::styled("save  ", Style::default().fg(Color::Gray)),
        Span::styled("Esc ", Style::default().fg(Color::Cyan)),
        Span::styled("cancel", Style::default().fg(Color::Gray)),
    ]));

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, popup_area);
}

/// Create a centered rect for popups
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_form_new() {
        let form = AccountForm::new();
        assert!(form.id.is_empty());
        assert!(form.name.is_empty());
        assert!(form.jwt_token.is_empty());
        assert!(form.signing_key.is_empty());
        assert!(form.error_message.is_none());
        assert_eq!(form.focused_field, 0);
    }

    #[test]
    fn test_account_form_validation_empty_id() {
        let form = AccountForm {
            id: "".to_string(),
            name: "Test".to_string(),
            jwt_token: "token".to_string(),
            signing_key: "key".to_string(),
            error_message: None,
            focused_field: 0,
            replace_on_next_input: false,
        };

        let result = form.to_account();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ID is required"));
    }
}
