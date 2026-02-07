# Ratatui TUI Application Patterns & Best Practices

## 1. Async TUI with Tokio Integration Patterns

### Pattern 1: Tokio Main with Async Event Loop
**File**: `/tmp/ratatui-patterns/ktop/src/main.rs`

```rust
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config = Config::load(&config_path).unwrap_or_default();
    let data_rx = spawn_sources(&config);
    let mut terminal = tui::init()?;
    let mut app = app::App::new(&config, data_rx);

    let result = app.run(&mut terminal).await;

    tui::restore()?;
    result
}
```

**Key Points**:
- Use `#[tokio::main]` attribute for async main
- Initialize terminal separately from async runtime
- Use `spawn_sources()` to launch background tasks
- Handle terminal restoration on exit

### Pattern 2: Async Event Handler with mpsc Channel
**File**: `/tmp/ratatui-patterns/ktop/src/event.rs`

```rust
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _tx: mpsc::UnboundedSender<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let _tx = tx.clone();

        tokio::spawn(async move {
            let mut reader = event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);

            loop {
                tokio::select! {
                    _ = tick_interval.tick() => {
                        let _ = tx.send(Event::Tick);
                    }
                    Some(Ok(evt)) = reader.next() => {
                        match evt {
                            CrosstermEvent::Key(key) => {
                                let _ = tx.send(Event::Key(key));
                            }
                            CrosstermEvent::Resize(w, h) => {
                                let _ = tx.send(Event::Resize(w, h));
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        Self { rx, _tx }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
```

### Pattern 3: Async Data Sources with Trait
**File**: `/tmp/ratatui-patterns/ktop/src/source/mod.rs`

```rust
#[async_trait]
pub trait DataSource: Send + 'static {
    async fn collect(&mut self) -> Result<DataSnapshot>;
    fn interval(&self) -> Duration;

    async fn run(mut self, tx: mpsc::UnboundedSender<DataSnapshot>)
    where
        Self: Sized,
    {
        let mut interval = tokio::time::interval(self.interval());
        loop {
            interval.tick().await;
            match self.collect().await {
                Ok(snapshot) => {
                    if tx.send(snapshot).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!("data source error: {e}");
                }
            }
        }
    }
}

pub fn spawn_sources(config: &Config) -> mpsc::UnboundedReceiver<DataSnapshot> {
    let (tx, rx) = mpsc::unbounded_channel();

    let sys_source = system::SystemSource::new();
    let sys_tx = tx.clone();
    tokio::spawn(async move {
        sys_source.run(sys_tx).await;
    });

    rx
}
```

## 2. State Management for TUI Apps

### Pattern 1: Centralized App State Struct
**File**: `/tmp/ratatui-patterns/ktop/src/app.rs`

```rust
pub struct App {
    running: bool,
    selected_tab: usize,
    system_panel: SystemPanel,
    git_panel: GitPanel,
    data_rx: mpsc::UnboundedReceiver<DataSnapshot>,
    events: EventHandler,
}

impl App {
    pub async fn run(&mut self, terminal: &mut Tui) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|f| {
                let layout = AppLayout::new(f.area());
                draw_tabs(f, layout.header, TAB_TITLES, self.selected_tab);

                match self.selected_tab {
                    0 => self.system_panel.draw(f, layout.content),
                    1 => self.git_panel.draw(f, layout.content),
                    _ => {}
                }

                draw_statusbar(f, layout.statusbar);
            })?;

            tokio::select! {
                Some(event) = self.events.next() => {
                    if let Some(action) = self.handle_event(event) {
                        self.dispatch(action);
                    }
                }
                Some(snapshot) = self.data_rx.recv() => {
                    self.system_panel.on_data(&snapshot);
                    self.git_panel.on_data(&snapshot);
                }
            }
        }
        Ok(())
    }
}
```

### Pattern 2: Message Passing with Actions
**File**: `/tmp/ratatui-patterns/ktop/src/action.rs` (implied)

