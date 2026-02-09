/*
[INPUT]:  Frame layout regions and constraints (placeholder)
[OUTPUT]: Layout helpers for TUI UI (placeholder)
[POS]:    TUI UI layout module placeholder
[UPDATE]: 2026-02-09 Add placeholder module for TUI refactor
[UPDATE]: 2026-02-09 Add tab bar renderer
[UPDATE]: 2026-02-10 Use shared tab bar renderer
*/

use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Tabs};

use crate::tui::app::Tab;
use crate::tui::runtime::{border_style, header_style};

pub(in crate::tui) fn draw_tabs(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    current_tab: Tab,
) {
    let titles = vec![
        Line::from("Dashboard"),
        Line::from("Logs"),
        Line::from("Create"),
    ];
    let selected = match current_tab {
        Tab::Dashboard => 0,
        Tab::Logs => 1,
        Tab::Create => 2,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style())
                .title("Tabs"),
        )
        .highlight_style(header_style())
        .select(selected);

    frame.render_widget(tabs, area);
}
