# Ratatui Skills

[中文](README-zh.md) | [日本語](README-ja.md)

> Comprehensive skills collection for Rust's ratatui TUI library.

## Structure

```
ratatui-skills/
├── SKILL.md              # Main entry point (overview + quick reference)
├── skills/               # Sub-module skills
│   ├── basics/          # Terminal init, app structure, event loop
│   ├── layout/          # Constraint, Rect, Flex, split areas
│   ├── widgets/         # Block, List, Table, custom widgets
│   └── styling/         # Color, Style, Modifier, Text/Span/Line
└── references/          # Detailed documentation
    ├── _shared/         # Shared rules (rust-defaults.md)
    ├── basics/          # App structure, backends
    ├── layout/          # Constraints, flex modes
    ├── widgets/         # Built-in widgets, custom widgets
    └── styling/         # Colors, text styling
```

## Usage

### As Claude Code Skills

Copy or symlink this directory to your Claude Code skills directory:

```bash
# Option 1: Symlink
ln -s /path/to/ratatui-skills ~/.claude/skills/ratatui

# Option 2: Copy
cp -r /path/to/ratatui-skills ~/.claude/skills/ratatui
```

### Trigger Keywords

The skill activates on keywords like:
- `ratatui`, `TUI`, `terminal ui`, `ratatui::run`, `ratatui::init`
- `Layout`, `Constraint`, `Rect`, `Flex`, `horizontal`, `vertical`
- `Block`, `Paragraph`, `List`, `Table`, `Gauge`, `Chart`
- `Style`, `Color`, `Stylize`, `Span`, `Line`, `Text`
- `Widget`, `StatefulWidget`, `ListState`, `TableState`

## Modules

| Module | Purpose | Key APIs |
|--------|---------|----------|
| **basics** | Terminal setup | `ratatui::run()`, `init()`, `restore()`, `DefaultTerminal` |
| **layout** | Screen layout | `Layout`, `Constraint`, `Rect`, `Flex` |
| **widgets** | UI components | `Block`, `List`, `Table`, `Gauge`, `Scrollbar` |
| **styling** | Colors & text | `Style`, `Color`, `Stylize`, `Span`, `Line`, `Text` |

## Version

- **ratatui**: 0.30.0
- **Rust edition**: 2024
- **Last updated**: 2026-01-19
