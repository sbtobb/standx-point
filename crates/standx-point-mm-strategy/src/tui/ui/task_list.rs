/*
[INPUT]:  AppState task list and UiSnapshot runtime/metrics data
[OUTPUT]: Task list rendered into Ratatui frame
[POS]:    TUI UI task list rendering
[UPDATE]: 2026-02-09 Add placeholder module for TUI refactor
[UPDATE]: 2026-02-09 Move draw_task_list from tui/mod.rs
*/

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::app::{AppState, UiSnapshot};
use crate::tui::runtime::{border_style, runtime_label};

pub(in crate::tui) fn draw_task_list(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    app: &mut AppState,
    snapshot: &UiSnapshot,
) {
    let items = if app.tasks.is_empty() {
        vec![ListItem::new("No tasks found")]
    } else {
        app.tasks
            .iter()
            .map(|task| {
                let status = runtime_label(snapshot.runtime_status.get(&task.id));
                let metrics = snapshot.metrics.get(&task.id);
                let (orders, position) = metrics
                    .map(|m| (m.open_orders, m.position_qty.to_string()))
                    .unwrap_or((0, "-".to_string()));
                let line = format!(
                    "{} | {} | {} | ord:{} pos:{}",
                    task.id, task.symbol, status, orders, position
                );
                ListItem::new(line)
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style())
                .title("Tasks"),
        )
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, area, &mut app.list_state);
}
