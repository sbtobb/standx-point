use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::state::{AppState, Pane, SidebarMode};

/// Render the detail view showing account or task details
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
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
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("📋 ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "Detail View",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("   Mode: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{:?}", state.sidebar_mode),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(vec![
                Span::styled("   Selection: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    state.selected_index.to_string(),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("   ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "Item details will appear here",
                    Style::default().fg(Color::Gray),
                ),
            ]),
        ]
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
