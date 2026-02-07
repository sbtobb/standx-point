# Ratatui Built-in Widgets

## Block

Container widget for borders, titles, and padding.

```rust
use ratatui::widgets::{Block, Borders, Padding};

// Simple bordered block
let block = Block::bordered().title("Title");

// Detailed configuration
let block = Block::new()
    .borders(Borders::ALL)
    .border_style(Style::new().blue())
    .title("Top Title")
    .title_bottom("Bottom")
    .title_alignment(Alignment::Center)
    .padding(Padding::horizontal(1));

// Use as container
let paragraph = Paragraph::new("Content").block(block);
```

## Paragraph

Display styled and wrapped text.

```rust
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::text::Text;

// Simple text
let para = Paragraph::new("Hello World!");

// Styled text
let para = Paragraph::new("Text".red().bold());

// Multi-line
let para = Paragraph::new(vec![
    Line::from("Line 1"),
    Line::from("Line 2"),
]);

// With wrapping
let para = Paragraph::new(long_text)
    .wrap(Wrap { trim: true });

// With scrolling
let para = Paragraph::new(content)
    .scroll((offset_y, offset_x));
```

## List

Display selectable items.

```rust
use ratatui::widgets::{List, ListItem, ListState, ListDirection};

// Create items
let items: Vec<ListItem> = data
    .iter()
    .map(|s| ListItem::new(s.as_str()))
    .collect();

// Configure list
let list = List::new(items)
    .block(Block::bordered().title("List"))
    .highlight_style(Style::new().reversed())
    .highlight_symbol("> ")
    .repeat_highlight_symbol(true)
    .direction(ListDirection::TopToBottom);

// State management
let mut state = ListState::default();
state.select(Some(0));

// Navigation
fn next(&mut self, len: usize) {
    let i = match self.state.selected() {
        Some(i) => (i + 1) % len,
        None => 0,
    };
    self.state.select(Some(i));
}

fn previous(&mut self, len: usize) {
    let i = match self.state.selected() {
        Some(i) => (i + len - 1) % len,
        None => 0,
    };
    self.state.select(Some(i));
}
```

## Table

Multi-column data grid.

```rust
use ratatui::widgets::{Table, Row, Cell, TableState};
use ratatui::layout::Constraint;

// Create rows
let rows = vec![
    Row::new(vec![
        Cell::from("Alice"),
        Cell::from("alice@example.com"),
        Cell::from("Admin").green(),
    ]),
    Row::new(vec![
        Cell::from("Bob"),
        Cell::from("bob@example.com"),
        Cell::from("User"),
    ]),
];

// Configure table
let widths = [
    Constraint::Length(15),
    Constraint::Fill(1),
    Constraint::Length(10),
];

let table = Table::new(rows, widths)
    .block(Block::bordered().title("Users"))
    .header(
        Row::new(vec!["Name", "Email", "Role"])
            .style(Style::new().bold())
            .bottom_margin(1)
    )
    .highlight_style(Style::new().reversed())
    .highlight_symbol("> ");

// State
let mut state = TableState::default();
state.select(Some(0));
```

## Tabs

Horizontal tab bar.

```rust
use ratatui::widgets::Tabs;

let titles = vec!["Home", "Settings", "Help"];
let selected = 0;

let tabs = Tabs::new(titles)
    .block(Block::bordered())
    .select(selected)
    .style(Style::new().white())
    .highlight_style(Style::new().yellow().bold())
    .divider(" | ");
```

## Gauge

Progress bar.

```rust
use ratatui::widgets::Gauge;

// Percentage-based
let gauge = Gauge::default()
    .block(Block::bordered().title("Download"))
    .gauge_style(Style::new().fg(Color::Green).bg(Color::Black))
    .percent(75)
    .label("75%");

// Ratio-based (0.0 to 1.0)
let gauge = Gauge::default()
    .ratio(0.75)
    .label(format!("{:.1}%", 0.75 * 100.0));
```

## LineGauge

Thin progress line.

