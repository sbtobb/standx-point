use gpui::*;
use standx_point_adapter::types::SymbolPrice;
use std::collections::HashMap;

pub struct StatusBar {
    prices: HashMap<String, SymbolPrice>,
    task_stats: TaskStats,
    connection_status: ConnectionStatus,
}

#[derive(Clone, Copy, Default)]
pub struct TaskStats {
    pub running: usize,
    pub paused: usize,
    pub stopped: usize,
    pub failed: usize,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Reconnecting,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            prices: HashMap::new(),
            task_stats: TaskStats::default(),
            connection_status: ConnectionStatus::Disconnected,
        }
    }

    pub fn update_prices(&mut self, prices: HashMap<String, SymbolPrice>) {
        self.prices = prices;
    }

    pub fn update_task_stats(&mut self, stats: TaskStats) {
        self.task_stats = stats;
    }

    pub fn update_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h(px(32.))
            .bg(rgb(0x2d2d2d))
            .text_color(white())
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px_4()
            .text_sm()
            .child(self.render_connection_status())
            .child(self.render_prices())
            .child(self.render_task_stats())
    }
}

impl StatusBar {
    fn render_connection_status(&self) -> impl IntoElement {
        let (color, text) = match self.connection_status {
            ConnectionStatus::Connected => (rgb(0x4caf50), "Connected"),
            ConnectionStatus::Disconnected => (rgb(0xf44336), "Disconnected"),
            ConnectionStatus::Reconnecting => (rgb(0xffc107), "Reconnecting"),
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(color))
            .child(text)
    }

    fn render_prices(&self) -> impl IntoElement {
        let mut prices: Vec<_> = self.prices.values().collect();
        prices.sort_by_key(|p| &p.symbol);

        div().flex().flex_row().gap_4().children(
            prices
                .into_iter()
                .take(5)
                .map(|p| format!("{}: ${:.2}", p.symbol, p.mark_price)),
        )
    }

    fn render_task_stats(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .gap_4()
            .child(format!("Running: {}", self.task_stats.running))
            .child(div().text_color(rgb(0x666666)).child("|"))
            .child(format!("Paused: {}", self.task_stats.paused))
            .child(div().text_color(rgb(0x666666)).child("|"))
            .child(format!("Stopped: {}", self.task_stats.stopped))
            .child(div().text_color(rgb(0x666666)).child("|"))
            .child(format!("Failed: {}", self.task_stats.failed))
    }
}