```rust
// Action enum
enum Action {
    Quit,
    NextTab,
    PrevTab,
    Refresh,
}

// Handling events -> actions
fn handle_event(&self, event: Event) -> Option<Action> {
    match event {
        Event::Key(key) => match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Tab => Some(Action::NextTab),
            _ => None,
        },
        _ => None,
    }
}

// Dispatching actions
fn dispatch(&mut self, action: Action) {
    match action {
        Action::Quit => self.running = false,
        Action::NextTab => {
            self.selected_tab = (self.selected_tab + 1) % TAB_TITLES.len();
        }
        _ => {}
    }
}
```

### Pattern 3: Shared State with Arc<RwLock>
**File**: `/tmp/ratatui-official/ratatui/examples/apps/async-github/src/main.rs`

```rust
#[derive(Debug, Clone, Default)]
struct PullRequestListWidget {
    state: Arc<RwLock<PullRequestListState>>,
}

#[derive(Debug, Default)]
struct PullRequestListState {
    pull_requests: Vec<PullRequest>,
    loading_state: LoadingState,
    table_state: TableState,
}

impl PullRequestListWidget {
    fn run(&self) {
        let this = self.clone();
        tokio::spawn(this.fetch_pulls());
    }

    async fn fetch_pulls(self) {
        self.set_loading_state(LoadingState::Loading);
        match octocrab::instance().pulls("ratatui", "ratatui").list().send().await {
            Ok(page) => self.on_load(&page),
            Err(err) => self.on_err(&err),
        }
    }

    fn on_load(&self, page: &Page<OctoPullRequest>) {
        let prs = page.items.iter().map(Into::into);
        let mut state = self.state.write().unwrap();
        state.loading_state = LoadingState::Loaded;
        state.pull_requests.extend(prs);
    }
}
```

## 3. Form Input Handling

### Pattern 1: tui-input Library Integration
**File**: `/tmp/ratatui-patterns/tui-input/examples/ratatui_crossterm_input.rs`

```rust
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

#[derive(Debug, Default)]
struct App {
    input: Input,
    input_mode: InputMode,
    messages: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    #[default]
    Normal,
    Editing,
}

impl App {
    fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            let event = event::read()?;
            if let Event::Key(key) = event {
                match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => self.start_editing(),
                        KeyCode::Char('q') => return Ok(()),
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => self.push_message(),
                        KeyCode::Esc => self.stop_editing(),
                        _ => {
                            self.input.handle_event(&event);
                        }
                    },
                }
            }
        }
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let width = area.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);
        let style = match self.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Color::Yellow.into(),
        };
        let input = Paragraph::new(self.input.value())
            .style(style)
            .scroll((0, scroll as u16))
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, area);

        if self.input_mode == InputMode::Editing {
            let x = self.input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((area.x + x as u16, area.y + 1))
        }
    }
}
```

### Pattern 2: Custom Form Fields with Focus Management
**File**: `/tmp/ratatui-official/ratatui/examples/apps/input-form/src/main.rs`

```rust
struct InputForm {
    focus: Focus,
    first_name: StringField,
    last_name: StringField,
    age: AgeField,
}

impl InputForm {
    fn on_key_press(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Tab => self.focus = self.focus.next(),
            _ => match self.focus {
                Focus::FirstName => self.first_name.on_key_press(event),
                Focus::LastName => self.last_name.on_key_press(event),
                Focus::Age => self.age.on_key_press(event),
            },
        }
    }

    fn render(&self, frame: &mut Frame) {
        let layout = Layout::vertical(Constraint::from_lengths([1, 1, 1]));
        let [first_name_area, last_name_area, age_area] = frame.area().layout(&layout);

        frame.render_widget(&self.first_name, first_name_area);
        frame.render_widget(&self.last_name, last_name_area);
        frame.render_widget(&self.age, age_area);

        let cursor_position = match self.focus {
            Focus::FirstName => first_name_area + self.first_name.cursor_offset(),
            Focus::LastName => last_name_area + self.last_name.cursor_offset(),
            Focus::Age => age_area + self.age.cursor_offset(),
        };
        frame.set_cursor_position(cursor_position);
    }
}
```

## 4. Split-Pane Layout Patterns

