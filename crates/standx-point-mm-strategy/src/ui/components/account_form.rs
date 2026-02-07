use ratatui::Frame;
/// **Input**: `Account` domain model and ratatui layout/style primitives.
/// **Output**: `AccountForm` state and `render` function for the account dialog.
/// **Position**: TUI component for account create/edit dialog rendering.
/// **Update**: Revisit when form fields or validation rules change.
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use std::fmt;

use crate::state::storage::Account;
use standx_point_adapter::Chain;

use super::help_text;
use super::single_select::SingleSelect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountChain {
    Bsc,
    Solana,
}

impl fmt::Display for AccountChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountChain::Bsc => write!(f, "bsc"),
            AccountChain::Solana => write!(f, "solana"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountField {
    Chain,
    PrivateKey,
    Name,
}

impl AccountField {
    pub fn next(self) -> Self {
        match self {
            AccountField::Chain => AccountField::PrivateKey,
            AccountField::PrivateKey => AccountField::Name,
            AccountField::Name => AccountField::Chain,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            AccountField::Chain => AccountField::Name,
            AccountField::PrivateKey => AccountField::Chain,
            AccountField::Name => AccountField::PrivateKey,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccountSeed {
    pub name: String,
    pub chain: AccountChain,
    pub private_key: String,
}

/// Account form data for creating/editing accounts
#[derive(Debug, Clone)]
pub struct AccountForm {
    pub account_id: Option<String>,
    pub name: String,
    pub private_key: String,
    pub chain: AccountChain,
    pub error_message: Option<String>,
    pub focused_field: AccountField,
    pub chain_select: SingleSelect<AccountChain>,
}

impl AccountForm {
    pub fn new() -> Self {
        let chain_options = vec![AccountChain::Bsc, AccountChain::Solana];
        let mut chain_select = SingleSelect::new("Chains", chain_options);
        chain_select.set_selected_index(0);
        Self {
            account_id: None,
            name: String::new(),
            private_key: String::new(),
            chain: AccountChain::Bsc,
            error_message: None,
            focused_field: AccountField::Chain,
            chain_select,
        }
    }

    pub fn from_account(account: &Account) -> Self {
        let chain_options = vec![AccountChain::Bsc, AccountChain::Solana];
        let mut chain_select = SingleSelect::new("Chains", chain_options);
        let chain = match account.chain {
            Some(Chain::Solana) => {
                chain_select.set_selected_index(1);
                AccountChain::Solana
            }
            _ => {
                chain_select.set_selected_index(0);
                AccountChain::Bsc
            }
        };
        Self {
            account_id: Some(account.id.clone()),
            name: account.name.clone(),
            private_key: String::new(),
            chain,
            error_message: None,
            focused_field: AccountField::Name,
            chain_select,
        }
    }

    pub fn validate_name(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Account name is required".to_string());
        }
        Ok(())
    }

    pub fn to_account_seed(&self) -> Result<AccountSeed, String> {
        if self.name.trim().is_empty() {
            return Err("Account name is required".to_string());
        }
        if self.private_key.trim().is_empty() {
            return Err("Private key is required".to_string());
        }

        Ok(AccountSeed {
            name: self.name.trim().to_string(),
            chain: self.chain,
            private_key: self.private_key.trim().to_string(),
        })
    }
}

/// Render the account form dialog
pub fn render(frame: &mut Frame, area: Rect, form: &AccountForm, is_edit: bool) {
    // Create centered popup
    let popup_area = centered_rect(80, 70, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let title = if is_edit {
        let id = form
            .account_id
            .as_ref()
            .map(|value| value.as_str())
            .unwrap_or("unknown");
        format!(" Edit Account: {} ", id)
    } else {
        " Create New Account ".to_string()
    };

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(popup_area);
    frame.render_widget(block, popup_area);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(3)])
        .split(inner_area);
    let content_area = sections[0];
    let help_area = sections[1];
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(content_area);
    let fields_area = columns[0];
    let select_area = columns[1];

    // Create the form content
    let mut content = vec![Line::from("")];

    if let Some(id) = form.account_id.as_ref() {
        content.push(Line::from(vec![
            Span::styled("ID:        ", Style::default().fg(Color::Cyan)),
            Span::raw(id),
        ]));
        content.push(Line::from(""));
    }

    let chain_style = if form.focused_field == AccountField::Chain {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    content.push(Line::from(vec![
        Span::styled("Chain:     ", chain_style),
        Span::raw(form.chain.to_string()),
        if form.focused_field == AccountField::Chain {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    if !is_edit {
        let key_style = if form.focused_field == AccountField::PrivateKey {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };
        content.push(Line::from(vec![
            Span::styled("Private Key: ", key_style),
            Span::raw(&form.private_key),
            if form.focused_field == AccountField::PrivateKey {
                Span::styled(" █", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            },
        ]));
        content.push(Line::from(""));
    }

    let name_style = if form.focused_field == AccountField::Name {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    content.push(Line::from(vec![
        Span::styled("Name:        ", name_style),
        Span::raw(&form.name),
        if form.focused_field == AccountField::Name {
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
        Span::styled("Tab ", Style::default().fg(Color::Cyan)),
        Span::styled("switch fields  ", Style::default().fg(Color::Cyan)),
        Span::styled("↑↓/j/k ", Style::default().fg(Color::Cyan)),
        Span::styled("select option  ", Style::default().fg(Color::Cyan)),
        Span::styled("Enter ", Style::default().fg(Color::Cyan)),
        Span::styled("select/save  ", Style::default().fg(Color::Cyan)),
        Span::styled("Esc ", Style::default().fg(Color::Cyan)),
        Span::styled("cancel", Style::default().fg(Color::Cyan)),
    ]));

    let paragraph = Paragraph::new(content)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, fields_area);

    if form.focused_field == AccountField::Chain {
        form.chain_select.render(frame, select_area);
    }

    let help_text = match form.focused_field {
        AccountField::Chain => "选择链类型：bsc/solana",
        AccountField::PrivateKey => "输入钱包私钥（不使用缩写）",
        AccountField::Name => "输入账户名称，用于显示",
    };
    help_text::render(frame, help_area, help_text);
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
        assert!(form.account_id.is_none());
        assert!(form.name.is_empty());
        assert!(form.private_key.is_empty());
        assert_eq!(form.chain, AccountChain::Bsc);
        assert!(form.error_message.is_none());
        assert_eq!(form.focused_field, AccountField::Chain);
    }

    #[test]
    fn test_account_form_validation_empty_name() {
        let mut form = AccountForm::new();
        form.private_key = "key".to_string();
        let result = form.to_account_seed();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Account name is required"));
    }
}
