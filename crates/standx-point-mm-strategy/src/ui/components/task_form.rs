/// **Input**: `Task` domain model and ratatui layout/style primitives.
/// **Output**: `TaskForm` state and `render` function for the task dialog.
/// **Position**: TUI component for task create/edit dialog rendering.
/// **Update**: Revisit when form fields or validation rules change.
/// **Update**: Show selected account display label in the form.
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use uuid::Uuid;

use super::help_text;
use super::single_select::SingleSelect;

use crate::state::storage::Task;

const DEFAULT_BASE_QTY: &str = "0.1";
const DEFAULT_TIERS: u8 = 2;
const DEFAULT_MAX_POSITION_USD: &str = "50000";
const DEFAULT_PRICE_JUMP_THRESHOLD_BPS: &str = "5";
const SYMBOL_OPTIONS: [&str; 4] = ["BTC-USD", "ETH-USD", "XAG-USD", "XAU-USD"];
const RISK_LEVEL_OPTIONS: [&str; 3] = ["conservative", "moderate", "aggressive"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskField {
    Symbol,
    AccountId,
    RiskLevel,
    MaxPositionUsd,
    PriceJumpThresholdBps,
}

impl TaskField {
    pub fn next(self) -> Self {
        match self {
            TaskField::Symbol => TaskField::AccountId,
            TaskField::AccountId => TaskField::RiskLevel,
            TaskField::RiskLevel => TaskField::MaxPositionUsd,
            TaskField::MaxPositionUsd => TaskField::PriceJumpThresholdBps,
            TaskField::PriceJumpThresholdBps => TaskField::Symbol,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            TaskField::Symbol => TaskField::PriceJumpThresholdBps,
            TaskField::AccountId => TaskField::Symbol,
            TaskField::RiskLevel => TaskField::AccountId,
            TaskField::MaxPositionUsd => TaskField::RiskLevel,
            TaskField::PriceJumpThresholdBps => TaskField::MaxPositionUsd,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountOption {
    Existing { id: String, name: String },
    CreateNew,
}

impl std::fmt::Display for AccountOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountOption::Existing { id, name } => write!(f, "{name} ({id})"),
            AccountOption::CreateNew => write!(f, "+ Create new account"),
        }
    }
}

/// Task form data for creating/editing tasks
#[derive(Debug, Clone)]
pub struct TaskForm {
    pub id: String,
    pub symbol: String,
    pub account_id: String,
    pub risk_level: String,
    pub max_position_usd: String,
    pub price_jump_threshold_bps: String,
    pub base_qty: String,
    pub tiers: u8,
    pub error_message: Option<String>,
    pub focused_field: TaskField,
    pub symbol_select: SingleSelect<String>,
    pub risk_level_select: SingleSelect<String>,
    pub account_select: SingleSelect<AccountOption>,
}

impl TaskForm {
    pub fn new() -> Self {
        let symbol_options = symbol_options();
        let risk_level_options = risk_level_options();
        let mut symbol_select = SingleSelect::new("Symbols", symbol_options.clone());
        let mut risk_level_select = SingleSelect::new("Risk Levels", risk_level_options.clone());
        let account_select = SingleSelect::new("Accounts", Vec::new());
        if !symbol_options.is_empty() {
            symbol_select.set_selected_index(0);
        }
        if !risk_level_options.is_empty() {
            risk_level_select.set_selected_index(0);
        }
        let symbol = symbol_options.first().cloned().unwrap_or_default();
        let risk_level = risk_level_options.first().cloned().unwrap_or_default();
        Self {
            id: Uuid::new_v4().to_string(),
            symbol,
            account_id: String::new(),
            risk_level,
            max_position_usd: DEFAULT_MAX_POSITION_USD.to_string(),
            price_jump_threshold_bps: DEFAULT_PRICE_JUMP_THRESHOLD_BPS.to_string(),
            base_qty: DEFAULT_BASE_QTY.to_string(),
            tiers: DEFAULT_TIERS,
            error_message: None,
            focused_field: TaskField::Symbol,
            symbol_select,
            risk_level_select,
            account_select,
        }
    }

    pub fn from_task(task: &Task) -> Self {
        let mut symbol_options = symbol_options();
        if !symbol_options.iter().any(|option| option == &task.symbol) {
            symbol_options.push(task.symbol.clone());
        }
        let mut risk_level_options = risk_level_options();
        if !risk_level_options
            .iter()
            .any(|option| option == &task.risk_level)
        {
            risk_level_options.push(task.risk_level.clone());
        }
        let mut symbol_select = SingleSelect::new("Symbols", symbol_options);
        symbol_select.set_selected_value(&task.symbol);
        let mut risk_level_select = SingleSelect::new("Risk Levels", risk_level_options);
        risk_level_select.set_selected_value(&task.risk_level);
        let account_select = SingleSelect::new("Accounts", Vec::new());
        Self {
            id: task.id.clone(),
            symbol: task.symbol.clone(),
            account_id: task.account_id.clone(),
            risk_level: task.risk_level.clone(),
            max_position_usd: task.max_position_usd.clone(),
            price_jump_threshold_bps: task.price_jump_threshold_bps.to_string(),
            base_qty: task.base_qty.clone(),
            tiers: task.tiers,
            error_message: None,
            focused_field: TaskField::Symbol,
            symbol_select,
            risk_level_select,
            account_select,
        }
    }

    pub fn to_task(&self) -> Result<Task, String> {
        if self.id.is_empty() {
            return Err("Task ID is required".to_string());
        }
        if self.symbol.is_empty() {
            return Err("Symbol is required".to_string());
        }
        if self.account_id.is_empty() {
            return Err("Account ID is required".to_string());
        }
        if self.risk_level.is_empty() {
            return Err("Risk level is required".to_string());
        }
        if self.max_position_usd.is_empty() {
            return Err("Max position USD is required".to_string());
        }
        if self.price_jump_threshold_bps.is_empty() {
            return Err("Price jump threshold bps is required".to_string());
        }

        let price_jump_threshold_bps = self
            .price_jump_threshold_bps
            .parse::<u32>()
            .map_err(|_| "Price jump threshold bps must be a valid number".to_string())?;

        let task = Task::new(
            self.id.clone(),
            self.symbol.clone(),
            self.account_id.clone(),
            self.risk_level.clone(),
            self.max_position_usd.clone(),
            price_jump_threshold_bps,
            self.base_qty.clone(),
            self.tiers,
        );

        task.validate().map_err(|e| e.to_string())?;
        Ok(task)
    }
}

/// Render the task form dialog
pub fn render(frame: &mut Frame, area: Rect, form: &TaskForm, is_edit: bool) {
    // Create centered popup
    let popup_area = centered_rect(80, 70, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let title = if is_edit {
        format!(" Edit Task: {} ", form.id)
    } else {
        " Create New Task ".to_string()
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

    // ID field
    let id_style = Style::default().fg(Color::Cyan);
    content.push(Line::from(vec![
        Span::styled("ID:                        ", id_style),
        Span::raw(&form.id),
        Span::raw(""),
    ]));
    content.push(Line::from(""));

    // Symbol field
    let symbol_style = if form.focused_field == TaskField::Symbol {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    content.push(Line::from(vec![
        Span::styled("Symbol:                    ", symbol_style),
        Span::raw(&form.symbol),
        if form.focused_field == TaskField::Symbol {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Account field
    let account_style = if form.focused_field == TaskField::AccountId {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let account_display = form
        .account_select
        .selected()
        .map(|option: AccountOption| option.to_string())
        .or_else(|| {
            form.account_select
                .options()
                .get(form.account_select.cursor_index())
                .map(|option: &AccountOption| option.to_string())
        })
        .unwrap_or_else(|| form.account_id.clone());
    content.push(Line::from(vec![
        Span::styled("Account:                   ", account_style),
        Span::raw(account_display),
        if form.focused_field == TaskField::AccountId {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Risk Level field
    let risk_style = if form.focused_field == TaskField::RiskLevel {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    content.push(Line::from(vec![
        Span::styled("Risk Level:                ", risk_style),
        Span::raw(&form.risk_level),
        if form.focused_field == TaskField::RiskLevel {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Max Position USD field
    let max_position_style = if form.focused_field == TaskField::MaxPositionUsd {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    content.push(Line::from(vec![
        Span::styled("Max Position (USD):        ", max_position_style),
        Span::raw(&form.max_position_usd),
        if form.focused_field == TaskField::MaxPositionUsd {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Price Jump Threshold BPS field
    let price_jump_style = if form.focused_field == TaskField::PriceJumpThresholdBps {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    content.push(Line::from(vec![
        Span::styled("Price Jump Threshold (bps):", price_jump_style),
        Span::raw(&form.price_jump_threshold_bps),
        if form.focused_field == TaskField::PriceJumpThresholdBps {
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

    match form.focused_field {
        TaskField::Symbol => form.symbol_select.render(frame, select_area),
        TaskField::AccountId => form.account_select.render(frame, select_area),
        TaskField::RiskLevel => form.risk_level_select.render(frame, select_area),
        _ => {}
    }

    let help_text = match form.focused_field {
        TaskField::Symbol => "选择交易对（单选）",
        TaskField::AccountId => "选择已有账户，或按 Enter 创建新账户",
        TaskField::RiskLevel => "选择风险等级：保守/中性/激进",
        TaskField::MaxPositionUsd => "最大仓位（USD），仅输入数字",
        TaskField::PriceJumpThresholdBps => "价格跳变阈值（bps/sec），仅输入数字",
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
    fn test_task_form_new() {
        let form = TaskForm::new();
        assert!(!form.id.is_empty());
        assert!(Uuid::parse_str(&form.id).is_ok());
        assert_eq!(form.symbol, SYMBOL_OPTIONS[0]);
        assert!(form.account_id.is_empty());
        assert_eq!(form.risk_level, RISK_LEVEL_OPTIONS[0]);
        assert_eq!(form.max_position_usd, DEFAULT_MAX_POSITION_USD);
        assert_eq!(
            form.price_jump_threshold_bps,
            DEFAULT_PRICE_JUMP_THRESHOLD_BPS
        );
        assert_eq!(form.base_qty, DEFAULT_BASE_QTY);
        assert_eq!(form.tiers, DEFAULT_TIERS);
        assert!(form.error_message.is_none());
        assert_eq!(form.focused_field, TaskField::Symbol);
    }

    #[test]
    fn test_task_form_validation_empty_id() {
        let mut form = TaskForm::new();
        form.id.clear();
        form.symbol = "BTC-USD".to_string();
        form.account_id = "account-1".to_string();
        form.risk_level = "conservative".to_string();

        let result = form.to_task();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ID is required"));
    }
}

fn symbol_options() -> Vec<String> {
    SYMBOL_OPTIONS
        .iter()
        .map(|symbol| symbol.to_string())
        .collect()
}

fn risk_level_options() -> Vec<String> {
    RISK_LEVEL_OPTIONS
        .iter()
        .map(|level| level.to_string())
        .collect()
}