### Pattern 1: Vertical Split (Header, Content, Footer)
**File**: `/tmp/ratatui-patterns/ktop/src/ui/layout.rs`

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub header: Rect,
    pub content: Rect,
    pub statusbar: Rect,
}

impl AppLayout {
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // header / tab bar
                Constraint::Min(1),    // content
                Constraint::Length(1), // status bar
            ])
            .split(area);

        Self {
            header: chunks[0],
            content: chunks[1],
            statusbar: chunks[2],
        }
    }
}
```

### Pattern 2: Horizontal Split with Multiple Panels
**File**: `/tmp/ratatui-patterns/ktop/src/ui/tabs.rs` (implied from panel module)

```rust
// In ktop/src/panel/mod.rs
pub trait Panel {
    fn draw(&self, f: &mut Frame, area: Rect);
    fn on_data(&mut self, snapshot: &DataSnapshot);
}

// In ktop/src/panel/system_panel.rs and git_panel.rs
pub struct SystemPanel {
    // panel-specific state
}

impl Panel for SystemPanel {
    fn draw(&self, f: &mut Frame, area: Rect) {
        // render system metrics
    }

    fn on_data(&mut self, snapshot: &DataSnapshot) {
        // update system panel state
    }
}
```

### Pattern 3: Complex Nested Layout
**File**: `/tmp/ratatui-official/ratatui/examples/apps/demo2/src/app.rs`

```rust
impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::vertical([
            Constraint::Length(3), // Tabs
            Constraint::Fill(1),  // Content
            Constraint::Length(1), // Status bar
        ]);
        let [tabs_area, content_area, status_area] = area.layout(&layout);

        Tabs::new(Tab::iter().map(|t| t.to_string()))
            .block(Block::bordered())
            .select(self.tab as usize)
            .render(tabs_area, buf);

        match self.tab {
            Tab::About => self.about_tab.render(content_area, buf),
            Tab::Recipe => self.recipe_tab.render(content_area, buf),
            Tab::Email => self.email_tab.render(content_area, buf),
            Tab::Traceroute => self.traceroute_tab.render(content_area, buf),
            Tab::Weather => self.weather_tab.render(content_area, buf),
        }
    }
}
```

## 5. Event Loop Patterns with Crossterm

### Pattern 1: Blocking Event Read with Timeout
**File**: `/tmp/ratatui-official/ratatui/examples/apps/demo2/src/app.rs`

```rust
fn handle_events(&mut self) -> Result<()> {
    let timeout = Duration::from_secs_f64(1.0 / 50.0);
    if !event::poll(timeout)? {
        return Ok(());
    }
    if let Some(key) = event::read()?.as_key_press_event() {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.mode = Mode::Quit,
            KeyCode::Char('h') | KeyCode::Left => self.prev_tab(),
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => self.next_tab(),
            _ => {}
        };
    }
    Ok(())
}
```

### Pattern 2: Async Event Stream
**File**: `/tmp/ratatui-official/ratatui/examples/apps/async-github/src/main.rs`

```rust
async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
    self.pull_requests.run();

    let period = Duration::from_secs_f32(1.0 / Self::FRAMES_PER_SECOND);
    let mut interval = tokio::time::interval(period);
    let mut events = EventStream::new();

    while !self.should_quit {
        tokio::select! {
            _ = interval.tick() => { terminal.draw(|frame| self.render(frame))?; },
            Some(Ok(event)) = events.next() => self.handle_event(&event),
        }
    }
    Ok(())
}
```

### Pattern 3: Combined Event and Data Handling
**File**: `/tmp/ratatui-patterns/ktop/src/app.rs`

```rust
pub async fn run(&mut self, terminal: &mut Tui) -> color_eyre::Result<()> {
    while self.running {
        terminal.draw(|f| {
            // render UI
        })?;

        tokio::select! {
            Some(event) = self.events.next() => {
                if let Some(action) = self.handle_event(event) {
                    self.dispatch(action);
                }
            }
            Some(snapshot) = self.data_rx.recv() => {
                self.system_panel.on_data(&snapshot);
                self.git_panel.on_data(&snapshot);
            }
        }
    }
    Ok(())
}
```

## Key Takeaways

### Common Project Structure
```
src/
├── main.rs          # Entry point with tokio runtime
├── app.rs           # App state and main event loop
├── event.rs         # Event handling with mpsc channel
├── tui.rs           # Terminal initialization/restore
├── ui/              # UI rendering and layout
│   ├── layout.rs    # Layout definitions
│   ├── tabs.rs      # Tab bar widget
│   └── statusbar.rs # Status bar widget
├── panel/           # Panel implementations
│   ├── mod.rs
│   ├── system_panel.rs
│   └── git_panel.rs
└── source/          # Async data sources
    ├── mod.rs
    ├── system.rs
    └── git.rs
