---
name: ratatui
description: |
  CRITICAL: Use for ratatui TUI library questions. Triggers on:
  ratatui, TUI, terminal ui, ratatui::run, ratatui::init, ratatui::restore,
  DefaultTerminal, Frame, terminal.draw, crossterm, termion, termwiz,
  Layout, Constraint, Rect, Flex, Direction, horizontal, vertical,
  Block, Paragraph, List, Table, Tabs, Chart, Gauge, Scrollbar, Canvas,
  Widget, StatefulWidget, ListState, TableState, ListItem, Row, Cell,
  Style, Color, Modifier, Stylize, Span, Line, Text, bold, italic,
  "how to start ratatui", "ratatui hello world", "ratatui app structure",
  ratatui 入门, 终端界面, ratatui 教程, TUI 应用, 布局, 组件, 样式
---

# Ratatui TUI Library

> **Version:** ratatui 0.30.0 | **Last Updated:** 2026-01-19
>
> Check for updates: https://crates.io/crates/ratatui

You are an expert at the Rust `ratatui` crate. Help users by:
- **Writing code**: Generate Rust code following the patterns below
- **Answering questions**: Explain concepts, troubleshoot issues, reference documentation

## Code Generation Rules

**IMPORTANT: Before generating any Rust code, read `./references/_shared/rust-defaults.md` for shared rules.**

Key rules:
- Use `edition = "2024"` in Cargo.toml (NOT 2021)
- Use latest ratatui version: `ratatui = "0.30"`
- Use crossterm backend by default (cross-platform)

## Module Navigation

This skill is organized into focused sub-modules. For detailed information, refer to:

| Module | File | Topics |
|--------|------|--------|
| **Basics** | `./skills/basics/SKILL.md` | Terminal init, app structure, event loop |
| **Layout** | `./skills/layout/SKILL.md` | Constraint, Rect, Flex, split areas |
| **Widgets** | `./skills/widgets/SKILL.md` | Block, List, Table, Gauge, custom widgets |
| **Styling** | `./skills/styling/SKILL.md` | Color, Style, Modifier, Text/Span/Line |

## Key Concepts

Ratatui uses **immediate rendering with intermediate buffers**:
- Each frame, render all widgets to a buffer
- Terminal compares current/previous buffers
- Only changed cells are written to terminal

## Quick Reference

### Simplest App
```rust
use crossterm::event;

fn main() -> std::io::Result<()> {
    ratatui::run(|mut terminal| {
        loop {
            terminal.draw(|frame| {
                frame.render_widget("Hello World!", frame.area());
            })?;
            if event::read()?.is_key_press() {
                break Ok(());
            }
        }
    })
}
```

### App with Layout
```rust
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Block, Paragraph};

fn render(frame: &mut Frame) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(1),
    ]).areas(frame.area());

    frame.render_widget(
        Paragraph::new("Header").block(Block::bordered()),
        header,
    );
    frame.render_widget(
        Paragraph::new("Body content"),
        body,
    );
    frame.render_widget(
        Paragraph::new("Footer"),
        footer,
    );
}
```

### Styled Text
```rust
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};

let line = Line::from(vec![
    "Normal ".into(),
    "bold".bold(),
    " and ".into(),
    "red".red(),
]);
```

### List with Selection
```rust
use ratatui::widgets::{Block, List, ListItem, ListState};
use ratatui::style::Stylize;

let items: Vec<ListItem> = vec![
    ListItem::new("Item 1"),
    ListItem::new("Item 2"),
];

let list = List::new(items)
    .block(Block::bordered().title("List"))
    .highlight_style(Style::new().reversed())
    .highlight_symbol("> ");

let mut state = ListState::default();
state.select(Some(0));

frame.render_stateful_widget(list, area, &mut state);
```

## API Reference Table

| Function/Type | Description | Example |
|---------------|-------------|---------|
| `ratatui::run(f)` | Run app with auto init/restore | `ratatui::run(\|t\| { ... })` |
| `ratatui::init()` | Initialize terminal | `let mut term = ratatui::init();` |
| `ratatui::restore()` | Restore terminal state | `ratatui::restore();` |
| `terminal.draw(f)` | Draw a frame | `terminal.draw(\|frame\| { ... })?;` |
| `Layout::vertical([...])` | Create vertical layout | `Layout::vertical([Length(3), Fill(1)])` |
| `Layout::horizontal([...])` | Create horizontal layout | `Layout::horizontal([Percentage(50); 2])` |
| `frame.render_widget(w, a)` | Render widget | `frame.render_widget(para, area);` |
| `frame.render_stateful_widget(w, a, s)` | Render with state | `frame.render_stateful_widget(list, area, &mut state);` |

## Constraint Types

| Constraint | Description |
|------------|-------------|
| `Length(n)` | Exactly n cells |
| `Min(n)` | At least n cells |
| `Max(n)` | At most n cells |
| `Percentage(n)` | n% of available |
| `Ratio(a, b)` | a/b of available |
| `Fill(n)` | Fill with weight n |

## Built-in Widgets

| Widget | State Type | Description |
|--------|------------|-------------|
| `Block` | - | Container with borders/title |
| `Paragraph` | - | Text display with wrapping |
| `List` | `ListState` | Selectable list items |
| `Table` | `TableState` | Rows and columns |
| `Tabs` | - | Tab bar |
| `Gauge` | - | Progress bar |
| `Scrollbar` | `ScrollbarState` | Scroll indicator |
| `Chart` | - | Line/scatter charts |
| `BarChart` | - | Bar charts |
| `Canvas` | - | Custom drawing |

## When Writing Code

1. Use `ratatui::run()` for simple apps - handles init/restore automatically
2. Use `Layout::vertical/horizontal()` with `areas()` for compile-time known layouts
3. Wrap content widgets with `Block` for borders and titles
4. Handle `KeyEventKind::Press` to avoid duplicate key events on Windows
5. Use `crossterm` backend by default (works on all platforms)
6. Implement `Widget for &MyWidget` for reusable custom widgets

## When Answering Questions

1. Ratatui is immediate mode - rebuild UI every frame
2. Widgets are consumed when rendered (implement on `&Widget` for reuse)
3. Layout uses Cassowary constraint solver algorithm
4. Event handling is separate from ratatui - use crossterm/termion directly
5. Stateful widgets require external state management
