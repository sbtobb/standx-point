- Use one long-lived blocking crossterm event reader that forwards Key presses into the app channel; per-tick spawn_blocking can drop input events.

- Tests: wiremock can hit "connection closed before message completed" if the mock server is dropped while background tasks are still issuing requests; waiting for expected request counts makes shutdown tests stable.
- Fixed TUI key handling for tmux send-keys: removed KeyEventKind::Press filter since injected keys may have different event kinds (e.g., Repeat or Release)
- Architecture: Move form state structs (like `AccountForm`) to `app/state.rs` or `models` to avoid cyclic dependencies between `ui` (render) and `app` (state/logic).
- Pattern: Keep `ui` components as pure render functions; handle input logic in `App` event handlers or helper methods on the state struct, but keep persistence logic in `App`.
- Edit Mode in Forms: Passing `is_edit: bool` to input handlers allows reusing form logic while enforcing read-only constraints (skipping ID field navigation).
- State Management: Carrying context in `ModalType` variants (e.g. `AccountForm(form, is_edit)`) simplifies passing state to render/input handlers.
- TUI Modal Patterns: When using a generic `Confirm` modal in Ratatui/State pattern, embedding a `ConfirmAction` enum in the `ModalType::Confirm` variant is a clean way to pass context (e.g., *which* item to delete) from the trigger point (Normal mode) to the execution point (Dialog mode handler). This avoids needing separate mutable state variables like `pending_delete_id` in the main `AppState`, keeping the state cleaner.

- TUI shutdown: a `spawn_blocking` loop stuck in `crossterm::event::read()` can keep the Tokio runtime alive and prevent process exit; prefer `event::poll(timeout)` with a shutdown flag so the reader wakes up, observes shutdown, and exits promptly.

- TaskForm State Management: Moved `TaskForm` struct and input handling logic from `ui/components/task_form.rs` to `app/state.rs`.
  - Reason: The UI component should only be responsible for rendering (stateless/pure function).
  - Benefit: Decouples UI rendering from application state and logic. `TaskForm` in `app/state.rs` can now directly interact with `Task` domain model validation.
  - Pattern: This follows the pattern established by `AccountForm`.

- TUI Refactor - Task Edit Implementation
  - Pattern: Implemented edit flow by reusing `TaskForm` with an `is_edit` flag.
  - State Management: Modified `AppState` to handle `edit_selected_item` and pop the modal.
  - Persistence: Updated `DialogAction::SubmitTask` in `app/mod.rs` to handle update vs create logic using `Storage::update_task`.
  - Validation: Reused `TaskForm::to_task` for validation, extracting fields to update existing task in storage closure.
  - Constraints: Maintained separation of concerns - `TaskForm` handles UI state/input, `Storage` handles persistence, `AppState` coordinates.

- Implemented Task Delete flow parallel to Account Delete.
- Extended ConfirmAction and DialogAction enums to handle task deletion specifically.
- Ensured sync render by keeping async storage calls in the event handler loop.

- Per-task cancellation: keep a root `CancellationToken` for global shutdown, but create + store a child token per `task_id` alongside its `JoinHandle`. `stop_task(task_id)` cancels only that child and joins with a deadline; `shutdown_and_wait` cancels root and joins/aborts all remaining handles.
- TUI integration: persist task state (`Running`/`Stopped`/`Failed(msg)`) in storage independently from runtime state; refresh `AppState.tasks` from storage after start/stop so sync render stays accurate.

- Market data in TUI: keep `terminal.draw` fully synchronous; refresh `AppState.price_cache` on `AppEvent::Tick` by locking the shared `MarketDataHub` outside render and copying a snapshot for pure UI reads.
- One hub instance: wire the UI and `TaskManager` to the same `Arc<tokio::sync::Mutex<MarketDataHub>>`, and eagerly `subscribe_price` for persisted task symbols on startup so prices can show up before tasks are started.
- Testing note: binary-crate unit tests cannot rely on `#[cfg(test)]` APIs from dependency crates; prefer `cfg(debug_assertions)` or a dedicated feature for cross-crate test helpers.