```

### Best Practices

1. **Async Design**: Use tokio for async operations, separate data collection from rendering
2. **State Management**: Centralize state in App struct with message passing
3. **Message Passing**: Use mpsc channels for event and data communication
4. **Input Handling**: Use tui-input or tui-textarea libraries for form inputs
5. **Layout**: Predefine layout structs for consistency
6. **Panels**: Implement trait-based panels for modular rendering
7. **Error Handling**: Use color_eyre for error reporting and tracing for debugging
8. **Terminal Setup**: Separate terminal initialization from app logic


## 6. tui-textarea Integration Pattern

### Pattern 1: Text Area with Search Functionality
**File**: `/tmp/tui-textarea/examples/editor.rs`

```rust
use tui_textarea::{CursorMove, Input, Key, TextArea};

struct SearchBox<'a> {
    textarea: TextArea<'a>,
    open: bool,
}

impl Default for SearchBox<'_> {
    fn default() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_block(Block::default().borders(Borders::ALL).title("Search"));
        Self {
            textarea,
            open: false,
        }
    }
}

struct Buffer<'a> {
    textarea: TextArea<'a>,
    path: PathBuf,
    modified: bool,
}

struct Editor<'a> {
    current: usize,
    buffers: Vec<Buffer<'a>>,
    term: Terminal<CrosstermBackend<io::Stdout>>,
    message: Option<Cow<'static, str>>,
    search: SearchBox<'a>,
}

impl Editor<'_> {
    fn run(&mut self) -> io::Result<()> {
        loop {
            let search_height = self.search.height();
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(search_height),
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(f.area());

            self.term.draw(|f| {
                if search_height > 0 {
                    f.render_widget(&self.search.textarea, chunks[0]);
                }

                let buffer = &self.buffers[self.current];
                f.render_widget(&buffer.textarea, chunks[1]);

                // Render status line and message
            })?;

            if search_height > 0 {
                match crossterm::event::read()?.into() {
                    Input { key: Key::Enter, .. } => {
                        if !textarea.search_forward(true) {
                            self.message = Some("Pattern not found".into());
                        }
                        self.search.close();
                        textarea.set_search_pattern("").unwrap();
                    }
                    Input { key: Key::Esc, .. } => {
                        self.search.close();
                        textarea.set_search_pattern("").unwrap();
                    }
                    input => {
                        if let Some(query) = self.search.input(input) {
                            let maybe_err = textarea.set_search_pattern(query).err();
                            self.search.set_error(maybe_err);
                        }
                    }
                }
            } else {
                match crossterm::event::read()?.into() {
                    Input { key: Key::Char('q'), ctrl: true, .. } => break,
                    Input { key: Key::Char('g'), ctrl: true, .. } => {
                        self.search.open();
                    }
                    input => {
                        let buffer = &mut self.buffers[self.current];
                        buffer.modified = buffer.textarea.input(input);
                    }
                }
            }
        }
        Ok(())
    }
}
```

**Key Features of tui-textarea**:
1. Multi-line text editing with syntax highlighting support
2. Search and replace functionality with regex support
3. Undo/redo history management
4. Line numbering and hard tab support
5. Custom key bindings and search patterns
6. Overlay search bar integration

### Pattern 2: Minimal Text Area Usage
**File**: `/tmp/tui-textarea/examples/minimal.rs`

```rust
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use tui_textarea::{Input, Key, TextArea};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut textarea = TextArea::default();
    textarea.set_block(ratatui::widgets::Block::default().title("Editor").borders(ratatui::widgets::Borders::ALL));

    loop {
        term.draw(|f| f.render_widget(&textarea, f.area()))?;

        match crossterm::event::read()?.into() {
            Input { key: Key::Char('q'), ctrl: true, .. } => break,
            input => {
                textarea.input(input);
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
```

## 7. Additional Important Patterns

### Pattern: Terminal Initialization and Cleanup
**File**: `/tmp/ratatui-patterns/ktop/src/tui.rs`

```rust
use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> io::Result<Tui> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore() -> io::Result<()> {
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
```

### Pattern: Error Handling with color_eyre
**Common Usage Pattern**:

```rust
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    // Application code
    Ok(())
}
```

**Key Benefits**:
- Improved error reporting with stack traces
- Better user experience with color coding
- Support for custom error types

### Pattern: Logging with tracing
**File**: `/tmp/ratatui-patterns/ktop/src/main.rs`

```rust
use tracing_subscriber::fmt::init;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    // Application code
    Ok(())
}
```

**Usage in Data Sources**:
```rust
match self.collect().await {
    Ok(snapshot) => {
        if tx.send(snapshot).is_err() {
            break;
        }
    }
    Err(e) => {
        tracing::warn!("data source error: {e}");
    }
}
```


## 8. Advanced Widget Patterns

### Pattern: Custom Widget Implementation
**File**: `/tmp/ratatui-official/ratatui/examples/apps/advanced-widget-impl/src/main.rs` (implied)

Ratatui widgets follow this pattern:

```rust
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

struct MyWidget {
    // widget properties
}

impl Widget for MyWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render widget content to buffer
    }
}

