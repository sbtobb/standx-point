# Ratatui Backends

## Overview

Ratatui supports three terminal backends:
- **Crossterm** (default) - Cross-platform
- **Termion** - Unix-only
- **Termwiz** - Terminal emulator toolkit

## Backend Comparison

| Feature | Crossterm | Termion | Termwiz |
|---------|-----------|---------|---------|
| Platform | Linux/Mac/Windows | Linux/Mac | Linux/Mac/Windows |
| Default | Yes | No | No |
| Cargo feature | `crossterm` | `termion` | `termwiz` |
| Async support | Yes | No | Yes |
| Underline color | Yes | No | Yes |

## Using Crossterm (Default)

```toml
# Cargo.toml
[dependencies]
ratatui = "0.30"
crossterm = "0.29"
```

```rust
use ratatui::DefaultTerminal;

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    // ...
    ratatui::restore();
    Ok(())
}
```

### Crossterm Version Selection

```toml
# Use crossterm 0.28.x
ratatui = { version = "0.30", default-features = false, features = ["crossterm_0_28"] }

# Use crossterm 0.29.x (default)
ratatui = { version = "0.30", features = ["crossterm"] }
```

## Using Termion

```toml
# Cargo.toml
[dependencies]
ratatui = { version = "0.30", default-features = false, features = ["termion"] }
termion = "4"
```

```rust
use std::io::{self, stdout};
use ratatui::{backend::TermionBackend, Terminal};
use termion::raw::IntoRawMode;

fn main() -> io::Result<()> {
    let stdout = stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ...

    Ok(())
}
```

## Using Termwiz

```toml
# Cargo.toml
[dependencies]
ratatui = { version = "0.30", default-features = false, features = ["termwiz"] }
termwiz = "0.22"
```

```rust
use ratatui::{backend::TermwizBackend, Terminal};
use termwiz::terminal::Terminal as TermwizTerminal;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = TermwizBackend::new()?;
    let mut terminal = Terminal::new(backend)?;

    // ...

    Ok(())
}
```

## Backend Crates (v0.30.0+)

The modular workspace provides separate backend crates:

```toml
# Only crossterm backend
[dependencies]
ratatui = { version = "0.30", default-features = false }
ratatui-crossterm = "0.30"

# Only termion backend
[dependencies]
ratatui = { version = "0.30", default-features = false }
ratatui-termion = "0.30"
```

## Raw Mode and Alternate Screen

### What is Raw Mode?
- Disables line buffering
- Disables echo
- Disables special key processing (Ctrl+C, etc.)

### What is Alternate Screen?
- Switches to a separate screen buffer
- Original content preserved
- Restored when leaving alternate screen

### Manual Setup (Crossterm)

```rust
use std::io::stdout;
use crossterm::{
    execute,
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

fn setup() -> std::io::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    Ok(())
}

fn teardown() -> std::io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}
```

## Panic Handling

`ratatui::run()` and `ratatui::init()` install panic hooks automatically.

For manual setup:

```rust
use std::panic;

fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        // Restore terminal before panic message
        let _ = ratatui::restore();
        original_hook(info);
    }));
}
```

## TestBackend for Testing

```rust
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn test_render() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| {
        // render widgets
    }).unwrap();

    // Assert on buffer contents
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 0)].symbol(), "H");
}
```
