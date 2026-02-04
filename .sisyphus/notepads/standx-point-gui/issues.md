# Issues - standx-point-gui

## 2026-02-03 Initial Setup

(No issues yet - starting fresh)
- Multiple core-graphics versions (0.24.0 vs 0.25.0) in the dependency graph cause zed-font-kit to fail on macOS.
- Multiple core-graphics versions (0.24.0 vs 0.25.0) in the dependency graph cause zed-font-kit to fail on macOS.

## 2026-02-04 Build Verification

- standx-point-gui fails to compile due to GPUI API mismatches (ViewContext, App::new signature, Render::render signature).

## 2026-02-04 Module Compilation Errors

- **Issue**: `crates/standx-point-gui/src/price/mod.rs` and `crates/standx-point-gui/src/task/state_machine.rs` contain compilation errors (e.g. `tokio::sync::mpsc::Receiver` clone issue, `Arc` mutability issue).
- **Workaround**: Temporarily commented out `mod price`, `mod task`, `mod db` in `crates/standx-point-gui/src/lib.rs` to allow building the UI skeleton.
- **Action Required**: Fix `price` and `task` modules before re-enabling them.

## overflow_y missing on gpui::Div
- Encountered "no method named overflow_y" error on `gpui::Div`.
- `crates/standx-point-gui/src/ui/account_panel.rs` uses `.overflow_y(Overflow::Scroll)`, but `account_panel.rs` is commented out in `mod.rs`, so it is not compiled. This caused confusion.
- Temporarily removed scrolling from `TaskDetailPanel` to pass compilation.
