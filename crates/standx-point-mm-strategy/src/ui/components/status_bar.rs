use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::state::AppState;

/// Render the status bar at the top of the screen
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Blue));

    // Create the content lines
    let mut lines = vec![];

    // Title and mode line
    let mode_str = match state.mode {
        crate::app::state::AppMode::Normal => "NORMAL",
        crate::app::state::AppMode::Insert => "INSERT",
        crate::app::state::AppMode::Dialog => "DIALOG",
    };

    let title_spans = vec![
        Span::styled(
            "StandX MM Strategy",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("Mode: {}", mode_str),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("Sidebar: {:?}", state.sidebar_mode),
            Style::default().fg(Color::Green),
        ),
    ];
    lines.push(Line::from(title_spans));

    // Status message line
    if let Some(ref msg) = state.status_message {
        let status_spans = vec![
            Span::styled("Status: ", Style::default().fg(Color::Blue)),
            Span::styled(msg.clone(), Style::default().fg(Color::White)),
        ];
        lines.push(Line::from(status_spans));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}
