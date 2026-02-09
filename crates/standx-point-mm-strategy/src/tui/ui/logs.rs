/*
[INPUT]:  LogBufferHandle snapshots for UI
[OUTPUT]: Log panel rendered into Ratatui frame
[POS]:    TUI UI logs panel rendering
[UPDATE]: 2026-02-09 Add placeholder module for TUI refactor
[UPDATE]: 2026-02-09 Move draw_logs from tui/mod.rs
*/

use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::runtime::border_style;
use crate::tui::LogBufferHandle;

pub(in crate::tui) fn draw_logs(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    buffer: &LogBufferHandle,
) {
    let lines = {
        let guard = buffer.lock().expect("log buffer lock");
        guard.snapshot()
    };
    let available = area.height.saturating_sub(2) as usize;
    let start = lines.len().saturating_sub(available);
    let view = &lines[start..];

    let text = view
        .iter()
        .map(|line| Line::from(Span::raw(line.clone())))
        .collect::<Vec<_>>();
    let log_widget = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style())
            .title("Logs"),
    );
    frame.render_widget(log_widget, area);
}