```rust
use ratatui::widgets::LineGauge;
use ratatui::symbols::line;

let gauge = LineGauge::default()
    .block(Block::bordered().title("Progress"))
    .ratio(0.5)
    .line_set(line::THICK)
    .filled_style(Style::new().green())
    .unfilled_style(Style::new().dark_gray());
```

## Scrollbar

Scroll indicator.

```rust
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
    .begin_symbol(Some("↑"))
    .end_symbol(Some("↓"))
    .thumb_symbol("█")
    .track_symbol(Some("│"));

let mut state = ScrollbarState::new(total_items)
    .position(current_position);

frame.render_stateful_widget(scrollbar, area, &mut state);
```

## Sparkline

Compact data visualization.

```rust
use ratatui::widgets::Sparkline;

let data = vec![0, 1, 2, 3, 4, 5, 4, 3, 2, 1, 0];

let sparkline = Sparkline::default()
    .block(Block::bordered().title("Sparkline"))
    .data(&data)
    .max(10)
    .style(Style::new().green());
```

## Chart

Line and scatter charts.

```rust
use ratatui::widgets::{Axis, Chart, Dataset, GraphType};
use ratatui::symbols::Marker;

let data = vec![(0.0, 1.0), (1.0, 3.0), (2.0, 2.0), (3.0, 4.0)];

let dataset = Dataset::default()
    .name("Data")
    .marker(Marker::Braille)
    .graph_type(GraphType::Line)
    .style(Style::new().green())
    .data(&data);

let chart = Chart::new(vec![dataset])
    .block(Block::bordered().title("Chart"))
    .x_axis(
        Axis::default()
            .title("X")
            .bounds([0.0, 4.0])
            .labels(vec!["0", "2", "4"])
    )
    .y_axis(
        Axis::default()
            .title("Y")
            .bounds([0.0, 5.0])
            .labels(vec!["0", "2.5", "5"])
    );
```

## BarChart

Bar chart visualization.

```rust
use ratatui::widgets::{Bar, BarChart, BarGroup};

let data = vec![
    ("Mon", 10),
    ("Tue", 20),
    ("Wed", 15),
    ("Thu", 25),
    ("Fri", 30),
];

let bars: Vec<Bar> = data
    .iter()
    .map(|(label, value)| Bar::default().value(*value).label((*label).into()))
    .collect();

let chart = BarChart::default()
    .block(Block::bordered().title("Weekly"))
    .data(BarGroup::default().bars(&bars))
    .bar_width(5)
    .bar_gap(2)
    .bar_style(Style::new().green())
    .value_style(Style::new().bold());
```

## Canvas

Drawing shapes.

```rust
use ratatui::widgets::canvas::{Canvas, Circle, Line, Rectangle};

let canvas = Canvas::default()
    .block(Block::bordered().title("Canvas"))
    .x_bounds([0.0, 100.0])
    .y_bounds([0.0, 100.0])
    .paint(|ctx| {
        ctx.draw(&Rectangle {
            x: 10.0,
            y: 10.0,
            width: 30.0,
            height: 20.0,
            color: Color::Green,
        });
        ctx.draw(&Circle {
            x: 50.0,
            y: 50.0,
            radius: 20.0,
            color: Color::Yellow,
        });
        ctx.draw(&Line {
            x1: 0.0,
            y1: 0.0,
            x2: 100.0,
            y2: 100.0,
            color: Color::Red,
        });
    })
    .marker(Marker::Braille);
```

## Clear

Clear area for overlays.

```rust
use ratatui::widgets::Clear;

// Clear before rendering popup
frame.render_widget(Clear, popup_area);
frame.render_widget(popup_content, popup_area);
```

## Calendar (feature: widget-calendar)

Monthly calendar display.

```rust
use ratatui::widgets::calendar::{CalendarEventStore, Monthly};
use time::Date;

let events = CalendarEventStore::today(Style::new().red().bold());

let calendar = Monthly::new(
    Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
    events,
)
.block(Block::bordered().title("January 2024"))
.show_weekdays_header(Style::new().bold())
.show_month_header(Style::new().bold());
```
