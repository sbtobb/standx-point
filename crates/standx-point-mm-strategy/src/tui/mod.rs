/*
[INPUT]:  Stored tasks, TaskManager runtime snapshots, log buffer, and account/order snapshots
[OUTPUT]: Ratatui-based TUI for account summary, positions/orders, logs, and controls
[POS]:    TUI module for standx-point-mm-strategy binary
[UPDATE]: When changing TUI layout, keybindings, or runtime controls
[UPDATE]: 2026-02-09 Refactor layout and palette for account/positions/orders
*/

use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use crossterm::event::{Event as CrosstermEvent, KeyCode};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{ExecutableCommand, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Wrap,
};
use ratatui::Terminal;
use rust_decimal::Decimal;
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::fmt::MakeWriter;

use standx_point_adapter::{
    Balance, Chain, Credentials, Order, OrderStatus, PaginatedOrders, Position, StandxClient,
    StandxError,
};
use standx_point_mm_strategy::metrics::TaskMetricsSnapshot;
use standx_point_mm_strategy::task::TaskRuntimeStatus;
use standx_point_mm_strategy::TaskManager;

use crate::cli::interactive::build_strategy_config;
use crate::state::storage::{Account as StoredAccount, Storage, Task as StoredTask};

const UI_TICK_INTERVAL: Duration = Duration::from_millis(250);
const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(200);
const LIVE_REFRESH_INTERVAL: Duration = Duration::from_secs(3);
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

struct UiSnapshot {
    runtime_status: HashMap<String, TaskRuntimeStatus>,
    metrics: HashMap<String, TaskMetricsSnapshot>,
}

#[derive(Debug)]
struct LiveTaskData {
    balance: Option<Balance>,
    positions: Vec<Position>,
    open_orders: Vec<Order>,
    last_update: Option<Instant>,
    last_error: Option<String>,
}

impl LiveTaskData {
    fn empty() -> Self {
        Self {
            balance: None,
            positions: Vec::new(),
            open_orders: Vec::new(),
            last_update: None,
            last_error: None,
        }
    }
}

struct AppState {
    storage: Arc<Storage>,
    task_manager: Arc<TokioMutex<TaskManager>>,
    log_buffer: LogBufferHandle,
    tasks: Vec<StoredTask>,
    list_state: ListState,
    status_message: String,
    last_refresh: Instant,
    last_live_refresh: Instant,
    live_data: HashMap<String, LiveTaskData>,
}

