/*
[INPUT]:  TUI submodules and exported runtime helpers
[OUTPUT]: Public TUI entrypoints and re-exports
[POS]:    TUI module entrypoint
[UPDATE]: When changing TUI layout, keybindings, or runtime controls
[UPDATE]: 2026-02-09 Refactor layout and palette for account/positions/orders
[UPDATE]: 2026-02-09 Extract TerminalGuard into terminal.rs and add tui module layout
[UPDATE]: 2026-02-09 Move AppState types into app.rs
[UPDATE]: 2026-02-09 Move panel renderers into ui submodules
[UPDATE]: 2026-02-09 Add tab bar and tab-specific views
[UPDATE]: 2026-02-10 Use shared draw_tabs renderer
[UPDATE]: 2026-02-10 Move runtime logic to runtime.rs and keep thin re-exports
*/

mod app;
mod events;
mod runtime;
mod state;
mod terminal;
pub mod ui;

pub(crate) use runtime::LOG_BUFFER_CAPACITY;
pub use runtime::{LogBuffer, LogBufferHandle, LogWriterFactory, run_tui_with_log};