- Graceful TUI quit: do shutdown work in the async event handler (not inside `terminal.draw`), and only set the run-loop exit flag after `TaskManager::shutdown_and_wait()` + storage persistence + hub shutdown complete.
- Raw-mode Ctrl+C: handle it as a key event (`Char('c')` + `CONTROL`) rather than relying on OS signals; treat it as a global quit path so it still works from Dialog mode.
- Input reader robustness: avoid `blocking_send` from a `spawn_blocking` input thread to a bounded channel when the UI loop might be busy (e.g. shutdown). Prefer `try_send` and drop excess key events to prevent the reader from deadlocking process exit.
- Tests: isolate on-disk Storage by allowing a test-specific data dir (`Storage::new_in_dir`) to avoid mutating the user's real `standx-mm` data.

- TUI Status Bar: Computed fields (like "Active: X/Y") can be derived on the fly in `render` from `AppState` collections (like `tasks`), keeping the state source of truth simple and avoiding redundant counters in `AppState`.

- Quit confirmation: Use `ModalType::Confirm { .., action: ConfirmAction::QuitAndStopTasks { running_tasks } }` so Dialog mode can map `y`/`Enter` to a shutdown request, and run `App::shutdown_and_exit()` only after releasing the `RwLock` guard (avoids deadlocks; render stays sync).

- `ratatui::frame::Frame::area()` is the robust way to check terminal size in immediate mode rendering.
- `crossterm::event::Event::Resize` must be forwarded to the app loop to trigger redraws, even if the payload isn't strictly needed for the `terminal.draw` call (as it fetches size from backend).
- `TestBackend` is very effective for verifying UI logic without a real terminal, including size constraints.

- **TUI Test Patterns**: No existing tests use `ratatui::backend::TestBackend` or PTY frameworks (`expectrl`, `insta`). Current tests are limited to:
  1. Form state validation tests (task_form.rs, account_form.rs)
  2. HTTP API integration tests with wiremock (tests/integration_test.rs)
  3. WebSocket reconnection backoff calculation (tests/reconnection_test.rs)
  4. Placeholder shutdown tests (tests/shutdown_test.rs)

- **Test Backend Options for Phase 7**:
  - `ratatui::backend::TestBackend` for unit testing render functions (pass Frame to components without real terminal)
  - `insta` for snapshot testing of rendered UI layouts
  - `expectrl` for PTY-driven integration tests (testing actual CLI behavior with --config)

- **Key Files to Modify/Add for Phase 7**:
  - `crates/standx-point-mm-strategy/Cargo.toml`: Add `dev-dependencies`: `insta`, `expectrl`, `tempfile`
  - `crates/standx-point-mm-strategy/src/ui/components/mod.rs`: Export all components for testing
  - `crates/standx-point-mm-strategy/tests/tui_render_tests.rs`: TestBackend-based unit tests for each UI component
  - `crates/standx-point-mm-strategy/tests/tui_snapshot_tests.rs`: Insta-based snapshot tests for render output
  - `crates/standx-point-mm-strategy/tests/cli_integration_tests.rs`: PTY-driven CLI integration tests using --config
  - `crates/standx-point-mm-strategy/README.md`: Add "Testing" section with TUI-specific test commands and patterns

- **Logging Plan**: Add tracing spans for TUI events (key presses, modal changes, render cycles) to allow debugging UI issues without PTY recording.

- **TUI Min-Size Overlay**:
  - Implemented check in `render` using `frame.area()`.
  - Used `ratatui::widgets::Clear` to wipe background before drawing overlay.
  - Extracted `render_overlay` and `is_terminal_too_small` helpers to enable unit testing without mocking complex `AppState` dependencies.
  - Verified with `ratatui::backend::TestBackend` which allows inspecting the buffer content for specific strings.

