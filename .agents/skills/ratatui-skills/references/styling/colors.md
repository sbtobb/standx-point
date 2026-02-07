# Ratatui Colors

## Color Enum

```rust
pub enum Color {
    Reset,              // Terminal default
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,               // Same as LightBlack
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    LightGray,          // Same as White
    White,
    Rgb(u8, u8, u8),    // True color
    Indexed(u8),        // 256 color palette
}
```

## Basic Colors

```rust
use ratatui::style::Color;

// Standard 8 colors
Color::Black
Color::Red
Color::Green
Color::Yellow
Color::Blue
Color::Magenta
Color::Cyan
Color::White

// Light/Bright variants
Color::LightBlack   // Gray
Color::LightRed
Color::LightGreen
Color::LightYellow
Color::LightBlue
Color::LightMagenta
Color::LightCyan
Color::LightWhite   // Bright white
```

## RGB Colors

True color support (most modern terminals):

```rust
// RGB values 0-255
Color::Rgb(255, 0, 0)      // Pure red
Color::Rgb(0, 255, 0)      // Pure green
Color::Rgb(0, 0, 255)      // Pure blue
Color::Rgb(255, 128, 0)    // Orange
Color::Rgb(128, 0, 128)    // Purple

// From hex
fn hex_to_rgb(hex: u32) -> Color {
    Color::Rgb(
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    )
}

let coral = hex_to_rgb(0xFF7F50);
```

## Indexed Colors (256 Palette)

```rust
// 0-7: Standard colors
Color::Indexed(0)  // Black
Color::Indexed(1)  // Red
Color::Indexed(2)  // Green
// ...

// 8-15: Bright colors
Color::Indexed(8)  // Bright Black (Gray)
Color::Indexed(9)  // Bright Red
// ...

// 16-231: 6x6x6 color cube
// Formula: 16 + 36*r + 6*g + b (r,g,b: 0-5)
Color::Indexed(196) // Bright red
Color::Indexed(46)  // Bright green
Color::Indexed(21)  // Bright blue

// 232-255: Grayscale (24 shades)
Color::Indexed(232) // Near black
Color::Indexed(243) // Mid gray
Color::Indexed(255) // Near white
```

## Using Colors with Style

```rust
use ratatui::style::{Color, Style};

// Foreground color
let style = Style::new().fg(Color::Red);

// Background color
let style = Style::new().bg(Color::Blue);

// Both
let style = Style::new()
    .fg(Color::White)
    .bg(Color::DarkGray);

// Underline color (requires feature)
let style = Style::new()
    .underlined()
    .underline_color(Color::Red);
```

## Using Stylize Shorthand

```rust
use ratatui::style::Stylize;

// Foreground colors
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

// Background colors (on_*)
"text".on_black()
"text".on_red()
"text".on_blue()
// ...
"text".on_light_blue()
```

## Color Conversion (palette feature)

With `palette` feature enabled:

```rust
use palette::{Srgb, Hsv};
use ratatui::style::Color;

// From palette Srgb
let color: Color = Srgb::new(1.0, 0.5, 0.0).into();

// From palette Hsv
let color: Color = Hsv::new(120.0, 1.0, 1.0).into();
```

## Serialization (serde feature)

With `serde` feature enabled:

```rust
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Theme {
    primary: Color,
    secondary: Color,
}

// JSON format
// { "primary": "Red", "secondary": { "Rgb": [255, 128, 0] } }
```

## Terminal Compatibility

| Color Type | Support |
|------------|---------|
| 8 basic colors | All terminals |
| 16 colors (+ light) | Most terminals |
| 256 indexed | Modern terminals |
| RGB true color | Modern terminals |

Check terminal support:
```bash
# Check TERM variable
echo $TERM

# Check color support
echo $COLORTERM
```

## Common Color Palettes

### Monokai-inspired
```rust
let background = Color::Rgb(39, 40, 34);
let foreground = Color::Rgb(248, 248, 242);
let comment = Color::Rgb(117, 113, 94);
let red = Color::Rgb(249, 38, 114);
let green = Color::Rgb(166, 226, 46);
let yellow = Color::Rgb(230, 219, 116);
let blue = Color::Rgb(102, 217, 239);
let purple = Color::Rgb(174, 129, 255);
```

### Nord-inspired
```rust
let polar_night = Color::Rgb(46, 52, 64);
let snow_storm = Color::Rgb(236, 239, 244);
let frost_blue = Color::Rgb(136, 192, 208);
let aurora_red = Color::Rgb(191, 97, 106);
let aurora_green = Color::Rgb(163, 190, 140);
```
