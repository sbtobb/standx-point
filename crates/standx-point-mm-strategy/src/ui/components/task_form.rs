/// **Input**: `Task` domain model and ratatui layout/style primitives.
/// **Output**: `TaskForm` state and `render` function for the task dialog.
/// **Position**: TUI component for task create/edit dialog rendering.
/// **Update**: Revisit when form fields or validation rules change.
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::state::storage::Task;

/// Task form data for creating/editing tasks
#[derive(Debug, Clone, PartialEq)]
pub struct TaskForm {
    pub id: String,
    pub symbol: String,
    pub account_id: String,
    pub risk_level: String,
    pub max_position_usd: String,
    pub price_jump_threshold_bps: String,
    pub base_qty: String,
    pub tiers: String,
    pub error_message: Option<String>,
    pub focused_field: usize, // 0=id, 1=symbol, 2=account_id, 3=risk_level, 4=max_position_usd, 5=price_jump_threshold_bps, 6=base_qty, 7=tiers
}

impl TaskForm {
    pub fn new() -> Self {
        Self {
            id: String::new(),
            symbol: String::new(),
            account_id: String::new(),
            risk_level: String::new(),
            max_position_usd: String::new(),
            price_jump_threshold_bps: String::new(),
            base_qty: String::new(),
            tiers: String::new(),
            error_message: None,
            focused_field: 0,
        }
    }

    pub fn from_task(task: &Task) -> Self {
        Self {
            id: task.id.clone(),
            symbol: task.symbol.clone(),
            account_id: task.account_id.clone(),
            risk_level: task.risk_level.clone(),
            max_position_usd: task.max_position_usd.clone(),
            price_jump_threshold_bps: task.price_jump_threshold_bps.to_string(),
            base_qty: task.base_qty.clone(),
            tiers: task.tiers.to_string(),
            error_message: None,
            focused_field: 0,
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
        if self.base_qty.is_empty() {
            return Err("Base quantity is required".to_string());
        }
        if self.tiers.is_empty() {
            return Err("Tiers is required".to_string());
        }

        let price_jump_threshold_bps = self
            .price_jump_threshold_bps
            .parse::<u32>()
            .map_err(|_| "Price jump threshold bps must be a valid number".to_string())?;

        let tiers = self
            .tiers
            .parse::<u8>()
            .map_err(|_| "Tiers must be a valid number".to_string())?;

        let task = Task::new(
            self.id.clone(),
            self.symbol.clone(),
            self.account_id.clone(),
            self.risk_level.clone(),
            self.max_position_usd.clone(),
            price_jump_threshold_bps,
            self.base_qty.clone(),
            tiers,
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
        Span::styled("ID:                        ", id_style),
        Span::raw(&form.id),
        if form.focused_field == 0 {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Symbol field
    let symbol_style = if form.focused_field == 1 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    content.push(Line::from(vec![
        Span::styled("Symbol:                    ", symbol_style),
        Span::raw(&form.symbol),
        if form.focused_field == 1 {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Account ID field
    let account_style = if form.focused_field == 2 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    content.push(Line::from(vec![
        Span::styled("Account ID:                ", account_style),
        Span::raw(&form.account_id),
        if form.focused_field == 2 {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Risk Level field
    let risk_style = if form.focused_field == 3 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    content.push(Line::from(vec![
        Span::styled("Risk Level:                ", risk_style),
        Span::raw(&form.risk_level),
        if form.focused_field == 3 {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Max Position USD field
    let max_position_style = if form.focused_field == 4 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    content.push(Line::from(vec![
        Span::styled("Max Position (USD):        ", max_position_style),
        Span::raw(&form.max_position_usd),
        if form.focused_field == 4 {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Price Jump Threshold BPS field
    let price_jump_style = if form.focused_field == 5 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    content.push(Line::from(vec![
        Span::styled("Price Jump Threshold (bps):", price_jump_style),
        Span::raw(&form.price_jump_threshold_bps),
        if form.focused_field == 5 {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Base Quantity field
    let base_qty_style = if form.focused_field == 6 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    content.push(Line::from(vec![
        Span::styled("Base Quantity:             ", base_qty_style),
        Span::raw(&form.base_qty),
        if form.focused_field == 6 {
            Span::styled(" █", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]));
    content.push(Line::from(""));

    // Tiers field
    let tiers_style = if form.focused_field == 7 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    content.push(Line::from(vec![
        Span::styled("Tiers:                     ", tiers_style),
        Span::raw(&form.tiers),
        if form.focused_field == 7 {
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
    fn test_task_form_new() {
        let form = TaskForm::new();
        assert!(form.id.is_empty());
        assert!(form.symbol.is_empty());
        assert!(form.account_id.is_empty());
        assert!(form.risk_level.is_empty());
        assert!(form.max_position_usd.is_empty());
        assert!(form.price_jump_threshold_bps.is_empty());
        assert!(form.base_qty.is_empty());
        assert!(form.tiers.is_empty());
        assert!(form.error_message.is_none());
        assert_eq!(form.focused_field, 0);
    }

    #[test]
    fn test_task_form_validation_empty_id() {
        let form = TaskForm {
            id: "".to_string(),
            symbol: "BTC-USD".to_string(),
            account_id: "account-1".to_string(),
            risk_level: "conservative".to_string(),
            max_position_usd: "50000".to_string(),
            price_jump_threshold_bps: "5".to_string(),
            base_qty: "0.1".to_string(),
            tiers: "2".to_string(),
            error_message: None,
            focused_field: 0,
        };

        let result = form.to_task();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ID is required"));
    }
}