impl AppState {
    fn new(
        storage: Arc<Storage>,
        task_manager: Arc<TokioMutex<TaskManager>>,
        log_buffer: LogBufferHandle,
    ) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            storage,
            task_manager,
            log_buffer,
            tasks: Vec::new(),
            list_state,
            status_message: "Ready".to_string(),
            last_refresh: Instant::now() - Duration::from_secs(10),
            last_live_refresh: Instant::now() - LIVE_REFRESH_INTERVAL,
            live_data: HashMap::new(),
        }
    }

    fn selected_task(&self) -> Option<&StoredTask> {
        let idx = self.list_state.selected().unwrap_or(0);
        self.tasks.get(idx)
    }

    fn selected_live_data(&self) -> Option<&LiveTaskData> {
        let task = self.selected_task()?;
        self.live_data.get(&task.id)
    }

    async fn refresh_tasks(&mut self) -> Result<()> {
        let tasks = self.storage.list_tasks().await?;
        self.tasks = tasks;
        if self.tasks.is_empty() {
            self.list_state.select(None);
        } else if self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        } else if let Some(selected) = self.list_state.selected() {
            if selected >= self.tasks.len() {
                self.list_state.select(Some(self.tasks.len().saturating_sub(1)));
            }
        }
        self.last_refresh = Instant::now();
        Ok(())
    }

    async fn build_snapshot(&self) -> Result<UiSnapshot> {
        let manager = self.task_manager.lock().await;
        let runtime_status = manager.runtime_status_snapshot();
        let metrics = manager.task_metrics_snapshot().await;
        drop(manager);

        Ok(UiSnapshot {
            runtime_status,
            metrics,
        })
    }

    async fn refresh_live_data(&mut self) -> Result<()> {
        if self.last_live_refresh.elapsed() < LIVE_REFRESH_INTERVAL {
            return Ok(());
        }

        let Some(task) = self.selected_task().cloned() else {
            return Ok(());
        };

        let account = self
            .storage
            .get_account(&task.account_id)
            .await
            .ok_or_else(|| anyhow!("account not found: {}", task.account_id))?;

        let client = build_live_client(&account)?;
        let symbol = task.symbol.as_str();

        let mut data = self
            .live_data
            .remove(&task.id)
            .unwrap_or_else(LiveTaskData::empty);
        let mut errors = Vec::new();

        match client.query_balance().await {
            Ok(balance) => data.balance = Some(balance),
            Err(err) => errors.push(format!("balance: {err}")),
        }

        match client.query_positions(Some(symbol)).await {
            Ok(positions) => data.positions = positions,
            Err(err) => errors.push(format!("positions: {err}")),
        }

        match query_open_orders_with_fallback(&client, symbol).await {
            Ok(orders) => data.open_orders = orders.result,
            Err(err) => errors.push(format!("open_orders: {err}")),
        }

        data.last_update = Some(Instant::now());
        data.last_error = if errors.is_empty() {
            None
        } else {
            Some(errors.join(" | "))
        };

        self.live_data.insert(task.id.clone(), data);
        self.last_live_refresh = Instant::now();
        Ok(())
    }

    async fn start_selected_task(&mut self) -> Result<()> {
        let task = self
            .selected_task()
            .cloned()
            .ok_or_else(|| anyhow!("no task selected"))?;

        let config = build_strategy_config(&self.storage, &[task.clone()], true).await?;

        let mut manager = self.task_manager.lock().await;
        if manager.runtime_status(&task.id).is_some() {
            self.status_message = format!("task already running: {}", task.id);
            return Ok(());
        }
        manager.spawn_from_config(config).await?;
        self.status_message = format!("task started: {}", task.id);
        Ok(())
    }

    async fn stop_selected_task(&mut self) -> Result<()> {
        let task = self
            .selected_task()
            .cloned()
            .ok_or_else(|| anyhow!("no task selected"))?;

        let mut manager = self.task_manager.lock().await;
        manager.stop_task(&task.id).await?;
        self.status_message = format!("task stopped: {}", task.id);
        Ok(())
    }

    fn move_selection(&mut self, delta: isize) {
        if self.tasks.is_empty() {
            self.list_state.select(None);
            return;
        }
        let current = self.list_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, (self.tasks.len() - 1) as isize) as usize;
        self.list_state.select(Some(next));
        self.last_live_refresh = Instant::now() - LIVE_REFRESH_INTERVAL;
    }
}

fn draw_account_summary(
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
        Some(task) => format!("Account {} | Task {} | Symbol {} | {}", task.account_id, task.id, task.symbol, status),
        None => "Account Summary".to_string(),
    };

    let mut lines = Vec::new();
    if let Some(data) = app.selected_live_data() {
        if let Some(balance) = data.balance.as_ref() {
            let upnl_style = signed_style(balance.upnl);
            let equity = Span::styled(format!("Equity {}", format_decimal(balance.equity, 4)), Style::default());
            let upnl = Span::styled(format!("uPnL {}", format_decimal(balance.upnl, 4)), upnl_style);
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

        if let Some(error) = data.last_error.as_ref() {
            lines.push(Line::from(Span::styled(
                format!("Last error: {error}"),
                Style::default().fg(Color::Yellow),
            )));
        } else if let Some(updated) = data.last_update {
            lines.push(Line::from(format!("Last update: {}s", updated.elapsed().as_secs())));
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

fn draw_positions_table(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &AppState) {
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

    let table = Table::new(rows, [
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
    ])
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style())
            .title("Positions"),
    );
    frame.render_widget(table, area);
}

fn draw_open_orders_table(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &AppState) {
    let mut rows = Vec::new();
    let orders = app
        .selected_live_data()
        .map(|data| data.open_orders.as_slice())
        .unwrap_or(&[]);

    for order in orders {
        let side_style = order_side_style(order);
        let price = order.price.map(|p| format_decimal(p, 4)).unwrap_or_else(|| "-".to_string());
        rows.push(Row::new(vec![
            Cell::from(order.symbol.as_str()),
            Cell::from(Span::styled(format!("{:?}", order.side), side_style)),
            Cell::from(format!("{:?}", order.order_type)),
            Cell::from(price),
            Cell::from(format_decimal(order.qty, 4)),
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
        ]));
    }

    let header = Row::new(vec![
        Cell::from("Symbol"),
        Cell::from("Side"),
        Cell::from("Type"),
        Cell::from("Price"),
        Cell::from("Qty"),
        Cell::from("Status"),
    ])
    .style(header_style());

    let table = Table::new(rows, [
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
    ])
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style())
            .title("Open Orders"),
    );
    frame.render_widget(table, area);
}

