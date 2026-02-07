# Ratatui Application Structure

## Overview

Ratatui applications follow a common pattern:
1. Initialize terminal
2. Run main loop (draw + handle events)
3. Restore terminal

## Initialization Methods

### Method 1: `ratatui::run()` (Recommended for simple apps)

```rust
use crossterm::event;

fn main() -> std::io::Result<()> {
    ratatui::run(|mut terminal| {
        loop {
            terminal.draw(|frame| {
                // render widgets
            })?;
            if event::read()?.is_key_press() {
                break Ok(());
            }
        }
    })
}
```

**Benefits:**
- Automatic terminal setup and teardown
- Panic hooks installed automatically
- Simplest approach for basic apps

### Method 2: `init()` / `restore()` (More control)

```rust
fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
    loop {
        terminal.draw(render)?;
        if should_quit()? {
            break Ok(());
        }
    }
}
```

**Important:** Use a separate function for the main loop to ensure `restore()` is always called.

### Method 3: Manual Backend Construction

```rust
use std::io::stdout;
use ratatui::{backend::CrosstermBackend, Terminal};

fn main() -> std::io::Result<()> {
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Manual setup
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        stdout(),
        crossterm::terminal::EnterAlternateScreen
    )?;

    let result = run(&mut terminal);

    // Manual teardown
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        stdout(),
        crossterm::terminal::LeaveAlternateScreen
    )?;

    result
}
```

## Application Patterns

### Pattern 1: Functional Style

```rust
fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let mut state = AppState::default();

    loop {
        terminal.draw(|frame| render(frame, &state))?;
        if let Some(action) = handle_events()? {
            match action {
                Action::Quit => break,
                Action::Increment => state.counter += 1,
                Action::Decrement => state.counter -= 1,
            }
        }
    }

    ratatui::restore();
    Ok(())
}
```

### Pattern 2: App Struct with Methods

```rust
struct App {
    counter: i32,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self { counter: 0, should_quit: false }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        // ...
    }

    fn handle_events(&mut self) -> std::io::Result<()> {
        // ...
        Ok(())
    }
}
```

### Pattern 3: App as Widget

```rust
impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Compose child widgets
        let layout = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
        ]);
        let [header, body] = layout.areas(area);

        Paragraph::new("Header").render(header, buf);
        self.render_body(body, buf);
    }
}

// Usage
terminal.draw(|frame| {
    frame.render_widget(&app, frame.area());
})?;
```

## Event Handling

### Basic Event Loop

```rust
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

fn handle_events() -> std::io::Result<bool> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            match key.code {
                KeyCode::Char('q') => return Ok(true),
                KeyCode::Up => { /* handle up */ }
                KeyCode::Down => { /* handle down */ }
                _ => {}
            }
        }
        Event::Resize(width, height) => {
            // Terminal resized - next draw() will use new size
        }
        _ => {}
    }
    Ok(false)
}
```

### Non-blocking Events with Timeout

```rust
use std::time::Duration;

fn handle_events() -> std::io::Result<bool> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
```

## Terminal Methods

| Method | Description |
|--------|-------------|
| `terminal.draw(f)` | Draw frame, returns `CompletedFrame` |
| `terminal.clear()` | Clear terminal screen |
| `terminal.size()` | Get terminal size as `Size` |
| `terminal.get_frame()` | Get current frame (rarely needed) |
| `terminal.insert_before(n, f)` | Insert lines before viewport |
| `terminal.set_cursor_position(pos)` | Move cursor (via Frame) |

## Frame Methods

| Method | Description |
|--------|-------------|
| `frame.area()` | Get drawable `Rect` |
| `frame.render_widget(w, area)` | Render stateless widget |
| `frame.render_stateful_widget(w, area, state)` | Render stateful widget |
| `frame.set_cursor_position(pos)` | Show cursor at position |
| `frame.buffer_mut()` | Get mutable buffer reference |