- **TUI Help Overlay Modal Logic**:
  - Implemented modal key interception pattern in `App::handle_event` where a boolean flag (`show_help`) takes precedence over `AppMode`.
  - Standardized toggle behavior: F1 opens/closes, Esc closes, and all other keys are strictly consumed/ignored while help is open to prevent background actions.
  - Updated help text to explicitly reflect "Press F1 or Esc to close" for better UX clarity.
- Implemented vi-style sidebar jumps: 'gg' (jump to first item) with tick-based prefix timeout, 'G' (jump to last item) directly
## Test Isolation for Storage

When writing tests for components that interact with the file system, it's important to ensure tests are hermetic (isolated from each other and the real user data). Here's a pattern used:

1. **Test-only constructor**: Add a `#[cfg(test)]` marked constructor like `Storage::new_in_dir` that takes a specific directory path instead of using the default `dirs::data_dir()/standx-mm`.

2. **Unique temp directories**: Create unique temporary directories for each test using a combination of process ID (PID) and timestamp (SystemTime nanos) to ensure no collisions.

3. **Best-effort cleanup**: Implement a cleanup function that ignores errors, ensuring tests don't leave artifacts behind even if they fail.

This approach avoids:
- Tests interfering with each other
- Tests reading/writing real user data
- Contamination of test state between runs

- **Keypress Flash Pattern**: Implemented a tick-based flash message system in the status bar that shows the last key pressed for ~500ms without overwriting the long-lived status_message. Key components:
  - Added `keypress_flash: Option<(String, u8)>` field to AppState
  - Updated AppState::update_tick() to decrement and clear flash when ticks reach 0
  - Set flash in AppEvent::Key handler (when help overlay not shown) with 2 ticks (~500ms at 250ms per tick)
  - Modified status_bar::render to prioritize flash over status_message if present
- **F4 Credentials Toggle Pattern**: Implemented credentials visibility toggle using F4 key with keypress flash feedback. Added  field to AppState, toggle method, key handler, and modified render_account_details to respect the flag. Updated help.rs to document the new shortcut.
- **Activity Spinner Animation**: Implemented a tick-based ASCII spinner animation for async operations in the status bar. Key components:
  - Added `spinner_ticks: u8` (remaining duration) and `spinner_frame: u8` (current animation frame) fields to AppState
  - Updated AppState::update_tick() to decrement ticks and advance frame (mod 4) when spinner is active
  - Added start_selected_task(), stop_selected_task(), stop_all_tasks(), and stop_spinner() methods to AppState
  - Updated key handlers in mod.rs to set spinner when 's' (start task) or 'x' (stop all tasks) is pressed
  - Implemented rendering in status_bar.rs using ASCII frames ['|', '/', '-', '\\']
  - Duration: 3 seconds (12 ticks at 250ms per tick)
- **F4 Credentials Toggle Pattern**: Implemented credentials visibility toggle using F4 key with keypress flash feedback. Added `show_credentials: bool` field to AppState, toggle method, key handler, and modified render_account_details to respect the flag. Updated help.rs to document the new shortcut.
- **Focus Cycling**: Implemented Tab focus cycling between Sidebar -> Detail -> Menu -> Sidebar by adding `Pane::Menu` to `Pane` enum, updating the `Tab` key handler logic, and adding visual focus indication (background style) to `menu_bar::render`. Updated detail view hint logic to treat Menu focus similarly to Sidebar (showing hints instead of details) by checking `!= Pane::Detail`.
- **Auto-exit Mechanism**: Added an env-var gated auto-exit feature for TUI mode to support automated tests. The `STANDX_TUI_TEST_EXIT_AFTER_TICKS` variable takes a positive integer N, and the TUI run loop exits after N ticks (each tick is 250ms) with Ok. The feature is disabled when the env var is unset, maintaining production behavior.
- CLI mode backward compatibility testing: Created integration test using existing binary invocation pattern. The test verifies that --config and --dry-run flags work together correctly by running the compiled binary with a test configuration and checking for a successful exit status. This ensures that CLI mode remains functional alongside the new TUI mode.

