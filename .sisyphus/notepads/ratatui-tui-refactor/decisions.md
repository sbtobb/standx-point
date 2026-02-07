## 2026-02-04

- Keep the ratatui render path synchronous; preload/refresh data in AppState to avoid awaiting inside `terminal.draw`.
