use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::state::{AppState, Pane, SidebarMode};
use crate::state::storage::{Account, Storage, Task};

/// Render the detail view showing account or task details
pub fn render(frame: &mut Frame, area: Rect, state: &AppState, storage: &Storage) {
    let block = Block::default()
        .title("Detail View")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    // If sidebar is focused, show selection hint
    let content = if state.focused_pane == Pane::Sidebar {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("ℹ ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "Select an item from the sidebar",
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("   Press ", Style::default().fg(Color::Gray)),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(" to view details", Style::default().fg(Color::Gray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("   Press ", Style::default().fg(Color::Gray)),
                Span::styled("n", Style::default().fg(Color::Yellow)),
                Span::styled(" to create a new ", Style::default().fg(Color::Gray)),
                Span::styled(
                    match state.sidebar_mode {
                        SidebarMode::Accounts => "account",
                        SidebarMode::Tasks => "task",
                    },
                    Style::default().fg(Color::Cyan),
                ),
            ]),
        ]
    } else {
        // Detail view is focused - show actual content based on selection
        match state.sidebar_mode {
            SidebarMode::Accounts => {
                if state.selected_index < state.accounts.len() {
                    let account = &state.accounts[state.selected_index];
                    render_account_details(account)
                } else {
                    vec![
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("❌ ", Style::default().fg(Color::Red)),
                            Span::styled("Account not found", Style::default().fg(Color::Red)),
                        ]),
                    ]
                }
            }
            SidebarMode::Tasks => {
                if state.selected_index < state.tasks.len() {
                    let task = &state.tasks[state.selected_index];
                    render_task_details(task)
                } else {
                    vec![
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("❌ ", Style::default().fg(Color::Red)),
                            Span::styled("Task not found", Style::default().fg(Color::Red)),
                        ]),
                    ]
                }
            }
        }
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render account details
fn render_account_details(account: &Account) -> Vec<Line> {
    // Mask sensitive fields
    let masked_jwt = if account.jwt_token.len() > 8 {
        format!(
            "{}...{}",
            &account.jwt_token[..4],
            &account.jwt_token[account.jwt_token.len() - 4..]
        )
    } else {
        "****".to_string()
    };

    let masked_signing_key = if account.signing_key.len() > 8 {
        format!(
            "{}...{}",
            &account.signing_key[..4],
            &account.signing_key[account.signing_key.len() - 4..]
        )
    } else {
        "****".to_string()
    };

    let masked_signing_key = if account.signing_key.len() > 8 {
        format!(
            "{}...{}",
            &account.signing_key[..4],
            &account.signing_key[account.signing_key.len() - 4..]
        )
    } else {
        "****".to_string()
    };

    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("📁 ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "Account Details",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   ID: ", Style::default().fg(Color::Gray)),
            Span::styled(&account.id, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Name: ", Style::default().fg(Color::Gray)),
            Span::styled(&account.name, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   JWT Token: ", Style::default().fg(Color::Gray)),
            Span::styled(masked_jwt, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Signing Key: ", Style::default().fg(Color::Gray)),
            Span::styled(masked_signing_key, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Created At: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", account.created_at.format("%Y-%m-%d %H:%M:%S")),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Updated At: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", account.updated_at.format("%Y-%m-%d %H:%M:%S")),
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ]
}

/// Render task details
fn render_task_details(task: &Task) -> Vec<Line> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("📋 ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "Task Details",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   ID: ", Style::default().fg(Color::Gray)),
            Span::styled(&task.id, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Symbol: ", Style::default().fg(Color::Gray)),
            Span::styled(&task.symbol, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Account ID: ", Style::default().fg(Color::Gray)),
            Span::styled(&task.account_id, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Risk Level: ", Style::default().fg(Color::Gray)),
            Span::styled(&task.risk_level, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Max Position (USD): ", Style::default().fg(Color::Gray)),
            Span::styled(&task.max_position_usd, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled(
                "   Price Jump Threshold (bps): ",
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                format!("{}", task.price_jump_threshold_bps),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Base Qty: ", Style::default().fg(Color::Gray)),
            Span::styled(&task.base_qty, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Tiers: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", task.tiers),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("   State: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:?}", task.state),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Created At: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", task.created_at.format("%Y-%m-%d %H:%M:%S")),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Updated At: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", task.updated_at.format("%Y-%m-%d %H:%M:%S")),
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ]
}