fn draw_footer(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &AppState) {
    let key_style = Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD);
    let line1 = Line::from(vec![
        Span::styled("[Up/Down]", key_style),
        Span::raw(" Select  "),
        Span::styled("[s]", key_style),
        Span::raw(" Start  "),
        Span::styled("[x]", key_style),
        Span::raw(" Stop  "),
        Span::styled("[r]", key_style),
        Span::raw(" Refresh"),
    ]);
    let line2 = Line::from(vec![
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

fn border_style() -> Style {
    Style::default().fg(Color::Magenta)
}

fn header_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

fn signed_style(value: Decimal) -> Style {
    if value.is_sign_negative() {
        Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)
    } else if value.is_sign_positive() {
        Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

fn order_side_style(order: &Order) -> Style {
    match order.side {
        standx_point_adapter::Side::Buy => Style::default().fg(Color::LightGreen),
        standx_point_adapter::Side::Sell => Style::default().fg(Color::LightRed),
    }
}

fn format_decimal(value: Decimal, scale: u32) -> String {
    value.round_dp(scale).to_string()
}

fn build_live_client(account: &StoredAccount) -> Result<StandxClient> {
    let mut client = StandxClient::new().map_err(|err| anyhow!("create StandxClient failed: {err}"))?;
    let chain = account.chain.unwrap_or(Chain::Bsc);
    let wallet_address = "unknown".to_string();
    client.set_credentials(Credentials {
        jwt_token: account.jwt_token.clone(),
        wallet_address,
        chain,
    });
    Ok(client)
}

async fn query_open_orders_with_fallback(
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
                        UiEvent::Input(CrosstermEvent::Key(key)) => match key.code {
                            KeyCode::Char('q') => {
                                should_quit = true;
                            }
                            KeyCode::Char('r') => {
                                if let Err(err) = app.refresh_tasks().await {
                                    app.status_message = format!("refresh tasks failed: {err}");
                                }
                            }
                            KeyCode::Char('s') => {
                                if let Err(err) = app.start_selected_task().await {
                                    app.status_message = format!("start task failed: {err}");
                                }
                            }
                            KeyCode::Char('x') => {
                                if let Err(err) = app.stop_selected_task().await {
                                    app.status_message = format!("stop task failed: {err}");
                                }
                            }
                            KeyCode::Up => app.move_selection(-1),
                            KeyCode::Down => app.move_selection(1),
                            _ => {}
                        },
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
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(7),
            Constraint::Length(4),
        ])
        .split(area);

    draw_account_summary(frame, layout[0], app, snapshot);

    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(layout[1]);
    draw_task_list(frame, middle[0], app, snapshot);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(middle[1]);
    draw_positions_table(frame, right[0], app);
    draw_open_orders_table(frame, right[1], app);

    draw_logs(frame, layout[2], &app.log_buffer);
    draw_footer(frame, layout[3], app);
}

fn draw_task_list(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    app: &mut AppState,
    snapshot: &UiSnapshot,
) {
    let items = if app.tasks.is_empty() {
        vec![ListItem::new("No tasks found")]
    } else {
        app.tasks
            .iter()
            .map(|task| {
                let status = runtime_label(snapshot.runtime_status.get(&task.id));
                let metrics = snapshot.metrics.get(&task.id);
                let (orders, position) = metrics
                    .map(|m| (m.open_orders, m.position_qty.to_string()))
                    .unwrap_or((0, "-".to_string()));
                let line = format!(
                    "{} | {} | {} | ord:{} pos:{}",
                    task.id, task.symbol, status, orders, position
                );
                ListItem::new(line)
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style())
                .title("Tasks"),
        )
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_logs(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, buffer: &LogBufferHandle) {
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

fn runtime_label(status: Option<&TaskRuntimeStatus>) -> String {
    match status {
        Some(TaskRuntimeStatus::Running) => "running".to_string(),
        Some(TaskRuntimeStatus::Finished) => "finished".to_string(),
        None => "stopped".to_string(),
    }
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        stdout.execute(EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    fn draw<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.terminal.show_cursor();
        let mut stdout = io::stdout();
        let _ = stdout.execute(LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}
