/*
[INPUT]:  AppState selected open orders data
[OUTPUT]: Open orders table rendered into Ratatui frame
[POS]:    TUI UI open orders table rendering
[UPDATE]: 2026-02-09 Add placeholder module for TUI refactor
[UPDATE]: 2026-02-09 Move draw_open_orders_table from tui/mod.rs
[UPDATE]: 2026-02-10 Add TP/SL/Reduce/Time columns with payload parsing
*/

use std::str::FromStr;

use ratatui::layout::Constraint;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use rust_decimal::Decimal;
use serde_json::Value;

use crate::tui::app::AppState;
use crate::tui::runtime::{border_style, format_decimal, header_style, order_side_style};

fn parse_payload_value(payload: Option<&str>) -> Option<Value> {
    payload.and_then(|raw| serde_json::from_str(raw).ok())
}

fn parse_decimal_value(value: &Value) -> Option<Decimal> {
    match value {
        Value::String(raw) => Decimal::from_str(raw).ok(),
        Value::Number(number) => Decimal::from_str(&number.to_string()).ok(),
        _ => None,
    }
}

fn format_payload_decimal(payload: Option<&Value>, keys: &[&str]) -> String {
    let Some(payload) = payload else {
        return "-".to_string();
    };

    for key in keys {
        if let Some(decimal) = payload.get(*key).and_then(parse_decimal_value) {
            return format_decimal(decimal, 4);
        }
    }

    "-".to_string()
}

fn format_order_time(created_at: &str) -> String {
    let time_part = created_at
        .split_once('T')
        .map(|(_, time)| time)
        .or_else(|| created_at.split_once(' ').map(|(_, time)| time));

    let Some(time_part) = time_part else {
        return created_at.to_string();
    };

    let time_part = time_part
        .split(|ch| ch == 'Z' || ch == '+' || ch == '-')
        .next()
        .unwrap_or(time_part);
    let time_part = time_part.split('.').next().unwrap_or(time_part);
    let trimmed: String = time_part.chars().take(8).collect();

    if trimmed.is_empty() {
        created_at.to_string()
    } else {
        trimmed
    }
}

pub(in crate::tui) fn draw_open_orders_table(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    app: &AppState,
) {
    let mut rows = Vec::new();
    let orders = app
        .selected_live_data()
        .map(|data| data.open_orders.as_slice())
        .unwrap_or(&[]);

    for order in orders {
        let side_style = order_side_style(order);
        let price = order
            .price
            .map(|p| format_decimal(p, 4))
            .unwrap_or_else(|| "-".to_string());
        let payload_value = parse_payload_value(order.payload.as_deref());
        let tp = format_payload_decimal(payload_value.as_ref(), &["tp_price", "tpPrice", "tp"]);
        let sl = format_payload_decimal(payload_value.as_ref(), &["sl_price", "slPrice", "sl"]);
        let reduce_only = if order.reduce_only { "Yes" } else { "No" };
        let created_time = format_order_time(&order.created_at);
        let price_cell = format!("{price:>12}");
        let qty_cell = format!("{:>12}", format_decimal(order.qty, 4));
        let tp_cell = format!("{tp:>10}");
        let sl_cell = format!("{sl:>10}");
        let reduce_cell = format!("{reduce_only:^8}");
        let time_cell = format!("{created_time:^10}");
        rows.push(Row::new(vec![
            Cell::from(order.symbol.as_str()),
            Cell::from(Span::styled(format!("{:?}", order.side), side_style)),
            Cell::from(format!("{:?}", order.order_type)),
            Cell::from(price_cell),
            Cell::from(qty_cell),
            Cell::from(tp_cell),
            Cell::from(sl_cell),
            Cell::from(reduce_cell),
            Cell::from(time_cell),
            Cell::from(format!("{:?}", order.status)),
        ]));
    }

    if rows.is_empty() {
        rows.push(Row::new(vec![
            Cell::from("No open orders"),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ]));
    }

    let header = Row::new(vec![
        Cell::from("Symbol"),
        Cell::from("Side"),
        Cell::from("Type"),
        Cell::from("Price"),
        Cell::from("Qty"),
        Cell::from("TP"),
        Cell::from("SL"),
        Cell::from("Reduce"),
        Cell::from("Time"),
        Cell::from("Status"),
    ])
    .style(header_style());

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style())
            .title("Open Orders"),
    );
    frame.render_widget(table, area);
}
