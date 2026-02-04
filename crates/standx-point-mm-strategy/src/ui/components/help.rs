use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

/// Render the help overlay showing all keyboard shortcuts
pub fn render(frame: &mut Frame, area: Rect) {
    // Create a centered popup
    let popup_area = centered_rect(70, 80, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Help - Keyboard Shortcuts ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let content = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  j / ↓    ", Style::default().fg(Color::Cyan)),
            Span::raw("Move selection down"),
        ]),
        Line::from(vec![
            Span::styled("  k / ↑    ", Style::default().fg(Color::Cyan)),
            Span::raw("Move selection up"),
        ]),
        Line::from(vec![
            Span::styled("  h / ←    ", Style::default().fg(Color::Cyan)),
            Span::raw("Focus sidebar"),
        ]),
        Line::from(vec![
            Span::styled("  l / →    ", Style::default().fg(Color::Cyan)),
            Span::raw("Focus detail view"),
        ]),
        Line::from(vec![
            Span::styled("  Tab      ", Style::default().fg(Color::Cyan)),
            Span::raw("Cycle focus"),
        ]),
        Line::from(vec![
            Span::styled("  Enter    ", Style::default().fg(Color::Cyan)),
            Span::raw("Select / Confirm"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Mode Switching",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  F1       ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("  F2       ", Style::default().fg(Color::Cyan)),
            Span::raw("Switch to Accounts"),
        ]),
        Line::from(vec![
            Span::styled("  F3       ", Style::default().fg(Color::Cyan)),
            Span::raw("Switch to Tasks"),
        ]),
        Line::from(vec![
            Span::styled("  F4       ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle credentials visibility"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  n        ", Style::default().fg(Color::Cyan)),
            Span::raw("Create new item"),
        ]),
        Line::from(vec![
            Span::styled("  e        ", Style::default().fg(Color::Cyan)),
            Span::raw("Edit selected item"),
        ]),
        Line::from(vec![
            Span::styled("  d        ", Style::default().fg(Color::Cyan)),
            Span::raw("Delete selected item"),
        ]),
        Line::from(vec![
            Span::styled("  s        ", Style::default().fg(Color::Cyan)),
            Span::raw("Start task"),
        ]),
        Line::from(vec![
            Span::styled("  x        ", Style::default().fg(Color::Cyan)),
            Span::raw("Stop task"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  q / Esc  ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit application"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press F1 or Esc to close...",
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        )]),
    ];

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
