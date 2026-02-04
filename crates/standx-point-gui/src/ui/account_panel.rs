/// **Input**: Account model and adapter data (Balance, Position, Order).
/// **Output**: AccountPanel UI rendering balances, positions, and orders.
/// **Position**: UI account detail panel.
/// **Update**: When account data shape or displayed fields change.
use crate::state::Account;
use gpui::StatefulInteractiveElement;
use gpui::*;
use rust_decimal::Decimal;
use standx_point_adapter::types::{Balance, Order, Position, Side};

pub struct AccountPanel {
    pub account: Account,
    pub balance: Option<Balance>,
    pub positions: Vec<Position>,
    pub orders: Vec<Order>,
}

impl AccountPanel {
    pub fn new(account: Account) -> Self {
        Self {
            account,
            balance: None,
            positions: Vec::new(),
            orders: Vec::new(),
        }
    }

    fn format_decimal(d: Decimal, prec: u32) -> String {
        format!("{:.1$}", d, prec as usize)
    }

    fn format_pnl(d: Decimal) -> (String, Hsla) {
        let s = format!("{:.2}", d);
        let color = if d.is_sign_positive() {
            rgb(0x22c55e) // green-500
        } else if d.is_sign_negative() {
            rgb(0xef4444) // red-500
        } else {
            rgb(0x9ca3af) // gray-400
        };
        (s, color.into())
    }

