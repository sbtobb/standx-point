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
use standx_point_adapter::Side;

use crate::state::storage::Task as StoredTask;
use crate::tui::app::AppState;
use crate::tui::runtime::{border_style, format_decimal, header_style, order_side_style};

const DEFAULT_TP_BPS: i64 = 4;

fn parse_optional_bps(raw: &Option<String>) -> Option<Decimal> {
    let value = raw.as_ref()?.trim();
    if value.is_empty() {
        return None;
    }

    let bps = Decimal::from_str(value).ok()?;
    if bps > Decimal::ZERO {
        Some(bps)
    } else {
        None
    }
}

fn default_sl_multiplier(level: &str) -> Decimal {
    match level.to_ascii_lowercase().as_str() {
        "low" => Decimal::from(2),
        "medium" => Decimal::from(3),
        "high" => Decimal::from(4),
        "xhigh" => Decimal::from(5),
        _ => Decimal::from(2),
    }
}

fn task_tp_sl_bps(task: Option<&StoredTask>) -> (Option<Decimal>, Option<Decimal>) {
    let Some(task) = task else {
        return (None, None);
    };

    let tp_bps = parse_optional_bps(&task.tp_bps).or(Some(Decimal::from(DEFAULT_TP_BPS)));
    let sl_bps = parse_optional_bps(&task.sl_bps)
        .or_else(|| tp_bps.map(|tp| tp * default_sl_multiplier(&task.risk_level)));

    (tp_bps, sl_bps)
}

fn tp_sl_from_bps(price: Option<Decimal>, side: Side, bps: Option<Decimal>) -> Option<Decimal> {
    let price = price?;
    let bps = bps?;
    if price <= Decimal::ZERO || bps <= Decimal::ZERO {
        return None;
    }

    let ratio = bps / Decimal::from(10_000);
    let adjusted = match side {
        Side::Buy => price * (Decimal::ONE + ratio),
        Side::Sell => price * (Decimal::ONE - ratio),
    };
    if adjusted > Decimal::ZERO {
        Some(adjusted)
    } else {
        None
    }
}

fn sl_from_bps(price: Option<Decimal>, side: Side, bps: Option<Decimal>) -> Option<Decimal> {
    let price = price?;
    let bps = bps?;
    if price <= Decimal::ZERO || bps <= Decimal::ZERO {
        return None;
    }

    let ratio = bps / Decimal::from(10_000);
    let adjusted = match side {
        Side::Buy => price * (Decimal::ONE - ratio),
        Side::Sell => price * (Decimal::ONE + ratio),
    };
    if adjusted > Decimal::ZERO {
        Some(adjusted)
    } else {
        None
    }
}

fn parse_payload_value(payload: Option<&str>) -> Option<Value> {
    let parsed = payload.and_then(|raw| serde_json::from_str(raw).ok())?;

    match parsed {
        Value::String(inner) => serde_json::from_str(&inner)
            .ok()
            .or(Some(Value::String(inner))),
        value => Some(value),
    }
}

fn parse_decimal_value(value: &Value) -> Option<Decimal> {
    match value {
        Value::String(raw) => Decimal::from_str(raw).ok(),
        Value::Number(number) => Decimal::from_str(&number.to_string()).ok(),
        _ => None,
    }
}

fn find_decimal_by_keys(value: &Value, keys: &[&str]) -> Option<Decimal> {
    if let Some(decimal) = parse_decimal_value(value) {
        return Some(decimal);
    }

    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(decimal) = map.get(*key).and_then(parse_decimal_value) {
                    return Some(decimal);
                }
            }
            for nested in map.values() {
                if let Some(decimal) = find_decimal_by_keys(nested, keys) {
                    return Some(decimal);
                }
            }
            None
        }
        Value::Array(items) => {
            for item in items {
                if let Some(decimal) = find_decimal_by_keys(item, keys) {
                    return Some(decimal);
                }
            }
            None
        }
        _ => None,
    }
}