## Documentation Conventions (2026-02-05)
- **TUI Documentation**: When documenting the TUI, followed a "Bilingual Hybrid" approach:
    - **Prose/Explanation**: Chinese (Simplified) to comply with project rules for assistant communication.
    - **Technical Terms**: English (Commands, Keybindings, Envs, Flags) to maintain consistency with the codebase and developer experience.
- **Workflow-oriented**: Structured the TUI guide around the "First-run" experience (Account creation -> Task creation -> Execution).
- **Shortcuts Alignment**: Ensured the README shortcut table is an exact mirror of the implementation in `crates/standx-point-mm-strategy/src/ui/components/help.rs`.
- **TUI Logging Implementation**: Added structured logging for TUI operations using tracing crate. Key log points:
  - App startup: info level log with mode (TUI)
  - Key events: debug level log with key details
  - Help overlay: info level log when opened/closed
  - Dialog/modal operations: info level log for create, edit, delete dialogs
  - Task operations: info level log for start task, stop all tasks
  - Shutdown: info level log for shutdown start and complete
  - Terminal resize: debug level log with new dimensions
  - Log fields: Added contextual fields like sidebar_mode, focused_pane, and selected_index/task_id where available

## Ctrl+C 强制退出实现

**问题**: 需要实现 Ctrl+C 强制退出功能，该功能应在所有模式下立即生效，包括帮助覆盖层打开时。

**解决方案**:
1. 在 `handle_event` 方法的最开头添加 Ctrl+C 检测逻辑
2. 该处理逻辑绕过了所有其他事件处理（包括帮助覆盖层的键消耗）
3. 设置 `should_exit = true` 并添加 `action="force_quit"` 日志
4. 更新帮助组件以显示新的快捷键

**实现细节**:
- 在 `crates/standx-point-mm-strategy/src/app/mod.rs` 中添加 Ctrl+C 处理
- 在 `crates/standx-point-mm-strategy/src/ui/components/help.rs` 中更新帮助文本
- 所有测试通过，验证了功能正确性

- Added unit tests to verify Ctrl+C force quit works as a global shortcut
  - Tests cover normal mode, help mode, dialog mode, and insert mode
  - Followed existing test patterns using temporary directories for isolation
  - All tests pass successfully
## Price Subscription Placement

**Why place price subscriptions in `run_tui_mode` instead of `App::new()`?**

Placing subscriptions in `run_tui_mode` (after test mode check) prevents the MarketDataHub from starting its WebSocket worker during tests. The `STANDX_TUI_TEST_EXIT_AFTER_TICKS` test mode skips terminal initialization and should avoid network connections entirely to keep tests fast and reliable.

Subscriptions are added for BTC-USD and ETH-USD - the two symbols displayed in the status bar. This ensures the status bar shows real-time prices when the app is running in normal TUI mode.

- Account form modal pattern: store `AccountForm` inside `ModalType`, handle input in AppState insert mode, clear sensitive fields on cancel/close, and refresh cached lists after persistence.
- 账户编辑：在 `is_edit` 模式下强制 ID 只读，跳过 ID 字段切换，并在输入处理层阻止修改。

- Account deletion: store `ConfirmAction::DeleteAccount { account_id }` in `ModalType::Confirm`, execute in `close_dialog`, refresh accounts, and clamp `selected_index` after removal.
- 任务创建模态流程：在 Tasks 侧栏按 `n` 打开 `TaskForm`，Insert 模式处理输入；`Enter` 先做 `TaskForm::to_task` 校验，再检查 `account_id` 是否存在，成功后 `Storage::create_task` + `list_tasks` 刷新并关闭模态，失败则保留窗口并写入 `error_message`。

- TaskManager 单任务停止：实现 `stop_task(task_id)` 时先从 map `remove` 出 `JoinHandle`/token 再 `await`，避免跨 `await` 的可变借用；超时分支先 `abort()` 再返回错误，保证 UI 不会被卡住。

