/*
[INPUT]:  AppState selected live positions data
[OUTPUT]: Positions table rendered into Ratatui frame
[POS]:    TUI UI positions table rendering
[UPDATE]: 2026-02-09 Add placeholder module for TUI refactor
[UPDATE]: 2026-02-09 Move draw_positions_table from tui/mod.rs
*/

use ratatui::layout::Constraint;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};

use crate::tui::app::AppState;
use crate::tui::runtime::{border_style, format_decimal, header_style, signed_style};

pub(in crate::tui) fn draw_positions_table(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    app: &AppState,
) {
    let mut rows = Vec::new();
    let positions = app
        .selected_live_data()
        .map(|data| data.positions.as_slice())
        .unwrap_or(&[]);

    for position in positions.iter().filter(|p| !p.qty.is_zero()) {
        let side_style = signed_style(position.qty);
        let upnl_style = signed_style(position.upnl);
        rows.push(Row::new(vec![
            Cell::from(position.symbol.as_str()),
            Cell::from(Span::styled(format_decimal(position.qty, 4), side_style)),
            Cell::from(format_decimal(position.entry_price, 4)),
            Cell::from(format_decimal(position.mark_price, 4)),
            Cell::from(Span::styled(format_decimal(position.upnl, 4), upnl_style)),
        ]));
    }

    if rows.is_empty() {
        rows.push(Row::new(vec![
            Cell::from("No positions"),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ]));
    }

    let header = Row::new(vec![
        Cell::from("Symbol"),
        Cell::from("Qty"),
        Cell::from("Entry"),
        Cell::from("Mark"),
        Cell::from("uPnL"),
    ])
    .style(header_style());

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style())
            .title("Positions"),
    );
    frame.render_widget(table, area);
}
