# Ratatui Flex Modes

## Overview

`Flex` controls how extra space is distributed when constraints are satisfied.

## Flex Modes

### `Flex::Start` (Default)
Align content to start, excess space at end.

```
[Item1][Item2][Item3]
```

### `Flex::End`
Align content to end, excess space at start.

```
              [Item1][Item2][Item3]
```

### `Flex::Center`
Center content, equal space on both sides.

```
      [Item1][Item2][Item3]
```

### `Flex::SpaceBetween`
Distribute space between items, none at edges.

```
[Item1]      [Item2]      [Item3]
```

### `Flex::SpaceAround`
Space around each item (edges get half space).

```
  [Item1]    [Item2]    [Item3]
```

### `Flex::SpaceEvenly`
Equal space everywhere including edges.

```
   [Item1]   [Item2]   [Item3]
```

### `Flex::Legacy`
Legacy behavior - excess space goes to last element.

```
[Item1][Item2][Item3              ]
```

## Examples

### Centered Buttons

```rust
use ratatui::layout::{Constraint, Flex, Layout};

let [_, btn1, _, btn2, _] = Layout::horizontal([
    Constraint::Fill(1),
    Constraint::Length(10),
    Constraint::Length(2),   // Gap between buttons
    Constraint::Length(10),
    Constraint::Fill(1),
]).areas(area);

// Or with Flex
let buttons = Layout::horizontal([
    Constraint::Length(10),
    Constraint::Length(10),
])
.flex(Flex::Center)
.spacing(2)
.areas(area);
```

### Navigation Bar with SpaceBetween

```rust
let nav_items = Layout::horizontal([
    Constraint::Length(8),  // Home
    Constraint::Length(10), // Products
    Constraint::Length(8),  // About
    Constraint::Length(10), // Contact
])
.flex(Flex::SpaceBetween)
.areas(nav_area);
```

### Card Grid with SpaceEvenly

```rust
let cards = Layout::horizontal([
    Constraint::Length(20),
    Constraint::Length(20),
    Constraint::Length(20),
])
.flex(Flex::SpaceEvenly)
.areas(grid_area);
```

### Toolbar with Start Alignment

```rust
let toolbar = Layout::horizontal([
    Constraint::Length(8),   // Save
    Constraint::Length(8),   // Open
    Constraint::Length(8),   // New
])
.flex(Flex::Start)
.spacing(1)
.areas(toolbar_area);
```

## Visual Comparison

Given 3 items of 10 cells each in an 80 cell wide area:

```
Flex::Start:
[----10----][----10----][----10----]

Flex::End:
                              [----10----][----10----][----10----]

Flex::Center:
               [----10----][----10----][----10----]

Flex::SpaceBetween:
[----10----]              [----10----]              [----10----]

Flex::SpaceAround:
      [----10----]      [----10----]      [----10----]

Flex::SpaceEvenly:
         [----10----]         [----10----]         [----10----]
```

## Combining Flex with Margin and Spacing

```rust
let layout = Layout::horizontal([
    Constraint::Length(10),
    Constraint::Length(10),
    Constraint::Length(10),
])
.flex(Flex::Center)
.margin(2)           // 2 cells margin on all sides
.spacing(1);         // 1 cell between items

let areas = layout.areas(frame.area());
```

## When to Use Each

| Mode | Use Case |
|------|----------|
| `Start` | Default alignment, left-to-right content |
| `End` | Right-aligned content, action buttons |
| `Center` | Modal dialogs, centered forms |
| `SpaceBetween` | Navigation bars, toolbars |
| `SpaceAround` | Card layouts, icon grids |
| `SpaceEvenly` | Uniform distribution needed |
| `Legacy` | Backward compatibility only |
