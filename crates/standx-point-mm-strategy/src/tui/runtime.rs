/*
[INPUT]:  Stored tasks, TaskManager runtime snapshots, log buffer, and account/order snapshots
[OUTPUT]: Ratatui-based TUI run loop, rendering, and log buffer utilities
[POS]:    TUI runtime loop and shared helpers
[UPDATE]: When changing TUI layout, keybindings, or runtime controls
[UPDATE]: 2026-02-09 Refactor layout and palette for account/positions/orders
[UPDATE]: 2026-02-09 Extract TerminalGuard into terminal.rs and add tui module layout
[UPDATE]: 2026-02-09 Move AppState types into app.rs
[UPDATE]: 2026-02-09 Move panel renderers into ui submodules
[UPDATE]: 2026-02-09 Add tab bar and tab-specific views
[UPDATE]: 2026-02-10 Use shared draw_tabs renderer
[UPDATE]: 2026-02-10 Move runtime logic out of tui/mod.rs
[UPDATE]: 2026-02-10 Render active modal overlay in TUI draw loop
*/

use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use crossterm::event::Event as CrosstermEvent;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use rust_decimal::Decimal;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::fmt::MakeWriter;

use standx_point_adapter::{
    Chain, Credentials, Order, OrderStatus, PaginatedOrders, StandxClient, StandxError,
};
use standx_point_mm_strategy::TaskManager;
use standx_point_mm_strategy::task::TaskRuntimeStatus;

use super::app::{ActiveModal, AppState, Tab, UiSnapshot};
use super::events::handle_key_event;
use super::terminal::TerminalGuard;
use super::ui::modal::draw_modal;
use super::ui::*;
use crate::state::storage::{Account as StoredAccount, Storage};

const UI_TICK_INTERVAL: Duration = Duration::from_millis(250);
const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(200);
pub(crate) const LIVE_REFRESH_INTERVAL: Duration = Duration::from_secs(3);
pub(crate) const LOG_BUFFER_CAPACITY: usize = 2000;

pub type LogBufferHandle = Arc<StdMutex<LogBuffer>>;

#[derive(Debug, Default)]
pub struct LogBuffer {
    lines: VecDeque<String>,
    capacity: usize,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            capacity,
        }
    }

    pub fn push_line(&mut self, line: String) {
        if self.capacity == 0 {
            return;
        }
        if self.lines.len() >= self.capacity {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.lines.iter().cloned().collect()
    }
}

#[derive(Clone)]
pub struct LogWriterFactory {
    buffer: LogBufferHandle,
}

impl LogWriterFactory {
    pub fn new(buffer: LogBufferHandle) -> Self {
        Self { buffer }
    }
}

pub struct LogWriter {
    buffer: LogBufferHandle,
    partial: String,
}

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let chunk = String::from_utf8_lossy(buf);
        self.partial.push_str(&chunk);
        while let Some(pos) = self.partial.find('\n') {
            let line = self.partial[..pos].trim_end_matches('\r').to_string();
            self.partial = self.partial[pos + 1..].to_string();
            let buffer = self.buffer.clone();
            let mut guard = buffer.lock().expect("log buffer lock");
            guard.push_line(line);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.partial.is_empty() {
            let line = std::mem::take(&mut self.partial);
            let buffer = self.buffer.clone();
            let mut guard = buffer.lock().expect("log buffer lock");
            guard.push_line(line);
        }
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogWriterFactory {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogWriter {
            buffer: self.buffer.clone(),
            partial: String::new(),
        }
    }
}

enum UiEvent {
    Input(CrosstermEvent),
}

