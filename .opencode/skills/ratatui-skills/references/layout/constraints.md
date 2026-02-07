# Ratatui Constraints

## Constraint Types

### `Constraint::Length(n)`
Fixed size of exactly n cells.

```rust
// Header exactly 3 rows tall
Constraint::Length(3)
```

### `Constraint::Percentage(n)`
Relative size as percentage of available space.

```rust
// Take 50% of available space
Constraint::Percentage(50)
```

### `Constraint::Ratio(numerator, denominator)`
Proportional size using ratios.

```rust
// Take 1/3 of available space
Constraint::Ratio(1, 3)
```

### `Constraint::Fill(weight)`
Fill remaining space proportionally to weight.

```rust
// Two areas, second twice as large
Layout::horizontal([
    Constraint::Fill(1),  // 1/3 of remaining
    Constraint::Fill(2),  // 2/3 of remaining
])
```

### `Constraint::Min(n)`
Minimum size of at least n cells. Highest priority.

```rust
// At least 10 cells, but can grow
Constraint::Min(10)
```

### `Constraint::Max(n)`
Maximum size of at most n cells.

```rust
// Up to 50 cells, but can shrink
Constraint::Max(50)
```

## Priority Resolution

When constraints conflict, they are resolved by priority:

1. **Min** - Highest priority, always satisfied first
2. **Max** - High priority
3. **Length/Percentage/Ratio** - Medium priority
4. **Fill** - Lowest priority, takes remaining space

## Examples

### Fixed Header + Flexible Body + Fixed Footer

```rust
let [header, body, footer] = Layout::vertical([
    Constraint::Length(3),   // Fixed header
    Constraint::Fill(1),     // Flexible body
    Constraint::Length(1),   // Fixed footer
]).areas(frame.area());
```

### Sidebar with Min/Max

```rust
let [sidebar, main] = Layout::horizontal([
    Constraint::Min(20).max(40),  // 20-40 cells
    Constraint::Fill(1),           // Rest
]).areas(frame.area());
```

### Equal Columns

```rust
// Three equal columns
let [a, b, c] = Layout::horizontal([
    Constraint::Ratio(1, 3),
    Constraint::Ratio(1, 3),
    Constraint::Ratio(1, 3),
]).areas(frame.area());

// Or using Fill
let [a, b, c] = Layout::horizontal([
    Constraint::Fill(1),
    Constraint::Fill(1),
    Constraint::Fill(1),
]).areas(frame.area());

// Or shorthand
let [a, b, c] = Layout::horizontal([Constraint::Fill(1); 3]).areas(frame.area());
```

### Weighted Distribution

```rust
// Left: 1 part, Middle: 2 parts, Right: 1 part
let [left, middle, right] = Layout::horizontal([
    Constraint::Fill(1),
    Constraint::Fill(2),
    Constraint::Fill(1),
]).areas(frame.area());
```

### Mixed Constraints

```rust
let [fixed, flex, percent] = Layout::horizontal([
    Constraint::Length(20),      // Fixed 20
    Constraint::Fill(1),         // Fill remaining
    Constraint::Percentage(25),  // 25% of total
]).areas(frame.area());
```

## Constraint Chaining

Constraints support method chaining for combining:

```rust
// Between 10 and 50 cells
let constraint = Constraint::Min(10).max(50);

// At least 20, but prefer 30
let constraint = Constraint::Min(20).length(30);
```

## Dynamic Layouts

When constraint count isn't known at compile time:

```rust
fn create_columns(count: usize, area: Rect) -> Vec<Rect> {
    let constraints: Vec<Constraint> = (0..count)
        .map(|_| Constraint::Fill(1))
        .collect();

    Layout::horizontal(constraints)
        .split(area)
        .to_vec()
}
```
