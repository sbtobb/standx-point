use crate::state::{Account, Task, TaskStatus};
use gpui::*;
use standx_point_adapter::types::{Order, Side, Trade};

pub struct TaskDetailPanel {
    task: Option<Task>,
    account: Option<Account>,
    order_history: Vec<Order>,
    trade_history: Vec<Trade>,
    operation_logs: Vec<OperationLog>,
}

#[derive(Clone, Debug)]
pub struct OperationLog {
    pub timestamp: i64,
    pub action: String,
    pub details: String,
}

impl TaskDetailPanel {
    pub fn new() -> Self {
        Self {
            task: None,
            account: None,
            order_history: Vec::new(),
            trade_history: Vec::new(),
            operation_logs: Vec::new(),
        }
    }

    pub fn set_task(&mut self, task: Task) {
        self.task = Some(task);
    }

    pub fn set_account(&mut self, account: Account) {
        self.account = Some(account);
    }

    pub fn set_order_history(&mut self, orders: Vec<Order>) {
        self.order_history = orders;
    }

    pub fn set_trade_history(&mut self, trades: Vec<Trade>) {
        self.trade_history = trades;
    }

    pub fn set_operation_logs(&mut self, logs: Vec<OperationLog>) {
        self.operation_logs = logs;
    }

    fn render_task_header(&self, task: &Task) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .bg(rgb(0x2d2d2d))
            .rounded_md()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .child(task.name.clone()),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .rounded_md()
                            .bg(rgb(0x404040))
                            .text_sm()
                            .child(task.symbol.clone()),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xaaaaaa))
                    .child(format!("ID: {}", task.id)),
            )
    }

    fn render_status_section(&self, task: &Task) -> impl IntoElement {
        let (status_color, status_text) = match task.status {
            TaskStatus::Running => (rgb(0x4caf50), "Running"),
            TaskStatus::Paused => (rgb(0xff9800), "Paused"),
            TaskStatus::Stopped => (rgb(0xf44336), "Stopped"),
            TaskStatus::Failed => (rgb(0xf44336), "Failed"),
            TaskStatus::Draft => (rgb(0x9e9e9e), "Draft"),
            TaskStatus::Pending => (rgb(0x2196f3), "Pending"),
        };

        div()
            .flex()
            .items_center()
            .gap_2()
            .p_4()
            .child("Status:")
            .child(
                div()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(status_color)
                    .text_color(rgb(0xffffff))
                    .child(status_text),
            )
    }

    fn render_account_section(&self) -> impl IntoElement {
        if let Some(ref account) = self.account {
            div().p_4().child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().font_weight(FontWeight::BOLD).child("Account"))
                    .child(
                        div().flex().gap_2().child(account.alias.clone()).child(
                            div()
                                .text_color(rgb(0xaaaaaa))
                                .child(format!("({})", truncate_address(&account.address))),
                        ),
                    ),
            )
        } else {
            div().child("No account associated")
        }
    }

    fn render_order_history_table(&self) -> impl IntoElement {
        let headers = ["Time", "Symbol", "Side", "Type", "Price", "Qty", "Status"];

        div()
            .flex()
            .flex_col()
            .p_4()
            .gap_2()
            .child(div().font_weight(FontWeight::BOLD).child("Order History"))
            .child(
                div()
                    .w_full()
                    .border_1()
                    .border_color(rgb(0x404040))
                    .rounded_md()
                    .child(
                        div().flex().bg(rgb(0x333333)).p_2().children(
                            headers
                                .iter()
                                .map(|h| div().flex_1().font_weight(FontWeight::BOLD).child(*h)),
                        ),
                    )
                    .child(if self.order_history.is_empty() {
                        div().p_4().child("No data")
                    } else {
                        div().flex().flex_col().children(
                            self.order_history
                                .iter()
                                .take(50)
                                .enumerate()
                                .map(|(i, order)| {
                                    let bg = if i % 2 == 0 {
                                        rgb(0x252525)
                                    } else {
                                        rgb(0x2a2a2a)
                                    };
                                    div()
                                        .flex()
                                        .bg(bg)
                                        .p_2()
                                        .child(div().flex_1().child(order.created_at.clone()))
                                        .child(div().flex_1().child(order.symbol.clone()))
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_color(match order.side {
                                                    Side::Buy => rgb(0x4caf50),
                                                    Side::Sell => rgb(0xf44336),
                                                })
                                                .child(format!("{:?}", order.side)),
                                        )
                                        .child(
                                            div().flex_1().child(format!("{:?}", order.order_type)),
                                        )
                                        .child(div().flex_1().child(
                                            order.price.map_or("-".to_string(), |p| p.to_string()),
                                        ))
                                        .child(div().flex_1().child(order.qty.to_string()))
                                        .child(div().flex_1().child(format!("{:?}", order.status)))
                                }),
                        )
                    }),
            )
    }

    fn render_trade_history_table(&self) -> impl IntoElement {
        let headers = ["Time", "Symbol", "Side", "Price", "Qty", "Fee", "PnL"];

        div()
            .flex()
            .flex_col()
            .p_4()
            .gap_2()
            .child(div().font_weight(FontWeight::BOLD).child("Trade History"))
            .child(
                div()
                    .w_full()
                    .border_1()
                    .border_color(rgb(0x404040))
                    .rounded_md()
                    .child(
                        div().flex().bg(rgb(0x333333)).p_2().children(
                            headers
                                .iter()
                                .map(|h| div().flex_1().font_weight(FontWeight::BOLD).child(*h)),
                        ),
                    )
                    .child(if self.trade_history.is_empty() {
                        div().p_4().child("No data")
                    } else {
                        div().flex().flex_col().children(
                            self.trade_history
                                .iter()
                                .take(50)
                                .enumerate()
                                .map(|(i, trade)| {
                                    let bg = if i % 2 == 0 {
                                        rgb(0x252525)
                                    } else {
                                        rgb(0x2a2a2a)
                                    };
                                    div()
                                        .flex()
                                        .bg(bg)
                                        .p_2()
                                        .child(div().flex_1().child(trade.created_at.clone()))
                                        .child(div().flex_1().child(trade.symbol.clone()))
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_color(match trade.side {
                                                    Side::Buy => rgb(0x4caf50),
                                                    Side::Sell => rgb(0xf44336),
                                                })
                                                .child(format!("{:?}", trade.side)),
                                        )
                                        .child(div().flex_1().child(trade.price.to_string()))
                                        .child(div().flex_1().child(trade.qty.to_string()))
                                        .child(div().flex_1().child(format!(
                                            "{} {}",
                                            trade.fee_qty, trade.fee_asset
                                        )))
                                        .child(div().flex_1().child(trade.pnl.to_string()))
                                }),
                        )
                    }),
            )
    }

    fn render_operation_logs(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .p_4()
            .gap_2()
            .child(div().font_weight(FontWeight::BOLD).child("Operation Logs"))
            .child(
                div()
                    .w_full()
                    .border_1()
                    .border_color(rgb(0x404040))
                    .rounded_md()
                    .bg(rgb(0x1e1e1e))
                    .p_2()
                    .h_64()
                    .child(if self.operation_logs.is_empty() {
                        div().child("No logs")
                    } else {
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .children(self.operation_logs.iter().map(|log| {
                                div().text_sm().child(format!(
                                    "[{}] {}: {}",
                                    log.timestamp, log.action, log.details
                                ))
                            }))
                    }),
            )
    }
}

impl Render for TaskDetailPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(ref task) = self.task {
            div()
                .size_full()
                .flex()
                .flex_col()
                .bg(rgb(0x1e1e1e))
                .text_color(rgb(0xe5e5e5))
                .child(self.render_task_header(task))
                .child(self.render_status_section(task))
                .child(self.render_account_section())
                .child(self.render_order_history_table())
                .child(self.render_trade_history_table())
                .child(self.render_operation_logs())
        } else {
            div()
                .size_full()
                .flex()
                .justify_center()
                .items_center()
                .bg(rgb(0x1e1e1e))
                .text_color(rgb(0xe5e5e5))
                .child("Select a task to view details")
        }
    }
}

fn truncate_address(address: &str) -> String {
    if address.len() > 10 {
        format!("{}...{}", &address[0..6], &address[address.len() - 4..])
    } else {
        address.to_string()
    }
}
