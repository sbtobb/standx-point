# Creating Custom Widgets

## Widget Trait

The basic trait for stateless widgets:

```rust
pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer);
}
```

## Implementation Patterns

### Pattern 1: Reference-Based (Recommended)

Implement on `&Widget` for reusability:

```rust
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Widget;

struct Counter {
    value: u32,
    label: String,
}

impl Widget for &Counter {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = format!("{}: {}", self.label, self.value);
        Line::raw(text).render(area, buf);
    }
}

// Usage - widget can be reused
let counter = Counter { value: 42, label: "Count".to_string() };
frame.render_widget(&counter, area1);
frame.render_widget(&counter, area2);  // Can render again
```

### Pattern 2: Consuming Widget

Original pattern, widget is consumed:

```rust
impl Widget for Counter {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = format!("{}: {}", self.label, self.value);
        Line::raw(text).render(area, buf);
    }
}

// Usage - widget is consumed
let counter = Counter { value: 42, label: "Count".to_string() };
frame.render_widget(counter, area);
// counter is no longer available
```

### Pattern 3: Both Patterns

Support both usage styles:

```rust
impl Widget for &Counter {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Implementation
    }
}

impl Widget for Counter {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}
```

## StatefulWidget

For widgets with external state:

```rust
use ratatui::widgets::StatefulWidget;

struct ScrollableList {
    items: Vec<String>,
}

#[derive(Default)]
struct ScrollableListState {
    offset: usize,
    selected: Option<usize>,
}

impl StatefulWidget for &ScrollableList {
    type State = ScrollableListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let visible_items = self.items
            .iter()
            .skip(state.offset)
            .take(area.height as usize);

        for (i, item) in visible_items.enumerate() {
            let y = area.y + i as u16;
            let style = if Some(state.offset + i) == state.selected {
                Style::new().reversed()
            } else {
                Style::default()
            };
            buf.set_string(area.x, y, item, style);
        }
    }
}

// Usage
let list = ScrollableList { items: vec![...] };
let mut state = ScrollableListState::default();
frame.render_stateful_widget(&list, area, &mut state);
```

## Mutable Widget Pattern

For widgets that modify internal state during render:

```rust
struct AnimatedWidget {
    frame_count: u32,
}

impl Widget for &mut AnimatedWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.frame_count += 1;
        let text = format!("Frame: {}", self.frame_count);
        Line::raw(text).render(area, buf);
    }
}

// Usage
let mut widget = AnimatedWidget { frame_count: 0 };
frame.render_widget(&mut widget, area);
```

## Composing Widgets

Build complex widgets from simpler ones:

```rust
struct Panel {
    title: String,
    content: String,
}

impl Widget for &Panel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Use Block for border
        let block = Block::bordered().title(self.title.as_str());
        let inner = block.inner(area);
        block.render(area, buf);

        // Render content inside
        Paragraph::new(self.content.as_str()).render(inner, buf);
    }
}
```

## Widget with Configuration

Builder pattern for configurable widgets:

```rust
struct ProgressBar {
    progress: f64,
    style: Style,
    label: Option<String>,
}

impl ProgressBar {
    fn new(progress: f64) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            style: Style::default(),
            label: None,
        }
    }

    fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Widget for ProgressBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let filled = (area.width as f64 * self.progress) as u16;

        // Draw filled portion
        for x in area.x..area.x + filled {
            buf[(x, area.y)].set_char('█').set_style(self.style);
        }

        // Draw empty portion
        for x in area.x + filled..area.x + area.width {
            buf[(x, area.y)].set_char('░');
        }

        // Draw label
        if let Some(label) = self.label {
            let x = area.x + (area.width - label.len() as u16) / 2;
            buf.set_string(x, area.y, &label, Style::default());
        }
    }
}

// Usage
frame.render_widget(
    ProgressBar::new(0.75)
        .style(Style::new().green())
        .label("75%"),
    area
);
```

## Buffer Operations

Direct buffer manipulation:

```rust
impl Widget for &MyWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Set single cell
        buf[(area.x, area.y)]
            .set_char('X')
            .set_style(Style::new().red());

        // Set string
        buf.set_string(area.x, area.y, "Hello", Style::default());

        // Set styled spans
        buf.set_line(area.x, area.y, &Line::from(vec![
            Span::raw("Hello "),
            Span::styled("World", Style::new().bold()),
        ]), area.width);

        // Fill area
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_char('.');
            }
        }
    }
}
```

## Best Practices

1. **Implement on references** - Use `impl Widget for &MyWidget` for reusability
2. **Compose existing widgets** - Build on Block, Paragraph, etc.
3. **Use builder pattern** - For configurable widgets
4. **Handle area bounds** - Widgets may receive empty areas
5. **Respect area limits** - Don't draw outside the given area