- 任务启停：在 Tasks 模式下用 `s`/`x` 控制单任务启动/停止，成功后持久化 `TaskState::Running/Stopped` 并刷新任务列表，同时在刷新后 clamp `selected_index`，避免索引越界。
## Clippy Fix Learnings (Feb 5, 2026)

1. **Get First Element**: Replace `.get(0)` with `.first()` for better readability and idiomatic Rust
2. **Collapsible If**: Combine nested `if let Some(x) = y { if x > z { ... } }` into `if let Some(x) = y && x > z { ... }`
3. **Question Mark Operator**: Replace `let Some(x) = y else { return None; }` with `let x = y?;` for conciseness
4. **FromStr Trait**: Implement `std::str::FromStr` instead of a standalone `from_str` method to avoid ambiguity
5. **Dead Code**: Add `#[allow(dead_code)]` to unused enum variants, traits, and methods that are intended for future features
6. **Too Many Arguments**: Use `#[allow(clippy::too_many_arguments)]` for methods that need many parameters temporarily
7. **Path vs PathBuf**: Use `&Path` instead of `&PathBuf` in function arguments to accept both Path and PathBuf inputs
8. **Useless Vec**: Replace `vec!["a", "b"]` with array `["a", "b"]` when the collection size is fixed
## View Details Shortcuts Implementation

Implemented "view details" shortcuts for the TUI:
- `Enter` key: When sidebar has focus and an item is selected, focuses Detail pane to show the item's details
- `v` key: Same behavior as Enter for both Accounts and Tasks modes
- Handled edge case when no selectable item is selected (shows appropriate status message)

Key changes:
- Modified `handle_normal_mode` function in `crates/standx-point-mm-strategy/src/app/mod.rs`
- Changed the behavior of Enter key from directly focusing Detail pane to first checking if an item is selected
- Added 'v' key as an alias for the same functionality
- Improved user feedback by showing a status message when no item is selected

- TUI 退出：在事件处理里设置 `exit_requested`，释放 `RwLock` 后再执行 `TaskManager::shutdown_and_wait()`、持久化 `Running -> Stopped`、`MarketDataHub::shutdown()`，最后再设置 `should_exit`，避免在 render 中做清理。
- 并发规则：不要在持有 `RwLock` guard 时 `await` 可能阻塞的 shutdown/IO；先提取所需数据再 `await`，避免死锁。
- Keep cargo test -p standx-point-mm-strategy clean from warnings by removing unused imports/variables in tests and dead code in test modules
- Account form Ctrl+K shortcut: Implemented Ctrl+K to clear currently focused field in insert mode. Respect edit mode read-only ID field and clear error message after clearing.
- Task form Ctrl+K shortcut: Implemented Ctrl+K to clear currently focused field in insert mode, following the same pattern as AccountForm. Respect edit mode read-only ID field (field 0) and clear error message after clearing.
- Cargo workspace patches: Patch overrides for crates-io must be placed in the root Cargo.toml. Patch blocks in non-root crates are ignored and produce a 'patch for the non root package will be ignored' warning.
- [patch.crates-io] only applies to crates from crates.io; git dependencies (like zed-font-kit referenced via git URL) aren't affected by crates-io patch blocks, which is why the patch was unused and produced a warning.
- Keep workspace warnings clean by renaming unused variables with `_` prefix (e.g., `client` → `_client`) in examples or test code that isn't actively using the variable yet.
- For unused import warnings in Rust: Move imports used only in tests inside the `#[cfg(test)]` test module to avoid warnings when compiling in non-test mode.
- Keep entire workspace warnings clean by checking `cargo test --workspace` regularly and addressing warnings. For unused test utilities in shared modules that are used by some but not all test binaries, add `#[allow(dead_code)]` to suppress warnings.
- AccountForm Ctrl+A select-all semantics: Added `replace_on_next_input` boolean field to AccountForm. Pressing Ctrl+A sets the flag, next character replaces entire field, Backspace clears entire field if flag is set, and flag resets after action. Respects edit mode read-only ID field.
- Added tests for select-all functionality: test_account_form_select_all_replace and test_account_form_select_all_backspace
