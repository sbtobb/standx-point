use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::state::{AppState, Pane};

/// Render the menu bar at the bottom of the screen
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let menu_items = vec![
        ("Ctrl+1", "Help"),
        ("Ctrl+2", "Accounts"),
        ("Ctrl+3", "Tasks"),
        ("n", "New"),
        ("e", "Edit"),
        ("d", "Delete"),
        ("s", "Start"),
        ("x", "Stop"),
        ("q", "Quit"),
    ];

    let mut spans = vec![];

    // Visual indicator for focus
    let base_style = if state.focused_pane == Pane::Menu {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };

    for (i, (key, desc)) in menu_items.iter().enumerate() {
        // Add separator if not first item
        if i > 0 {
            spans.push(Span::styled(" | ", base_style));
        }

        // Key in bold highlight color
        spans.push(Span::styled(
            *key,
            base_style.fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));

        // Description in normal color
        spans.push(Span::styled(
            format!(" {}", desc),
            base_style.fg(Color::Cyan),
        ));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line)
        .alignment(Alignment::Center)
        .style(base_style);

    frame.render_widget(paragraph, area);
}