fn format_tp_sl_price(
    explicit: Option<Decimal>,
    payload: Option<&Value>,
    derived: Option<Decimal>,
    keys: &[&str],
) -> String {
    if let Some(decimal) = explicit
        .or_else(|| payload.and_then(|value| find_decimal_by_keys(value, keys)))
        .or(derived)
    {
        return format_decimal(decimal, 4);
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
    let (task_tp_bps, task_sl_bps) = task_tp_sl_bps(app.selected_task());

    for order in orders {
        let side_style = order_side_style(order);
        let price = order
            .price
            .map(|p| format_decimal(p, 4))
            .unwrap_or_else(|| "-".to_string());
        let payload_value = parse_payload_value(order.payload.as_deref());
        let derived_tp = tp_sl_from_bps(order.price, order.side, task_tp_bps);
        let derived_sl = sl_from_bps(order.price, order.side, task_sl_bps);
        let tp = format_tp_sl_price(
            order.tp_price,
            payload_value.as_ref(),
            derived_tp,
            &[
                "tp_price",
                "tpPrice",
                "take_profit_price",
                "takeProfitPrice",
                "tp",
            ],
        );
        let sl = format_tp_sl_price(
            order.sl_price,
            payload_value.as_ref(),
            derived_sl,
            &[
                "sl_price",
                "slPrice",
                "stop_loss_price",
                "stopLossPrice",
                "sl",
            ],
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_tp_sl_price_prefers_explicit_field() {
        let payload = serde_json::json!({ "tp_price": "100.0" });

        let formatted = format_tp_sl_price(
            Some(Decimal::from_str("101.0").unwrap()),
            Some(&payload),
            Some(Decimal::from_str("99.0").unwrap()),
            &["tp_price"],
        );

        assert_eq!(formatted, "101.0000");
    }

    #[test]
    fn format_tp_sl_price_reads_nested_payload_fields() {
        let payload = serde_json::json!({
            "risk": {
                "tpPrice": "52000",
                "slPrice": "48000"
            }
        });

        let tp = format_tp_sl_price(
            None,
            Some(&payload),
            Some(Decimal::from_str("1").unwrap()),
            &["tp_price", "tpPrice"],
        );
        let sl = format_tp_sl_price(
            None,
            Some(&payload),
            Some(Decimal::from_str("1").unwrap()),
            &["sl_price", "slPrice"],
        );

        assert_eq!(tp, "52000.0000");
        assert_eq!(sl, "48000.0000");
    }

    #[test]
    fn parse_payload_value_supports_double_encoded_json() {
        let raw = serde_json::to_string(&serde_json::json!({
            "tpPrice": "53000",
            "slPrice": "47000"
        }))
        .expect("serialize payload");
        let wrapped_raw = serde_json::to_string(&raw).expect("wrap payload");
        let payload = parse_payload_value(Some(&wrapped_raw)).expect("payload should parse");

        let tp = format_tp_sl_price(
            None,
            Some(&payload),
            Some(Decimal::from_str("1").unwrap()),
            &["tp_price", "tpPrice"],
        );
        let sl = format_tp_sl_price(
            None,
            Some(&payload),
            Some(Decimal::from_str("1").unwrap()),
            &["sl_price", "slPrice"],
        );

        assert_eq!(tp, "53000.0000");
        assert_eq!(sl, "47000.0000");
    }

    #[test]
    fn format_tp_sl_price_uses_derived_when_payload_missing() {
        let tp = format_tp_sl_price(
            None,
            None,
            Some(Decimal::from_str("111.25").unwrap()),
            &["tp_price"],
        );

        assert_eq!(tp, "111.2500");
    }

    #[test]
    fn task_tp_sl_bps_defaults_for_medium() {
        let task = StoredTask {
            id: "t-1".to_string(),
            symbol: "BTC-USD".to_string(),
            account_id: "a-1".to_string(),
            risk_level: "medium".to_string(),
            budget_usd: "50000".to_string(),
            tp_bps: None,
            sl_bps: None,
            state: crate::state::storage::TaskState::Stopped,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let (tp_bps, sl_bps) = task_tp_sl_bps(Some(&task));
        assert_eq!(tp_bps, Some(Decimal::from(4)));
        assert_eq!(sl_bps, Some(Decimal::from(12)));
    }
}