    fn render_header(&self, _window: &Window) -> impl IntoElement {
        let short_addr = if self.account.address.len() > 10 {
            format!(
                "{}...{}",
                &self.account.address[0..6],
                &self.account.address[self.account.address.len() - 4..]
            )
        } else {
            self.account.address.clone()
        };

        div()
            .flex()
            .justify_between()
            .items_center()
            .p_4()
            .bg(rgb(0x2d2d2d))
            .border_b_1()
            .border_color(rgb(0x404040))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child(self.account.alias.clone()),
                    )
                    .child(div().text_sm().text_color(rgb(0xa3a3a3)).child(short_addr)),
            )
            .child(
                div()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(rgb(0x404040))
                    .text_xs()
                    .text_color(rgb(0xe5e5e5))
                    .child(format!("{:?}", self.account.chain)),
            )
    }

    fn render_balance_card(
        &self,
        label: &str,
        value: Option<Decimal>,
        highlight: bool,
    ) -> impl IntoElement {
        let (val_str, color) = if let Some(v) = value {
            if highlight {
                Self::format_pnl(v)
            } else {
                (Self::format_decimal(v, 2), rgb(0xffffff).into())
            }
        } else {
            ("-".to_string(), rgb(0xffffff).into())
        };

        div()
            .flex()
            .flex_col()
            .p_3()
            .bg(rgb(0x2d2d2d))
            .rounded_md()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0xa3a3a3))
                    .mb_1()
                    .child(label.to_string()),
            )
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::BOLD)
                    .text_color(color)
                    .child(val_str),
            )
    }

    fn render_balance_summary(&self, _window: &Window) -> impl IntoElement {
        let balance = self.balance.as_ref();

        div()
            .grid()
            .grid_cols(4)
            .gap_3()
            .p_4()
            .child(self.render_balance_card("Equity", balance.map(|b| b.equity), false))
            .child(self.render_balance_card("Available", balance.map(|b| b.cross_available), false))
            .child(self.render_balance_card("Margin", balance.map(|b| b.cross_margin), false))
            .child(self.render_balance_card("PnL", balance.map(|b| b.upnl), true))
    }

    fn render_positions_table(&self, _window: &Window) -> impl IntoElement {
        let headers = ["Symbol", "Side", "Qty", "Entry", "Mark", "Liq", "PnL"];

        let header_row = div()
            .flex()
            .border_b_1()
            .border_color(rgb(0x404040))
            .py_2()
            .children(headers.iter().map(|h| {
                div()
                    .flex_1()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xa3a3a3))
                    .child(h.to_string())
            }));

        let rows = if self.positions.is_empty() {
            div()
                .flex()
                .justify_center()
                .py_8()
                .text_sm()
                .text_color(rgb(0x737373))
                .child("No positions")
        } else {
            div()
                .flex()
                .flex_col()
                .children(self.positions.iter().map(|p| {
                    let (pnl_str, pnl_color) = Self::format_pnl(p.upnl);
                    let side_color = match p.qty.is_sign_positive() {
                        true => rgb(0x22c55e),  // Long
                        false => rgb(0xef4444), // Short
                    };
                    let side_text = if p.qty.is_sign_positive() {
                        "Long"
                    } else {
                        "Short"
                    };

                    div()
                        .flex()
                        .py_2()
                        .border_b_1()
                        .border_color(rgb(0x333333))
                        .text_sm()
                        .child(div().flex_1().child(p.symbol.clone()))
                        .child(div().flex_1().text_color(side_color).child(side_text))
                        .child(div().flex_1().child(Self::format_decimal(p.qty.abs(), 3)))
                        .child(div().flex_1().child(Self::format_decimal(p.entry_price, 2)))
                        .child(div().flex_1().child(Self::format_decimal(p.mark_price, 2)))
                        .child(div().flex_1().child(Self::format_decimal(p.liq_price, 2)))
                        .child(div().flex_1().text_color(pnl_color).child(pnl_str))
                }))
        };

        div()
            .flex()
            .flex_col()
            .bg(rgb(0x262626))
            .rounded_md()
            .overflow_hidden()
            .child(
                div()
                    .bg(rgb(0x2d2d2d))
                    .px_3()
                    .py_2()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .child("Positions"),
            )
            .child(
                div()
                    .p_3()
                    .child(div().flex().flex_col().child(header_row).child(rows)),
            )
    }

    fn render_orders_table(&self, _window: &Window) -> impl IntoElement {
        let headers = ["Symbol", "Side", "Type", "Price", "Qty", "Filled", "Status"];

        let header_row = div()
            .flex()
            .border_b_1()
            .border_color(rgb(0x404040))
            .py_2()
            .children(headers.iter().map(|h| {
                div()
                    .flex_1()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xa3a3a3))
                    .child(h.to_string())
            }));

        let rows = if self.orders.is_empty() {
            div()
                .flex()
                .justify_center()
                .py_8()
                .text_sm()
                .text_color(rgb(0x737373))
                .child("No open orders")
        } else {
            div()
                .flex()
                .flex_col()
                .children(self.orders.iter().map(|o| {
                    let side_color = match o.side {
                        Side::Buy => rgb(0x22c55e),
                        Side::Sell => rgb(0xef4444),
                    };
                    let price_str = o
                        .price
                        .map(|p| Self::format_decimal(p, 2))
                        .unwrap_or("-".to_string());

                    div()
                        .flex()
                        .py_2()
                        .border_b_1()
                        .border_color(rgb(0x333333))
                        .text_sm()
                        .child(div().flex_1().child(o.symbol.clone()))
                        .child(
                            div()
                                .flex_1()
                                .text_color(side_color)
                                .child(format!("{:?}", o.side)),
                        )
                        .child(div().flex_1().child(format!("{:?}", o.order_type)))
                        .child(div().flex_1().child(price_str))
                        .child(div().flex_1().child(Self::format_decimal(o.qty, 3)))
                        .child(div().flex_1().child(Self::format_decimal(o.fill_qty, 3)))
                        .child(div().flex_1().child(format!("{:?}", o.status)))
                }))
        };

        div()
            .flex()
            .flex_col()
            .bg(rgb(0x262626))
            .rounded_md()
            .overflow_hidden()
            .child(
                div()
                    .bg(rgb(0x2d2d2d))
                    .px_3()
                    .py_2()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .child("Open Orders"),
            )
            .child(
                div()
                    .p_3()
                    .child(div().flex().flex_col().child(header_row).child(rows)),
            )
    }
}

impl Render for AccountPanel {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xe5e5e5))
            .child(self.render_header(window))
            .child(
                div()
                    .id("account_panel_scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .p_4()
                            .child(self.render_balance_summary(window))
                            .child(self.render_positions_table(window))
                            .child(self.render_orders_table(window)),
                    ),
            )
    }
}
