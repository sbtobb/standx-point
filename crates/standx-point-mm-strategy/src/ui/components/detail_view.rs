use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use rust_decimal::Decimal;

use crate::app::state::{AccountDetail, AppState, Pane, SidebarMode};
use crate::state::storage::{Account, Storage, Task};
use standx_point_mm_strategy::task::TaskRuntimeStatus;

/// Render the detail view showing account or task details
pub fn render(frame: &mut Frame, area: Rect, state: &AppState, _storage: &Storage) {
    let block = Block::default()
        .title("Detail View")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    // If sidebar or menu is focused, show selection hint
    let content = if state.focused_pane != Pane::Detail {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("ℹ ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "Select an item from the sidebar",
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("   Press ", Style::default().fg(Color::Cyan)),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(" to view details", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("   Press ", Style::default().fg(Color::Cyan)),
                Span::styled("n", Style::default().fg(Color::Yellow)),
                Span::styled(" to create a new ", Style::default().fg(Color::Cyan)),
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
                    let selected_account = &state.accounts[state.selected_index];
                    // Get account by ID from the cached list
                    if let Some(account) =
                        state.accounts.iter().find(|a| a.id == selected_account.id)
                    {
                        let detail = state.account_details.get(&account.id);
                        render_account_details(account, state.show_credentials, detail)
                    } else {
                        vec![
                            Line::from(""),
                            Line::from(vec![
                                Span::styled("❌ ", Style::default().fg(Color::Red)),
                                Span::styled("Account not found", Style::default().fg(Color::Red)),
                            ]),
                        ]
                    }
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
                    let selected_task = &state.tasks[state.selected_index];
                    // Get task by ID from the cached list
                    if let Some(task) = state.tasks.iter().find(|t| t.id == selected_task.id) {
                        let runtime_status = state.runtime_status.get(&task.id).copied();
                        render_task_details(task, runtime_status)
                    } else {
                        vec![
                            Line::from(""),
                            Line::from(vec![
                                Span::styled("❌ ", Style::default().fg(Color::Red)),
                                Span::styled("Task not found", Style::default().fg(Color::Red)),
                            ]),
                        ]
                    }
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
fn render_account_details<'a>(
    account: &'a Account,
    show_credentials: bool,
    detail: Option<&'a AccountDetail>,
) -> Vec<Line<'a>> {
    // Mask sensitive fields if needed
    let displayed_jwt = if show_credentials {
        account.jwt_token.clone()
    } else if account.jwt_token.len() > 8 {
        format!(
            "{}...{}",
            &account.jwt_token[..4],
            &account.jwt_token[account.jwt_token.len() - 4..]
        )
    } else {
        "****".to_string()
    };

    let displayed_signing_key = if show_credentials {
        account.signing_key.clone()
    } else if account.signing_key.len() > 8 {
        format!(
            "{}...{}",
            &account.signing_key[..4],
            &account.signing_key[account.signing_key.len() - 4..]
        )
    } else {
        "****".to_string()
    };

    let chain_label = account
        .chain
        .map(|chain| format!("{:?}", chain))
        .unwrap_or_else(|| "Unknown".to_string());

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("📁 ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "Account Details",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   ID: ", Style::default().fg(Color::Cyan)),
            Span::styled(&account.id, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Name: ", Style::default().fg(Color::Cyan)),
            Span::styled(&account.name, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Chain: ", Style::default().fg(Color::Cyan)),
            Span::styled(chain_label, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   JWT Token: ", Style::default().fg(Color::Cyan)),
            Span::styled(displayed_jwt, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Signing Key: ", Style::default().fg(Color::Cyan)),
            Span::styled(displayed_signing_key, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Created At: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{}", account.created_at.format("%Y-%m-%d %H:%M:%S")),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Updated At: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{}", account.updated_at.format("%Y-%m-%d %H:%M:%S")),
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ];

    lines.extend(render_account_live_data(detail));

    lines
}

fn render_account_live_data<'a>(detail: Option<&'a AccountDetail>) -> Vec<Line<'a>> {
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("   > ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "Live Account Data",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    let Some(detail) = detail else {
        lines.push(Line::from(vec![
            Span::styled("   Status: ", Style::default().fg(Color::Cyan)),
            Span::styled("not loaded", Style::default().fg(Color::Yellow)),
        ]));
        return lines;
    };

    let balance = detail.balance.as_ref();
    lines.push(Line::from(vec![
        Span::styled("   Equity: ", Style::default().fg(Color::Cyan)),
        render_decimal_or_placeholder(balance.map(|b| b.equity), 2),
    ]));
    lines.push(Line::from(vec![
        Span::styled("   Available: ", Style::default().fg(Color::Cyan)),
        render_decimal_or_placeholder(balance.map(|b| b.cross_available), 2),
    ]));
    lines.push(Line::from(vec![
        Span::styled("   Margin: ", Style::default().fg(Color::Cyan)),
        render_decimal_or_placeholder(balance.map(|b| b.cross_margin), 2),
    ]));
    lines.push(Line::from(vec![
        Span::styled("   PnL: ", Style::default().fg(Color::Cyan)),
        render_pnl_or_placeholder(balance.map(|b| b.upnl)),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("   Positions: ", Style::default().fg(Color::Cyan)),
        Span::styled(
            format!("{}", detail.positions.len()),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    if detail.positions.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("     ", Style::default().fg(Color::Cyan)),
            Span::styled("No open positions", Style::default().fg(Color::Cyan)),
        ]));
    } else {
        for position in detail.positions.iter().take(5) {
            let (side_label, side_style) = if position.qty.is_sign_positive() {
                ("Long", Style::default().fg(Color::Green))
            } else {
                ("Short", Style::default().fg(Color::Red))
            };
            let qty = position.qty.abs();

            lines.push(Line::from(vec![
                Span::styled("     ", Style::default().fg(Color::Cyan)),
                Span::styled(position.symbol.clone(), Style::default().fg(Color::Yellow)),
                Span::styled(" ", Style::default().fg(Color::Cyan)),
                Span::styled(side_label, side_style),
                Span::styled(" qty=", Style::default().fg(Color::Cyan)),
                Span::styled(format_decimal(qty, 3), Style::default().fg(Color::Yellow)),
                Span::styled(" entry=", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format_decimal(position.entry_price, 2),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(" mark=", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format_decimal(position.mark_price, 2),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(" upnl=", Style::default().fg(Color::Cyan)),
                Span::styled(format_decimal(position.upnl, 2), pnl_style(position.upnl)),
            ]));
        }
    }

    if let Some(last_updated) = detail.last_updated {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("   Updated: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{}s ago", last_updated.elapsed().as_secs()),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }

    if let Some(err) = detail.last_error.as_ref() {
        lines.push(Line::from(vec![
            Span::styled("   Last Error: ", Style::default().fg(Color::Cyan)),
            Span::styled(err.clone(), Style::default().fg(Color::Red)),
        ]));
    }

    lines
}

fn format_decimal(value: Decimal, precision: u32) -> String {
    format!("{:.1$}", value, precision as usize)
}

fn render_decimal_or_placeholder<'a>(value: Option<Decimal>, precision: u32) -> Span<'a> {
    match value {
        Some(v) => Span::styled(
            format_decimal(v, precision),
            Style::default().fg(Color::Yellow),
        ),
        None => Span::styled("--", Style::default().fg(Color::Cyan)),
    }
}

fn render_pnl_or_placeholder<'a>(value: Option<Decimal>) -> Span<'a> {
    match value {
        Some(v) => Span::styled(format_decimal(v, 2), pnl_style(v)),
        None => Span::styled("--", Style::default().fg(Color::Cyan)),
    }
}

fn pnl_style(value: Decimal) -> Style {
    if value.is_sign_positive() {
        Style::default().fg(Color::Green)
    } else if value.is_sign_negative() {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Yellow)
    }
}

/// Render task details
fn render_task_details(task: &Task, runtime_status: Option<TaskRuntimeStatus>) -> Vec<Line<'_>> {
    let runtime_label = match runtime_status {
        Some(TaskRuntimeStatus::Running) => "Running",
        Some(TaskRuntimeStatus::Finished) => "Finished",
        None => "Unknown",
    };
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("📋 ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "Task Details",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   ID: ", Style::default().fg(Color::Cyan)),
            Span::styled(&task.id, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Symbol: ", Style::default().fg(Color::Cyan)),
            Span::styled(&task.symbol, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Account ID: ", Style::default().fg(Color::Cyan)),
            Span::styled(&task.account_id, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Risk Level: ", Style::default().fg(Color::Cyan)),
            Span::styled(&task.risk_level, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Budget (USD): ", Style::default().fg(Color::Cyan)),
            Span::styled(&task.budget_usd, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   State: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:?}", task.state),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Runtime: ", Style::default().fg(Color::Cyan)),
            Span::styled(runtime_label, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("   Created At: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{}", task.created_at.format("%Y-%m-%d %H:%M:%S")),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Updated At: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{}", task.updated_at.format("%Y-%m-%d %H:%M:%S")),
                Style::default().fg(Color::Yellow),
            ),
        ]),
    ]
}
