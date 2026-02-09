/*
[INPUT]:  Stored tasks, TaskManager runtime snapshots, and log buffer
[OUTPUT]: Ratatui-based TUI for task status, logs, and controls
[POS]:    TUI module for standx-point-mm-strategy binary
[UPDATE]: When changing TUI layout, keybindings, or runtime controls
*/

use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use crossterm::event::{Event as CrosstermEvent, KeyCode};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{ExecutableCommand, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table};
use ratatui::Terminal;
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::fmt::MakeWriter;

use standx_point_mm_strategy::metrics::TaskMetricsSnapshot;
use standx_point_mm_strategy::task::TaskRuntimeStatus;
use standx_point_mm_strategy::TaskManager;

use crate::cli::interactive::build_strategy_config;
use crate::state::storage::{Storage, Task as StoredTask};

const UI_TICK_INTERVAL: Duration = Duration::from_millis(250);
const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(200);
pub(crate) const LOG_BUFFER_CAPACITY: usize = 2000;
const HEARTBEAT_STALE_AFTER: Duration = Duration::from_secs(20);

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

struct StatusRow {
    task_id: String,
    symbol: String,
    price: String,
    open_orders: String,
    position_qty: String,
    heartbeat: String,
    state: String,
    risk: String,
}

struct UiSnapshot {
    status_rows: Vec<StatusRow>,
    runtime_status: HashMap<String, TaskRuntimeStatus>,
}

struct AppState {
    storage: Arc<Storage>,
    task_manager: Arc<TokioMutex<TaskManager>>,
    log_buffer: LogBufferHandle,
    tasks: Vec<StoredTask>,
    list_state: ListState,
    status_message: String,
    last_refresh: Instant,
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
        }
    }

    fn selected_task(&self) -> Option<&StoredTask> {
        let idx = self.list_state.selected().unwrap_or(0);
        self.tasks.get(idx)
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
        let configs = manager.task_config_snapshot();
        let metrics = manager.task_metrics_snapshot().await;
        let hub = manager.market_data_hub();
        drop(manager);

        let hub = hub.lock().await;

        let mut rows = Vec::new();
        for (task_id, config) in configs {
            let price = hub
                .get_price(&config.symbol)
                .map(|price| price.mark_price)
                .unwrap_or_default();
            let metrics = metrics.get(&task_id).cloned().unwrap_or_else(|| TaskMetricsSnapshot {
                open_orders: 0,
                position_qty: rust_decimal::Decimal::ZERO,
                last_heartbeat: None,
                last_price: None,
                last_update: None,
            });

            let heartbeat = heartbeat_label(metrics.last_heartbeat);
            let state = runtime_label(runtime_status.get(&task_id));
            let price_label = if price.is_zero() {
                "-".to_string()
            } else {
                price.to_string()
            };

            rows.push(StatusRow {
                task_id: task_id.clone(),
                symbol: config.symbol.clone(),
                price: price_label,
                open_orders: metrics.open_orders.to_string(),
                position_qty: metrics.position_qty.to_string(),
                heartbeat,
                state,
                risk: config.risk.level.clone(),
            });
        }

        Ok(UiSnapshot {
            status_rows: rows,
            runtime_status,
        })
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
    }
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
    let status_height = status_height_for(area.height, snapshot.status_rows.len());
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(status_height),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

    draw_status_table(frame, layout[0], &snapshot.status_rows);

    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(layout[1]);
    draw_task_list(frame, middle[0], app, snapshot);
    draw_logs(frame, middle[1], &app.log_buffer);

    let footer = Paragraph::new(format!(
        "Hotkeys: [Up/Down] Select  [s] Start  [x] Stop  [r] Refresh  [q] Quit  |  Status: {}",
        app.status_message
    ))
    .block(Block::default().borders(Borders::ALL).title("Hotkeys"));
    frame.render_widget(footer, layout[2]);
}

fn status_height_for(total_height: u16, rows: usize) -> u16 {
    let min_height = 3;
    let desired = (rows + 2) as u16;
    let max_height = total_height.saturating_sub(4).max(min_height);
    desired.clamp(min_height, max_height)
}

fn draw_status_table(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, rows: &[StatusRow]) {
    let header = Row::new(vec![
        Cell::from("Task"),
        Cell::from("Symbol"),
        Cell::from("Price"),
        Cell::from("Orders"),
        Cell::from("Position"),
        Cell::from("Heartbeat"),
        Cell::from("State"),
        Cell::from("Risk"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    let mut table_rows = Vec::new();
    for row in rows {
        table_rows.push(Row::new(vec![
            Cell::from(row.task_id.as_str()),
            Cell::from(row.symbol.as_str()),
            Cell::from(row.price.as_str()),
            Cell::from(row.open_orders.as_str()),
            Cell::from(row.position_qty.as_str()),
            Cell::from(row.heartbeat.as_str()),
            Cell::from(row.state.as_str()),
            Cell::from(row.risk.as_str()),
        ]));
    }

    let table = Table::new(table_rows, [
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(8),
    ])
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(table, area);
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
                let line = format!("{} | {} | {}", task.id, task.symbol, status);
                ListItem::new(line)
            })
            .collect()
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Tasks"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
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
    let log_widget = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Logs"));
    frame.render_widget(log_widget, area);
}

fn heartbeat_label(last_heartbeat: Option<Instant>) -> String {
    match last_heartbeat {
        Some(instant) => {
            let age = instant.elapsed();
            if age > HEARTBEAT_STALE_AFTER {
                format!("stale {}s", age.as_secs())
            } else {
                format!("ok {}s", age.as_secs())
            }
        }
        None => "-".to_string(),
    }
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