impl MyWidget {
    fn new() -> Self {
        Self {
            // initial state
        }
    }
}
```

### Pattern: Stateful Widget with External State
**File**: `/tmp/ratatui-official/ratatui/examples/apps/async-github/src/main.rs`

```rust
#[derive(Debug, Clone, Default)]
struct PullRequestListWidget {
    state: Arc<RwLock<PullRequestListState>>,
}

impl Widget for &PullRequestListWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = self.state.write().unwrap();

        let block = Block::bordered()
            .title("Pull Requests")
            .title_bottom("j/k to scroll, q to quit");

        let table = Table::new(state.pull_requests.iter(), widths)
            .block(block)
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_symbol(">>")
            .row_highlight_style(Style::new().on_blue());

        StatefulWidget::render(table, area, buf, &mut state.table_state);
    }
}
```

## Summary of Best Practices

### Architecture Principles
1. **Separation of Concerns**: Split into app, ui, event, tui, and data source modules
2. **Async by Default**: Use tokio for all async operations
3. **Message Passing**: Use mpsc channels for inter-task communication
4. **Modular Design**: Implement traits for panels, widgets, and data sources
5. **State Management**: Centralize app state with clear ownership boundaries

### Performance Tips
1. **Minimize Redraws**: Only update parts of the UI that change
2. **Batch Updates**: Collect data before rendering
3. **Use Efficient Data Structures**: Prefer Vec over HashMap for list data
4. **Avoid Blocking Calls**: Always use async versions of I/O operations

### User Experience
1. **Responsive UI**: Handle resizes and render frames within 16ms for 60fps
2. **Clear Feedback**: Show loading states and error messages
3. **Consistent Key Bindings**: Follow common terminal conventions (q to quit, j/k to scroll)
4. **Visual Hierarchy**: Use borders, colors, and spacing to separate sections

### Development Tips
1. **Debug with tracing**: Use tracing to log errors and debug info
2. **Error Handling**: Use color_eyre for better error reporting
3. **Testing**: Write tests for individual widgets and data sources
4. **Documentation**: Document all public APIs and usage patterns

---
## References
- [ratatui Official Repository](https://github.com/ratatui-org/ratatui)
- [tui-input Library](https://github.com/sayanarijit/tui-input)
- [tui-textarea Library](https://github.com/rhysd/tui-textarea)
- [ktop - System Monitor TUI](https://github.com/kevinWangSheng/ktop)
- [stellar-contract-explorer](https://github.com/anataliocs/stellar-contract-explorer)