pub(super) fn draw_footer(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &AppState) {
    let key_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let line1 = Line::from(vec![
        Span::styled("[Up/Down]", key_style),
        Span::raw(" Select  "),
        Span::styled("[Tab/l]", key_style),
        Span::raw(" Switch  "),
        Span::styled("[1/2/3]", key_style),
        Span::raw(" Tabs  "),
        Span::styled("[a]", key_style),
        Span::raw(" Account  "),
        Span::styled("[t]", key_style),
        Span::raw(" Task"),
    ]);
    let line2 = Line::from(vec![
        Span::styled("[s]", key_style),
        Span::raw(" Start  "),
        Span::styled("[x]", key_style),
        Span::raw(" Stop  "),
        Span::styled("[r]", key_style),
        Span::raw(" Refresh  "),
        Span::styled("[q]", key_style),
        Span::raw(" Quit  "),
        Span::raw(format!("Status: {}", app.status_message)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style())
        .title("Hotkeys");
    let text = Text::from(vec![line1, line2]);
    let widget = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(widget, area);
}

pub(crate) fn border_style() -> Style {
    Style::default().fg(Color::Magenta)
}

pub(crate) fn header_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub(crate) fn signed_style(value: Decimal) -> Style {
    if value.is_sign_negative() {
        Style::default()
            .fg(Color::LightRed)
            .add_modifier(Modifier::BOLD)
    } else if value.is_sign_positive() {
        Style::default()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

pub(crate) fn order_side_style(order: &Order) -> Style {
    match order.side {
        standx_point_adapter::Side::Buy => Style::default().fg(Color::LightGreen),
        standx_point_adapter::Side::Sell => Style::default().fg(Color::LightRed),
    }
}

pub(crate) fn format_decimal(value: Decimal, scale: u32) -> String {
    let mut rounded = value.round_dp(scale);
    rounded.rescale(scale);
    rounded.to_string()
}

pub(crate) fn build_live_client(account: &StoredAccount) -> Result<StandxClient> {
    let mut client =
        StandxClient::new().map_err(|err| anyhow!("create StandxClient failed: {err}"))?;
    let chain = account.chain.unwrap_or(Chain::Bsc);
    let wallet_address = "unknown".to_string();
    client.set_credentials(Credentials {
        jwt_token: account.jwt_token.clone(),
        wallet_address,
        chain,
    });
    Ok(client)
}

pub(crate) async fn query_open_orders_with_fallback(
    client: &StandxClient,
    symbol: &str,
) -> Result<PaginatedOrders> {
    let open_orders = match client.query_open_orders(Some(symbol)).await {
        Ok(orders) => orders,
        Err(StandxError::Api { code: 404, message }) => {
            tracing::warn!(
                symbol = %symbol,
                "query_open_orders returned 404; treating as no open orders: {message}"
            );
            return Ok(PaginatedOrders {
                page_size: 0,
                result: Vec::new(),
                total: 0,
            });
        }
        Err(err) => return Err(anyhow!(err)).context("query_open_orders failed"),
    };

    if open_orders.total > open_orders.result.len() as u32 {
        let limit = open_orders.total;
        match client
            .query_orders(Some(symbol), Some(OrderStatus::Open), Some(limit))
            .await
        {
            Ok(expanded) => return Ok(expanded),
            Err(err) => {
                tracing::warn!(
                    symbol = %symbol,
                    total = open_orders.total,
                    page_size = open_orders.page_size,
                    "query_orders failed while expanding open orders: {err}"
                );
            }
        }
    }

    Ok(open_orders)
}

pub async fn run_tui_with_log(
    task_manager: Arc<TokioMutex<TaskManager>>,
    storage: Arc<Storage>,
    log_buffer: LogBufferHandle,
) -> Result<()> {
    let mut terminal = TerminalGuard::new()?;
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let input_shutdown = CancellationToken::new();
    let input_shutdown_clone = input_shutdown.clone();

    tokio::task::spawn_blocking(move || {
        while !input_shutdown_clone.is_cancelled() {
            if crossterm::event::poll(INPUT_POLL_INTERVAL).unwrap_or(false) {
                if let Ok(event) = crossterm::event::read() {
                    let _ = event_tx.send(UiEvent::Input(event));
                }
            }
        }
    });

    let mut app = AppState::new(storage, task_manager, log_buffer);
    app.refresh_accounts().await?;
    app.refresh_tasks().await?;

    let mut tick = tokio::time::interval(UI_TICK_INTERVAL);
    let mut should_quit = false;

    while !should_quit {
        tokio::select! {
            _ = tick.tick() => {
                if app.last_refresh.elapsed() > Duration::from_secs(2) {
                    if let Err(err) = app.refresh_tasks().await {
                        app.status_message = format!("refresh tasks failed: {err}");
                    }
                }
                if let Err(err) = app.refresh_live_data().await {
                    app.status_message = format!("refresh live data failed: {err}");
                }
            }
            maybe_event = event_rx.recv() => {
                if let Some(event) = maybe_event {
                    match event {
                        UiEvent::Input(CrosstermEvent::Key(key)) => {
                            if handle_key_event(&mut app, key.code).await {
                                should_quit = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let snapshot = app.build_snapshot().await?;
        terminal.draw(|frame| draw_ui(frame, &mut app, &snapshot))?;
    }

    input_shutdown.cancel();
    Ok(())
}

fn draw_ui(frame: &mut ratatui::Frame, app: &mut AppState, snapshot: &UiSnapshot) {
    let area = frame.size();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(3),
            Constraint::Length(4),
        ])
        .split(area);

    draw_tabs(frame, layout[1], app.current_tab);

    match app.current_tab {
        Tab::Dashboard => {
            let content = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(4), Constraint::Min(10)])
                .split(layout[0]);

            draw_account_summary(frame, content[0], app, snapshot);

            let middle = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
                .split(content[1]);
            draw_task_list(frame, middle[0], app, snapshot);

            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(middle[1]);
            draw_positions_table(frame, right[0], app);
            draw_open_orders_table(frame, right[1], app);
        }
        Tab::Logs => {
            draw_logs(frame, layout[0], &app.log_buffer);
        }
        Tab::Create => {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style())
                .title("Create");
            let account_items = if app.accounts.is_empty() {
                String::from("(none)")
            } else {
                app.accounts
                    .iter()
                    .map(|account| account.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let task_items = if app.tasks.is_empty() {
                String::from("(none)")
            } else {
                app.tasks
                    .iter()
                    .map(|task| task.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let create_text = format!(
                "Press [a] Create Account | [t] Create Task\nAccounts: {account_items}\nTasks: {task_items}"
            );
            let widget = Paragraph::new(create_text)
                .block(block)
                .alignment(Alignment::Center);
            frame.render_widget(widget, layout[0]);
        }
    }

    draw_footer(frame, layout[2], app);

    if let Some(active_modal) = app.active_modal.as_ref() {
        let modal = match active_modal {
            ActiveModal::CreateAccount(modal) => modal.to_modal(),
            ActiveModal::CreateTask(modal) => modal.to_modal(),
        };
        let modal_area = centered_rect(area, 60, 60);
        draw_modal(frame, modal_area, &modal);
    }
}

fn centered_rect(
    area: ratatui::layout::Rect,
    percent_x: u16,
    percent_y: u16,
) -> ratatui::layout::Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
}

pub(crate) fn runtime_label(status: Option<&TaskRuntimeStatus>) -> String {
    match status {
        Some(TaskRuntimeStatus::Running) => "running".to_string(),
        Some(TaskRuntimeStatus::Finished) => "finished".to_string(),
        None => "stopped".to_string(),
    }
}
