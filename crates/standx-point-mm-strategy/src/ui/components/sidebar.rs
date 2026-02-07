use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::state::{AppState, SidebarMode};
use crate::state::storage::Storage;

/// Render the sidebar showing accounts or tasks
pub fn render(frame: &mut Frame, area: Rect, state: &AppState, _storage: &Storage) {
    // Determine title and items based on sidebar mode
    let (title, items): (&str, Vec<ListItem>) = match state.sidebar_mode {
        SidebarMode::Accounts => {
            let title = "Accounts";
            if !state.accounts.is_empty() {
                let items = state
                    .accounts
                    .iter()
                    .map(|account| {
                        ListItem::new(Line::from(vec![
                            Span::styled("📁 ", Style::default().fg(Color::Yellow)),
                            Span::raw(&account.name),
                            Span::styled(" (", Style::default().fg(Color::Cyan)),
                            Span::styled(&account.id, Style::default().fg(Color::Cyan)),
                            Span::styled(")", Style::default().fg(Color::Cyan)),
                        ]))
                    })
                    .collect();
                (title, items)
            } else {
                let items = vec![
                    ListItem::new(Line::from(vec![
                        Span::styled("📁 ", Style::default().fg(Color::Yellow)),
                        Span::raw("No accounts yet"),
                    ])),
                    ListItem::new(Line::from(vec![
                        Span::styled("💡 ", Style::default().fg(Color::Cyan)),
                        Span::styled("Press 'n' to create", Style::default().fg(Color::Cyan)),
                    ])),
                ];
                (title, items)
            }
        }
        SidebarMode::Tasks => {
            let title = "Tasks";
            if !state.tasks.is_empty() {
                let items = state
                    .tasks
                    .iter()
                    .map(|task| {
                        ListItem::new(Line::from(vec![
                            Span::styled("📋 ", Style::default().fg(Color::Green)),
                            Span::raw(&task.id),
                            Span::styled(" (", Style::default().fg(Color::Cyan)),
                            Span::styled(&task.symbol, Style::default().fg(Color::Cyan)),
                            Span::styled(")", Style::default().fg(Color::Cyan)),
                        ]))
                    })
                    .collect();
                (title, items)
            } else {
                let items = vec![
                    ListItem::new(Line::from(vec![
                        Span::styled("📋 ", Style::default().fg(Color::Green)),
                        Span::raw("No tasks yet"),
                    ])),
                    ListItem::new(Line::from(vec![
                        Span::styled("💡 ", Style::default().fg(Color::Cyan)),
                        Span::styled("Press 'n' to create", Style::default().fg(Color::Cyan)),
                    ])),
                ];
                (title, items)
            }
        }
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_index));

    frame.render_stateful_widget(list, area, &mut list_state);
}
