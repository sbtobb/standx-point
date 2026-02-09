/*
[INPUT]:  AppState selection, UiSnapshot runtime status, and LiveTaskData balance/error data
[OUTPUT]: Account summary panel rendered into Ratatui frame
[POS]:    TUI UI account summary rendering
[UPDATE]: 2026-02-09 Add placeholder module for TUI refactor
[UPDATE]: 2026-02-09 Move draw_account_summary from tui/mod.rs
[UPDATE]: 2026-02-10 Render task price snapshot details
*/

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui::app::{AppState, LiveTaskData, UiSnapshot};
use crate::tui::runtime::{border_style, format_decimal, runtime_label, signed_style};

pub(in crate::tui) fn draw_account_summary(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    app: &AppState,
    snapshot: &UiSnapshot,
) {
    let task = app.selected_task();
    let status = task
        .and_then(|t| snapshot.runtime_status.get(&t.id))
        .map(|status| runtime_label(Some(status)))
        .unwrap_or_else(|| "stopped".to_string());

    let title = match task {
        Some(task) => format!(
            "Account {} | Task {} | Symbol {} | {}",
            task.account_id, task.id, task.symbol, status
        ),
        None => "Account Summary".to_string(),
    };

    let mut lines = Vec::new();
    if let Some(data) = app.selected_live_data() {
        let data: &LiveTaskData = data;
        if let Some(balance) = data.balance.as_ref() {
            let upnl_style = signed_style(balance.upnl);
            let equity = Span::styled(
                format!("Equity {}", format_decimal(balance.equity, 4)),
                Style::default(),
            );
            let upnl = Span::styled(
                format!("uPnL {}", format_decimal(balance.upnl, 4)),
                upnl_style,
            );
            let avail = Span::styled(
                format!("Available {}", format_decimal(balance.cross_available, 4)),
                Style::default(),
            );
            let locked = Span::styled(
                format!("Locked {}", format_decimal(balance.locked, 4)),
                Style::default(),
            );
            lines.push(Line::from(vec![
                equity,
                Span::raw("  "),
                upnl,
                Span::raw("  "),
                avail,
                Span::raw("  "),
                locked,
            ]));
        } else {
            lines.push(Line::from("Balance: -"));
        }

        let price_line = if let Some(price) = data.price_data.as_ref() {
            let mark = format_decimal(price.mark_price.clone(), 4);
            let last = price
                .last_price
                .as_ref()
                .map(|value| format_decimal(value.clone(), 4))
                .unwrap_or_else(|| "-".to_string());
            let min = format_decimal(price.min_price.clone(), 4);
            format!("Mark: {mark} | Last: {last} | Min: {min}")
        } else {
            "Mark: - | Last: - | Min: -".to_string()
        };
        lines.push(Line::from(price_line));

        if let Some(error) = data.last_error.as_ref() {
            lines.push(Line::from(Span::styled(
                format!("Last error: {error}"),
                Style::default().fg(Color::Yellow),
            )));
        } else if let Some(updated) = data.last_update {
            lines.push(Line::from(format!(
                "Last update: {}s",
                updated.elapsed().as_secs()
            )));
        }
    } else {
        lines.push(Line::from("No live data"));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style())
        .title(title);
    let text = Text::from(lines);
    let widget = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}
