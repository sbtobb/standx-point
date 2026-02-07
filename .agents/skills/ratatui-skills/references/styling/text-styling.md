# Ratatui Text Styling

## Text Hierarchy

```
Text
 ├── Line 1
 │    ├── Span "Hello "
 │    └── Span "World" (styled)
 └── Line 2
      └── Span "More text"
```

## Span

A segment of text with a single style.

```rust
use ratatui::text::Span;
use ratatui::style::{Style, Stylize};

// Raw (unstyled)
let span = Span::raw("Hello");

// Styled with Style struct
let span = Span::styled("World", Style::new().bold());

// Using Stylize trait (string -> Span)
let span = "Colored".red().bold();

// From conversion
let span: Span = "Text".into();
```

### Span Methods

```rust
// Get content
span.content  // Cow<'a, str>

// Get style
span.style    // Style

// Styled methods return new Span
span.style(Style::new().red())
span.fg(Color::Blue)
span.bg(Color::White)

// Reset style
span.reset_style()
```

## Line

A single line of styled text (collection of Spans).

```rust
use ratatui::text::Line;

// From string
let line = Line::raw("Simple line");

// From styled string
let line = Line::styled("Styled line", Style::new().blue());

// From spans
let line = Line::from(vec![
    Span::raw("Normal "),
    Span::styled("bold", Style::new().bold()),
    " text".into(),
]);

// Using shorthand
let line = Line::from(vec![
    "Hello ".into(),
    "world".red(),
    "!".into(),
]);

// From string with Stylize
let line: Line = "Full line".yellow().into();
```

### Line Methods

```rust
// Alignment
line.alignment(Alignment::Center)
line.left_aligned()
line.centered()
line.right_aligned()

// Style the whole line
line.style(Style::new().italic())

// Get spans
line.spans  // Vec<Span>

// Get width
line.width()  // usize
```

## Text

Multiple lines of styled text.

```rust
use ratatui::text::Text;

// From string (splits on newlines)
let text = Text::raw("Line 1\nLine 2\nLine 3");

// From lines
let text = Text::from(vec![
    Line::from("First line".blue()),
    Line::from("Second line".green()),
    Line::from(vec!["Mixed ".into(), "styles".red()]),
]);

// Styled text
let text = Text::styled("All italic", Style::new().italic());

// From iterator
let text: Text = ["Line 1", "Line 2", "Line 3"]
    .iter()
    .map(|s| Line::from(*s))
    .collect();
```

### Text Methods

```rust
// Get lines
text.lines  // Vec<Line>

// Get dimensions
text.width()   // usize (max line width)
text.height()  // usize (number of lines)

// Style all lines
text.style(Style::new().bold())

// Alignment
text.alignment(Alignment::Center)

// Add line
text.push_line(Line::from("New line"))

// Extend with lines
text.extend(other_lines)
```

## Stylize Trait

Shorthand methods available on strings and styled types:

```rust
use ratatui::style::Stylize;

// Color shortcuts
"text".black()
"text".red()
"text".green()
"text".yellow()
"text".blue()
"text".magenta()
"text".cyan()
"text".gray()
"text".white()

// Light variants
"text".light_red()
"text".light_green()
// ...

// Background (on_*)
"text".on_black()
"text".on_red()
"text".on_blue()
// ...

// Modifiers
"text".bold()
"text".dim()
"text".italic()
"text".underlined()
"text".slow_blink()
"text".rapid_blink()
"text".reversed()
"text".hidden()
"text".crossed_out()

// Reset
"text".reset()

// Not (remove modifier)
"text".not_bold()
"text".not_italic()
// ...
```

## Style Struct

For storing and reusing styles:

```rust
use ratatui::style::{Color, Modifier, Style};

// Create empty style
let style = Style::default();
let style = Style::new();

// Set foreground
let style = Style::new().fg(Color::Red);

// Set background
let style = Style::new().bg(Color::Blue);

// Add modifiers
let style = Style::new()
    .add_modifier(Modifier::BOLD)
    .add_modifier(Modifier::ITALIC);

// Combine modifiers
let style = Style::new()
    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

// Remove modifier
let style = style.remove_modifier(Modifier::BOLD);

// Set underline color
let style = Style::new()
    .underlined()
    .underline_color(Color::Red);

// Patch (merge styles)
let base = Style::new().fg(Color::White);
let highlight = base.patch(Style::new().bold());
```

## Common Patterns

### Highlighted Selection
```rust
let normal = Style::default();
let selected = Style::new().reversed();

let items: Vec<Line> = data.iter().enumerate()
    .map(|(i, item)| {
        let style = if i == selected_idx { selected } else { normal };
        Line::styled(item, style)
    })
    .collect();
```

### Status Indicators
```rust
fn status_style(status: Status) -> Style {
    match status {
        Status::Success => Style::new().green(),
        Status::Warning => Style::new().yellow(),
        Status::Error => Style::new().red().bold(),
        Status::Info => Style::new().blue(),
    }
}
```

### Syntax Highlighting
```rust
fn highlight_line(line: &str) -> Line {
    let mut spans = Vec::new();
    // Parse and create styled spans
    // ...
    Line::from(spans)
}
```

### Theme Structure
```rust
struct Theme {
    primary: Style,
    secondary: Style,
    accent: Style,
    error: Style,
    warning: Style,
    success: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: Style::new().fg(Color::White),
            secondary: Style::new().fg(Color::Gray),
            accent: Style::new().fg(Color::Cyan).bold(),
            error: Style::new().fg(Color::Red),
            warning: Style::new().fg(Color::Yellow),
            success: Style::new().fg(Color::Green),
        }
    }
}
```
