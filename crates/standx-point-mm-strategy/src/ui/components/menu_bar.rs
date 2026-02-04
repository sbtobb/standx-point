use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::state::AppState;

/// Render the menu bar at the bottom of the screen
pub fn render(frame: &mut Frame, area: Rect, _state: &AppState) {
    let menu_items = vec![
        ("F1", "Help"),
        ("F2", "Accounts"),
        ("F3", "Tasks"),
        ("n", "New"),
        ("e", "Edit"),
        ("d", "Delete"),
        ("s", "Start"),
        ("x", "Stop"),
        ("q", "Quit"),
    ];

    let mut spans = vec![];
    for (i, (key, desc)) in menu_items.iter().enumerate() {
        // Add separator if not first item
        if i > 0 {
            spans.push(Span::raw(" | "));
        }

        // Key in bold highlight color
        spans.push(Span::styled(
            *key,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));

        // Description in normal color
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(Color::Gray),
        ));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
