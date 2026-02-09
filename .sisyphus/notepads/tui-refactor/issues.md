
## AppState extraction map (from crates/standx-point-mm-strategy/src/tui/mod.rs)

### Fields and Types
- `storage`: `Arc<Storage>` (Persistence)
- `task_manager`: `Arc<TokioMutex<TaskManager>>` (Execution control)
- `log_buffer`: `LogBufferHandle` (Arc<StdMutex<LogBuffer>>)
- `tasks`: `Vec<StoredTask>` (UI snapshot of tasks)
- `list_state`: `ListState` (Selection index)
- `status_message`: `String` (Footer text)
- `last_refresh`: `Instant` (Throttle for storage refresh)
- `last_live_refresh`: `Instant` (Throttle for API polling)
- `live_data`: `HashMap<String, LiveTaskData>` (Transient account status)

### Dependencies
- `StandxClient`, `Balance`, `Position`, `Order` from `standx-point-adapter`
- `TaskManager`, `TaskRuntimeStatus`, `TaskMetricsSnapshot` from `standx-point-mm-strategy`

### Event Loop Branches (Hotkeys)
- `q`: Quit
- `r`: Refresh tasks (Storage)
- `s`: Start selected task
- `x`: Stop selected task
- `Up/Down`: Navigate task list

### UI Component Analysis
- Reusable Modal/Select Patterns: **NONE**. The current TUI is a monolithic layout.
- `cli/interactive.rs` uses `dialoguer` but this is for standard CLI input, not Ratatui.

### Migration Blockers/Observations
- The event loop is a large match block inside `run_tui_with_log`.
- `AppState` methods combine data fetching (Storage/API) with state updates.
- Refactoring should separate "Application State" from "UI/Component State".

## 2026-02-09 Task: Wave1-Task2
- Moved AppState, LiveTaskData, UiSnapshot into `crates/standx-point-mm-strategy/src/tui/app.rs`.
- Extracted key handling into `crates/standx-point-mm-strategy/src/tui/events.rs` and wired in `run_tui_with_log`.
- Moved refresh helpers (`refresh_tasks`, `build_snapshot`, `refresh_live_data`) into `crates/standx-point-mm-strategy/src/tui/state.rs`.

## 2026-02-09 Task: Wave2-Task3
- Split draw functions into `crates/standx-point-mm-strategy/src/tui/ui/*` and re-exported via `ui/mod.rs`.
- `tui/mod.rs` now calls `ui::*` for account/task/positions/orders/logs panels.

## 2026-02-10 Task: Wave2-Task4
- Added Tab enum + current_tab in AppState, with next_tab/set_tab helpers.
- Added Tab hotkeys in `events::handle_key_event` (Tab/l/1/2/3).
- Added tab bar rendering via `ui/layout::draw_tabs` and tab-based content in `draw_ui`.

## 2026-02-10 Task: Wave2-Task5
- Updated footer hotkeys to include Tab switching and create shortcuts.

## 2026-02-10 Task: Wave3-Task6
- Implemented modal framework types and helpers in `tui/ui/modal/mod.rs` with dead_code allowances until integration.

## 2026-02-10 Task: Wave3-Task7
- Added `CreateAccountModal` struct and `to_modal` builder in `tui/ui/modal/create_account.rs`.

## 2026-02-10 Task: Wave3-Task8
- Added `CreateTaskModal` struct and `to_modal` builder in `tui/ui/modal/create_task.rs`.

## 2026-02-10 Task: Wave4-Task9
- Added price snapshot refresh via `query_symbol_price` and rendered Mark/Last/Min in account summary.

## 2026-02-10 Task: Wave4-Task10
- Extended open orders table with TP/SL/Reduce/Time and payload parsing in `tui/ui/orders.rs`.

### Move Note
- 2026-02-09: Moved `AppState`, `LiveTaskData`, and `UiSnapshot` into `tui/app.rs`; `tui/mod.rs` now imports them.
