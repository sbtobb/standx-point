# Ratatui TUI Refactor for standx-point-mm-strategy

## TL;DR

Convert the existing CLI-based market making bot into a full TUI application with ratatui. The TUI provides a split-pane layout with a task sidebar, detailed account/task views, a global status bar showing live prices, and a bottom menu for operations. All configuration (accounts, tasks) is managed in-TUI with persistent storage, eliminating the need for manual YAML editing.

**Key Deliverables:**
- TUI application with ratatui (`main.rs` refactor)
- App state management with async event loop (`app/` module)
- UI layout components: status bar, sidebar, detail view, menu bar (`ui/` module)
- Account CRUD with persistence (`state/account.rs`)
- Task CRUD with start/stop integration (`state/task.rs`)
- In-TUI configuration forms (using `tui-input`, `tui-textarea`)

**Estimated Effort:** Large (XL)
**Parallel Execution:** NO - sequential architecture refactor
**Critical Path:** Dependencies → App State → Persistence → UI Layout → Account CRUD → Task CRUD → Integration → Testing

---

## Context

### Original Request
Refactor `crates/standx-point-mm-strategy` to use ratatui, converting the CLI into a TUI with:
1. Global status bar (price, task status)
2. Bottom operation menu
3. In-TUI configuration (no manual YAML)
4. Split-pane layout (sidebar task list, right detail view)
5. Account CRUD (multi-account)
6. Task CRUD with start/stop

### Existing Architecture
- **Binary entry point** (`main.rs`): CLI parsing with clap, config loading from YAML, graceful shutdown
- **Library structure** (`lib.rs`): Exports config, market_data, order_state, risk, strategy, task modules
- **Task management** (`task.rs`): TaskState enum (Init, Starting, Running, Stopping, Stopped, Failed), TaskManager with spawn/shutdown lifecycle
- **Configuration** (`config.rs`): StrategyConfig with TaskConfig (credentials, risk, sizing) - YAML-based
- **Market data** (`market_data.rs`): MarketDataHub with watch channel distribution
- **Current CLI**: clap-based with --config flag, dry-run option, init subcommand

### Key Constraints
- Must preserve existing task lifecycle and market data integration
- Must integrate with existing StandX adapter for trading
- Must maintain graceful shutdown behavior
- Must persist accounts/tasks across TUI sessions
- Must support async task execution within TUI event loop

### Research Findings
- **Ratatui patterns**: Event-driven architecture with crossterm, immediate mode rendering, stateful widgets
- **Async TUI integration**: Requires tokio runtime with crossterm event streams, message passing between UI and backend
- **State management**: Centralized AppState with message passing (channels) for UI updates
- **Persistence**: JSON/JSONL files for account/task storage (simpler than embedded DB for this use case)
- **Form input**: tui-input for single-line, tui-textarea for multi-line (e.g., signing keys)

---

## Work Objectives

### Core Objective
Convert the CLI-based market making bot into a full-featured TUI application with ratatui, enabling in-TUI account/task management with persistent storage and real-time market data display.

### Concrete Deliverables
1. **TUI Application** (`main.rs` refactor): Ratatui-based async event loop with graceful shutdown
2. **App State Module** (`app/`): Centralized state management with async message passing
3. **UI Module** (`ui/`): Layout components (status bar, sidebar, detail view, menu bar, dialogs)
4. **State Persistence** (`state/`): JSON-based account and task storage with CRUD operations
5. **Input Components** (`ui/input/`): Forms for account/task creation/editing using tui-input/tui-textarea
6. **Integration** (`app/integration.rs`): Bridge between TUI state and existing TaskManager/MarketDataHub

### Definition of Done
 - [x] TUI launches and displays split-pane layout (sidebar + detail view)
- [ ] Global status bar shows live price updates from MarketDataHub
 - [x] Bottom menu provides keyboard shortcuts for all operations
- [ ] Accounts can be created, edited, deleted, and persisted to JSON
- [ ] Tasks can be created, edited, deleted, started, stopped, and persisted to JSON
- [ ] Task lifecycle (Start → Running → Stop → Stopped) works through TUI controls
- [ ] All configuration is manageable in-TUI (no manual YAML editing required)
- [ ] Graceful shutdown works (Ctrl+C or menu option) with proper cleanup
 - [ ] Existing tests still pass (backward compatibility for library exports)

### Must Have
- Split-pane layout with sidebar task list and right-side detail view
- Global status bar showing live prices and task counts
- Bottom operation menu with keyboard shortcuts
- Account CRUD with multi-account support (persisted to JSON)
- Task CRUD with start/stop controls (persisted to JSON)
- In-TUI configuration forms (no manual file editing)
- Async TUI event loop with tokio integration
- Integration with existing TaskManager and MarketDataHub
- Graceful shutdown handling

### Must NOT Have (Guardrails)
- Do NOT modify existing library exports (`lib.rs` public API must remain compatible)
- Do NOT change existing TaskConfig/StrategyConfig structures (extend, don't modify)
- Do NOT remove existing CLI functionality (keep as legacy option or compile flag)
- Do NOT use external databases (keep persistence simple with JSON files)
- Do NOT block the TUI event loop with synchronous I/O (all operations must be async)
- Do NOT skip error handling in TUI (all errors must be displayed to user via dialogs)
- Do NOT use global mutable state (use proper AppState with message passing)

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES (existing tokio tests, wiremock for HTTP mocking)
- **Automated tests**: TDD for new modules, integration tests for TUI flows
- **Framework**: Existing tokio-test + wiremock, add ratatui-test for TUI testing

### Test Infrastructure (New)
- Add `ratatui::TestBackend` for unit testing TUI components
- Add `crossterm::event` mocking for keyboard input testing
- Add test fixtures for account/task JSON persistence
- Integration tests for full TUI lifecycle (startup → operation → shutdown)

### TDD Workflow (For New Modules)
Each new module follows RED-GREEN-REFACTOR:
1. **RED**: Write test for the module interface
2. **GREEN**: Implement minimum code to pass
3. **REFACTOR**: Clean up while keeping tests green

### Agent-Executed QA Scenarios (MANDATORY)

#### Scenario 1: TUI Startup and Layout Display
**Tool**: Bash (cargo run)
**Preconditions**: Terminal with 80x24 minimum size
**Steps**:
1. Run `cargo run --package standx-point-mm-strategy` (no --config needed for TUI mode)
2. Wait 2 seconds for TUI to initialize
3. Capture terminal state (expected: split-pane layout visible)
4. Verify: status bar at top shows "StandX MM Strategy" and price placeholder
5. Verify: sidebar on left shows "No accounts configured" or account list
6. Verify: right panel shows "Select a task" or task detail view
7. Verify: bottom bar shows menu options (F1=Help, F2=Accounts, F3=Tasks, F4=Settings, q=Quit)
8. Send 'q' key
9. Verify: TUI exits cleanly (back to shell prompt)
**Expected Result**: TUI displays correct layout, responds to quit command

#### Scenario 2: Account CRUD Flow
**Tool**: Bash (cargo run) + keyboard input simulation
**Preconditions**: TUI running, no accounts configured
**Steps**:
1. Run `cargo run --package standx-point-mm-strategy`
2. Wait for TUI to initialize
3. Press F2 (Accounts menu)
4. Press 'n' (New account)
5. Wait for account creation form
6. Type: Name="Test Account", JWT Token="test-jwt-123", Signing Key="test-key-456"
7. Press Enter to confirm
8. Verify: Account appears in sidebar list
9. Press 'e' to edit account
10. Change name to "Test Account Updated"
11. Press Enter to confirm
12. Verify: Updated name appears in sidebar
13. Press 'd' to delete account
14. Press 'y' to confirm deletion
15. Verify: Account removed from sidebar
16. Press 'q' to quit
**Expected Result**: Full account CRUD cycle completes successfully

#### Scenario 3: Task Lifecycle (Start/Stop)
**Tool**: Bash (cargo run) + keyboard input
**Preconditions**: TUI running, at least one account configured
**Steps**:
1. Run `cargo run --package standx-point-mm-strategy` with account already configured
2. Wait for TUI to initialize
3. Verify: Sidebar shows accounts and their tasks
4. Press F3 (Tasks menu)
5. Press 'n' (New task)
6. Fill task form: ID="btc-mm-1", Symbol="BTC-USD", Account=<select existing>, Risk Level="conservative"
7. Press Enter to confirm
8. Verify: Task appears in sidebar under selected account
9. With task selected, press 's' (Start)
10. Verify: Task status changes from "Stopped" to "Starting" to "Running"
11. Verify: Global status bar shows active task count incremented
12. Wait 5 seconds
13. Press 'x' (Stop)
14. Verify: Task status changes from "Running" to "Stopping" to "Stopped"
15. Press 'd' (Delete) then 'y' to confirm
16. Verify: Task removed from sidebar
17. Press 'q' to quit
**Expected Result**: Complete task lifecycle (create → start → run → stop → delete) works

---

## Execution Strategy

### Sequential Execution (NO Parallel Waves)

This is an architectural refactor with tight dependencies between components:

```
Phase 1: Dependencies (1 task)
│
Phase 2: App State Architecture (1 task)
│
Phase 3: Persistence Layer (1 task)
│
Phase 4: UI Layout Components (1 task)
│
Phase 5: Account CRUD (1 task)
│
Phase 6: Task CRUD (1 task)
│
Phase 7: Integration (1 task)
│
Phase 8: Keyboard Navigation (1 task)
│
Phase 9: Testing (1 task)
```

**Why Sequential:**
- Each phase builds on the previous (UI needs state, CRUD needs UI components)
- Shared state architecture must be established before UI components
- Integration requires all prior components to be functional

**Total Tasks:** 9 sequential phases
**Estimated Time:** 2-3 days for a senior Rust developer

---

## TODOs

### Phase 1: Initialize Dependencies and TUI Structure

**What to do:**
- Add ratatui, crossterm, tui-input, tui-textarea, unicode-width to Cargo.toml
- Create new directory structure: src/app/, src/ui/, src/state/
- Create entry point refactor plan in main.rs (keep existing CLI as legacy mode)
- Set up basic ratatui event loop structure with tokio integration

**Must NOT do:**
- Do NOT remove existing CLI functionality (add TUI as alternative mode)
- Do NOT modify existing library exports in lib.rs
- Do NOT skip the async runtime setup (must integrate tokio with ratatui)
- Do NOT use blocking I/O in the TUI event loop

**Recommended Agent Profile:**
- **Category**: visual-engineering
  - Reason: TUI is a visual interface requiring layout and widget expertise
- **Skills**: ["ratatui"]
  - ratatui: Essential for all TUI-related implementation
- **Skills Evaluated but Omitted**:
  - None - ratatui skill is the only specialized skill needed

**Parallelization:**
- **Can Run In Parallel**: NO
- **Parallel Group**: Sequential - Phase 1
- **Blocks**: Phase 2 (App State Architecture)
- **Blocked By**: None (can start immediately)

**References:**
- `Cargo.toml:1-27` - Current dependencies (add ratatui, crossterm, tui-input, tui-textarea)
- `src/main.rs:1-147` - Current CLI entry point (refactor to support TUI mode)
- Ratatui skill basics: `./skills/basics/SKILL.md` - Terminal init, app structure, event loop
- Ratatui skill layout: `./skills/layout/SKILL.md` - Constraint, Rect, Flex, split areas

**Why Each Reference Matters:**
- `Cargo.toml`: Add TUI dependencies without breaking existing workspace deps
- `main.rs`: Refactor entry point to detect TUI mode (no --config flag = TUI mode)
- Ratatui basics: Event loop pattern with tokio integration (critical for async tasks)
- Ratatui layout: Split-pane layout for sidebar + detail view (core UI requirement)

**Acceptance Criteria:**

**Dependencies:**
- [ ] `ratatui = "0.30"` added to Cargo.toml dependencies
- [ ] `crossterm = "0.28"` added with event-stream feature
- [ ] `tui-input = "0.11"` added for form inputs
- [ ] `tui-textarea = "0.7"` added for multi-line inputs
- [ ] `unicode-width = "0.2"` added for text width calculations
- [ ] `cargo check` passes with new dependencies

**Directory Structure:**
- [ ] `src/app/` directory created
- [ ] `src/ui/` directory created  
- [ ] `src/state/` directory created
- [ ] `src/app/mod.rs` created
- [ ] `src/ui/mod.rs` created
- [ ] `src/state/mod.rs` created

**Entry Point Refactor:**
- [ ] `main.rs` detects TUI mode when no --config flag provided
- [ ] CLI mode preserved when --config flag is present (backward compatible)
- [ ] TUI initialization with ratatui::init() implemented
- [ ] TUI cleanup with ratatui::restore() on exit
- [ ] Async tokio runtime integrated with ratatui event loop

**Basic Event Loop:**
- [ ] Event loop structure implemented (draw → handle events → update)
- [ ] Crossterm event reading integrated with tokio::select!
- [ ] Key event handling skeleton (q to quit, F-keys for menus)
- [x] Terminal resize handling
- [ ] Frame rate limiting (60 FPS target)

**Commit**: YES
- Message: `feat(tui): initialize ratatui dependencies and structure`
- Files: `Cargo.toml`, `src/main.rs`, `src/app/`, `src/ui/`, `src/state/`
- Pre-commit: `cargo check --package standx-point-mm-strategy`

---

### Phase 2: App State Architecture and Message Passing

**What to do:**
- Design and implement centralized AppState with all UI state (selected tab, focused pane, modal state)
- Create event/message system for communication between UI and backend (channels)
- Implement app modes (Normal, Insert, Command, Dialog) for vi-like navigation
- Create state machine for UI flows (AccountList → AccountCreate → AccountEdit)
- Set up async message handler that bridges UI events to TaskManager/MarketDataHub

**Must NOT do:**
- Do NOT use global mutable state (static variables) for app state
- Do NOT block the UI thread with synchronous I/O operations
- Do NOT mix UI rendering logic with business logic (keep them separated)
- Do NOT skip error handling in message passing (all errors must be captured and displayed)
- Do NOT use RefCell or other interior mutability in async contexts (use channels instead)

**Recommended Agent Profile:**
- **Category**: ultrabrain
  - Reason: Complex async state management with message passing requires deep Rust expertise
- **Skills**: ["ratatui", "rust"]
  - ratatui: Understanding of TUI app patterns
  - rust: Advanced async Rust patterns (channels, select!, Arc<Mutex<>>)
- **Skills Evaluated but Omitted**:
  - None - these two cover the requirements

**Parallelization:**
- **Can Run In Parallel**: NO
- **Parallel Group**: Sequential - Phase 2
- **Blocks**: Phase 3 (Persistence), Phase 4 (UI Layout)
- **Blocked By**: Phase 1 (Dependencies)

**References:**
- `src/task.rs:49-56` - TaskState enum (reusable for TUI state machine)
- `src/task.rs:59-69` - TaskManager structure (integrate with AppState)
- `src/market_data.rs` - MarketDataHub (subscribe to price updates)
- Ratatui skill basics: Event handling patterns with crossterm
- GPUI async skill: Channel patterns for async message passing (if available)

**Why Each Reference Matters:**
- `task.rs:TaskState`: Use as model for UI state machines (AccountState, TaskState)
- `task.rs:TaskManager`: Integrate with AppState - TUI controls start/stop, TaskManager executes
- `market_data.rs`: Subscribe to price updates, broadcast to UI via channels
- Ratatui patterns: Proper async event loop structure (not blocking on events)

**Acceptance Criteria:**

**AppState Structure:**
- [ ] `AppState` struct defined with all UI state fields:
  - `current_mode: AppMode` (Normal, Insert, Command, Dialog)
  - `selected_pane: Pane` (Sidebar, Detail, Menu)
  - `sidebar_state: SidebarState` (Accounts, Tasks)
  - `modal: Option<Modal>` (None, AccountForm, TaskForm, ConfirmDelete)
  - `status_message: Option<String>` (for user feedback)
- [ ] `AppState` is wrapped in `Arc<tokio::sync::RwLock<AppState>>` for thread-safe access
- [ ] AppState initialization loads persisted data from disk

**Message Passing System:**
- [ ] `AppEvent` enum defined with all possible events:
  - `Tick` (for periodic updates)
  - `Key(crossterm::event::KeyEvent)` (user input)
  - `PriceUpdate(Symbol, Decimal)` (market data)
  - `TaskStatusChange(task_id, TaskState)`
  - `AccountCreated/Updated/Deleted`
  - `ModalClose`
  - `Shutdown`
- [ ] `mpsc::channel(100)` created for event passing (bounded to prevent memory issues)
- [ ] Event handler loop implemented with `tokio::select!`:
  - Branch 1: Receive from crossterm event stream
  - Branch 2: Receive from app event channel
  - Branch 3: Receive shutdown signal
  - Branch 4: Periodic tick (every 250ms for UI updates)

**UI Mode State Machine:**
- [ ] `AppMode` enum with variants: `Normal`, `Insert`, `Command`, `Dialog`
- [ ] Mode transitions defined:
  - Normal → Insert: When entering text field
  - Insert → Normal: Press Esc
  - Normal → Command: Press ':'
  - Command → Normal: Press Esc or Enter
  - Normal → Dialog: Open modal
  - Dialog → Normal: Close modal
- [ ] Key event handler dispatches based on current mode

**Async Integration Bridge:**
- [ ] `IntegrationBridge` struct that connects AppState to TaskManager/MarketDataHub
- [ ] Method `subscribe_price(symbol)` that spawns market data subscription
- [ ] Method `start_task(task_id)` that calls TaskManager to spawn task
- [ ] Method `stop_task(task_id)` that calls TaskManager shutdown
- [ ] Price updates sent to UI via AppEvent channel
- [ ] Task status changes sent to UI via AppEvent channel

**Commit**: YES
- Message: `feat(tui): implement AppState architecture with message passing`
- Files: `src/app/`, `src/app/state.rs`, `src/app/event.rs`, `src/app/bridge.rs`
- Pre-commit: `cargo check --package standx-point-mm-strategy && cargo test --package standx-point-mm-strategy --lib`

---

### Phase 3: Persistence Layer for Accounts and Tasks

**What to do:**
- Design JSON-based persistence for accounts and tasks (simpler than DB for this use case)
- Implement storage module with atomic write operations (write to temp file, then rename)
- Create data models for persisted Account and Task (separate from config.rs TaskConfig)
- Implement CRUD operations: create, read (all), update, delete
- Add data migration/versioning for future schema changes
- Ensure thread-safe access (storage operations behind Arc<Mutex<>>)

**Must NOT do:**
- Do NOT use external databases (SQLite, PostgreSQL) - keep it simple with JSON
- Do NOT use blocking I/O in async contexts (use tokio::fs for file operations)
- Do NOT store credentials in plain text without consideration (document the trade-off)
- Do NOT skip atomic write operations (risk of data corruption on crash)
- Do NOT ignore file system errors (permission denied, disk full, etc.)

**Recommended Agent Profile:**
- **Category**: quick
  - Reason: File I/O and JSON serialization are straightforward tasks
- **Skills**: ["rust"]
  - rust: For async file operations with tokio::fs
- **Skills Evaluated but Omitted**:
  - None - simple module implementation

**Parallelization:**
- **Can Run In Parallel**: NO
- **Parallel Group**: Sequential - Phase 3
- **Blocks**: Phase 4 (UI Layout - needs persisted data to display)
- **Blocked By**: Phase 2 (AppState structure)

**References:**
- `src/config.rs:62-67` - StrategyConfig::from_file pattern (reuse for persistence)
- `src/task.rs:59-69` - TaskManager structure (integrate with storage)
- Tokio fs docs: `tokio::fs::write`, `tokio::fs::read_to_string` for async file I/O

**Why Each Reference Matters:**
- `config.rs`: Shows existing pattern for YAML loading - use similar pattern for JSON persistence
- `task.rs`: TaskManager currently loads from config - extend to load from persisted storage
- Tokio fs: All file operations must be async to not block the TUI event loop

**Acceptance Criteria:**

**Data Models:**
- [ ] `PersistedAccount` struct with fields:
  - `id: String` (UUID or user-provided)
  - `name: String`
  - `jwt_token: String`
  - `signing_key: String`
  - `created_at: DateTime<Utc>`
  - `updated_at: DateTime<Utc>`
  - `version: u32` (for schema migration)
- [ ] `PersistedTask` struct with fields:
  - `id: String` (user-provided, like "btc-mm-1")
  - `symbol: String` (e.g., "BTC-USD")
  - `account_id: String` (references PersistedAccount)
  - `risk_level: String` ("conservative", "moderate", "aggressive")
  - `max_position_usd: String` (decimal as string)
  - `price_jump_threshold_bps: u32`
  - `base_qty: String`
  - `tiers: u8`
  - `state: PersistedTaskState` (Stopped, Running, etc.)
  - `created_at: DateTime<Utc>`
  - `updated_at: DateTime<Utc>`
  - `version: u32`
- [ ] Enums for state: `PersistedTaskState { Stopped, Running, Failed(String) }`

**Storage Module:**
- [ ] `Storage` struct with fields:
  - `accounts_path: PathBuf` (e.g., ~/.local/share/standx-mm/accounts.json)
  - `tasks_path: PathBuf` (e.g., ~/.local/share/standx-mm/tasks.json)
  - `cache: Arc<Mutex<StorageCache>>` (in-memory cache for performance)
- [ ] `StorageCache` struct with:
  - `accounts: Vec<PersistedAccount>`
  - `tasks: Vec<PersistedTask>`
  - `dirty: bool` (whether cache needs persistence)
- [ ] `Storage::new()` that:
  - Creates data directory if not exists (use dirs::data_dir())
  - Loads existing accounts/tasks from JSON or starts with empty arrays
  - Initializes cache
- [ ] `Storage::load_accounts()` → `Result<Vec<PersistedAccount>>`:
  - Read file contents asynchronously with `tokio::fs::read_to_string`
  - Parse JSON with `serde_json::from_str`
  - Return error if file malformed (don't panic)
- [ ] `Storage::load_tasks()` → `Result<Vec<PersistedTask>>` (same pattern)
- [ ] `Storage::save_accounts(accounts: &[PersistedAccount])` → `Result<()>`:
  - Serialize to JSON with `serde_json::to_string_pretty`
  - Write to temp file first (`{path}.tmp`)
  - Rename temp file to target (`fs::rename`) for atomic write
  - Handle errors (permission denied, disk full, etc.)
- [ ] `Storage::save_tasks()` → `Result<()>` (same pattern)

**CRUD Operations:**
- [ ] `Storage::create_account(account: PersistedAccount)` → `Result<()>`:
  - Check for duplicate ID (return error if exists)
  - Add to cache
  - Persist to disk
- [ ] `Storage::update_account(id: &str, f: impl FnOnce(&mut PersistedAccount))` → `Result<()>`:
  - Find account by ID (return error if not found)
  - Apply closure to modify account
  - Update `updated_at` timestamp
  - Persist to disk
- [ ] `Storage::delete_account(id: &str)` → `Result<()>`:
  - Find account by ID
  - Check if any tasks reference this account (prevent deletion if so, or cascade delete)
  - Remove from cache
  - Persist to disk
- [ ] `Storage::get_account(id: &str)` → `Option<PersistedAccount>` (from cache)
- [ ] `Storage::list_accounts()` → `Vec<PersistedAccount>` (from cache, sorted by name)
- [ ] Same CRUD pattern for tasks: `create_task`, `update_task`, `delete_task`, `get_task`, `list_tasks`
- [ ] `Storage::get_tasks_for_account(account_id: &str)` → `Vec<PersistedTask>` (filtered list)

**Schema Migration:**
- [ ] `Storage::migrate()` called during initialization
- [ ] Check `version` field on loaded accounts/tasks
- [ ] If version < CURRENT_VERSION, apply migrations:
  - Version 1 → 2: Add `updated_at` field if missing
  - Version 2 → 3: Rename `risk_level` to `risk.profile` if needed
- [ ] Update version field after migration
- [ ] Log migrations applied

**Error Handling:**
- [ ] All storage methods return `Result<T, StorageError>`
- [ ] `StorageError` enum with variants:
  - `Io(std::io::Error)` - File system errors
  - `Serialization(serde_json::Error)` - JSON parse errors
  - `NotFound(String)` - Account/task not found
  - `Duplicate(String)` - Duplicate ID on create
  - `Corruption(String)` - Data integrity issues
- [ ] Display impl for StorageError (user-friendly messages)

**Testing:**
- [ ] Unit tests for all CRUD operations (use temp directory)
- [ ] Test atomic write (kill process mid-write, verify no corruption)
- [ ] Test schema migration (load old version data, verify migration applied)
- [ ] Test concurrent access (multiple tasks reading/writing)
- [ ] Integration test: create account → create task → delete account (cascade)

**Commit**: YES
- Message: `feat(storage): implement JSON persistence for accounts and tasks`
- Files: `src/state/`, `src/state/storage.rs`, `src/state/account.rs`, `src/state/task.rs`
- Pre-commit: `cargo test --package standx-point-mm-strategy state::`

---

### Phase 4: UI Layout Components

**What to do:**
- Implement split-pane layout (sidebar left, detail view right)
- Create global status bar component (top, full width)
- Create bottom menu bar component (bottom, full width)
- Create sidebar widget with selection state (List with highlight)
- Create detail view widget that shows account/task details
- Implement responsive layout (handle terminal resize)
- Add borders and styling (Block widgets with titles)

**Must NOT do:**
- Do NOT hardcode layout dimensions (use Constraint-based layouts)
- Do NOT skip handling terminal resize events
- Do NOT use blocking rendering (all render calls must be fast)
- Do NOT create custom widgets when built-in ones suffice
- Do NOT ignore color/style accessibility (use high-contrast defaults)

**Recommended Agent Profile:**
- **Category**: visual-engineering
  - Reason: TUI layout is visual design work (constraints, proportions, spacing)
- **Skills**: ["ratatui"]
  - ratatui: Layout, widgets, Block, styling - everything needed for UI

**Parallelization:**
- **Can Run In Parallel**: NO
- **Parallel Group**: Sequential - Phase 4
- **Blocks**: Phase 5 (Account CRUD - needs UI to render forms)
- **Blocked By**: Phase 3 (Persistence - UI needs data to display)

**References:**
- Ratatui skill layout: `./skills/layout/SKILL.md` - Layout constraints, Rect, Flex
- Ratatui skill widgets: `./skills/widgets/SKILL.md` - Block, List, Paragraph
- `examples/` in ratatui repo: `layout.rs`, `list.rs` for reference patterns

**Why Each Reference Matters:**
- Layout skill: Shows how to use Constraint::Percentage, Constraint::Ratio for split-pane
- Widgets skill: Block for borders/titles, List for sidebar, Paragraph for detail view
- Examples: Real-world patterns for common layouts (sidebar + main is very common)

**Acceptance Criteria:**

**Layout Structure:**
- [ ] Main layout areas defined using `Layout::vertical`:
  - `status_area` - Constraint::Length(3) - Top status bar
  - `main_area` - Constraint::Fill(1) - Main content area
  - `menu_area` - Constraint::Length(1) - Bottom menu bar
- [ ] Main content area split using `Layout::horizontal`:
  - `sidebar_area` - Constraint::Percentage(30) - Left sidebar
  - `detail_area` - Constraint::Fill(1) - Right detail view
- [ ] Layouts recalculated on terminal resize event

**Status Bar Component:**
- [ ] `StatusBar` struct with fields:
  - `title: &'static str` - "StandX MM Strategy"
  - `prices: HashMap<Symbol, Decimal>` - Live prices
  - `active_tasks: usize` - Count of running tasks
  - `status_message: Option<String>` - User feedback
- [ ] `render()` method returns `Block` widget:
  - Title centered at top
  - Prices displayed as "BTC: $65,432.10 | ETH: $3,456.78"
  - Task count: "Active: 3/5"
  - Status message in contrasting color
- [ ] Updates whenever price or task count changes

**Menu Bar Component:**
- [ ] `MenuBar` struct with menu items:
  - `F1` - Help
  - `F2` - Accounts
  - `F3` - Tasks
  - `F4` - Settings
  - `q` - Quit
- [ ] `render()` method returns `Paragraph` widget:
  - Items displayed horizontally: "F1 Help | F2 Accounts | F3 Tasks | F4 Settings | q Quit"
  - Shortcuts highlighted (e.g., bold F1, F2, etc.)
  - Centered or left-aligned based on space

**Sidebar Component:**
- [ ] `Sidebar` struct with fields:
  - `items: Vec<SidebarItem>` - Accounts or tasks
  - `selected: usize` - Currently selected index
  - `mode: SidebarMode` - Accounts | Tasks
- [ ] `SidebarItem` enum:
  - `Account { id, name, task_count }`
  - `Task { id, symbol, state }`
- [ ] `render()` method returns `List` widget:
  - Title shows "Accounts" or "Tasks" based on mode
  - Border around the list
  - Highlighted item shown with different background
  - Account items show "Test Account (3 tasks)"
  - Task items show "btc-mm-1 [Running] BTC-USD"
- [ ] Navigation: Up/Down arrows to select, Enter to view details

**Detail View Component:**
- [ ] `DetailView` enum with variants:
  - `Empty` - "Select an account or task to view details"
  - `Account { account: PersistedAccount, tasks: Vec<PersistedTask> }`
  - `Task { task: PersistedTask, account: PersistedAccount, metrics: TaskMetrics }`
- [ ] `render()` method returns `Block` with content:
  - Empty: Centered text "Select an account or task from the sidebar (F2/F3 to switch)"
  - Account:
    - Title: "Account: Test Account"
    - Section "Details": Name, ID, Created at
    - Section "Tasks": List of associated tasks with status
    - Section "Actions": [Edit (e)] [Delete (d)]
  - Task:
    - Title: "Task: btc-mm-1"
    - Section "Configuration": Symbol, Risk Level, Max Position
    - Section "Status": State (Running/Stopped), Uptime, Last Error
    - Section "Account": Linked account name
    - Section "Actions": [Start (s)] [Stop (x)] [Edit (e)] [Delete (d)]

**Terminal Resize Handling:**
- [ ] Resize event handler that recalculates all layouts
- [x] Minimum terminal size check (80x24)
- [x] If terminal too small: Display centered message "Terminal too small (need 80x24)" instead of normal UI
- [ ] Debounced resize (don't redraw on every pixel change, throttle to 100ms)

**Styling and Theming:**
- [ ] Color palette defined:
  - Primary: Blue (borders, titles)
  - Success: Green (running status, success messages)
  - Warning: Yellow (caution status, warnings)
  - Error: Red (failed status, errors)
  - Info: Cyan (prices, info messages)
  - Background: Default terminal background
  - Text: Default terminal foreground
- [ ] Style helper functions:
  - `style_bold()` - For titles and shortcuts
  - `style_dim()` - For secondary text
  - `style_highlight()` - For selected items
- [ ] Borders: Single line, rounded corners where supported

**Commit**: YES
- Message: `feat(ui): implement layout components (status bar, sidebar, detail view, menu bar)`
- Files: `src/ui/layout.rs`, `src/ui/status_bar.rs`, `src/ui/sidebar.rs`, `src/ui/detail_view.rs`, `src/ui/menu_bar.rs`
- Pre-commit: `cargo check --package standx-point-mm-strategy && cargo clippy --package standx-point-mm-strategy -- -D warnings`

---

### Phase 5: Account CRUD with In-TUI Forms

**What to do:**
- Implement Account CRUD dialogs (list view already in sidebar, now create/edit forms)
- Create input forms using tui-input for single-line fields (name, JWT token)
- Create textarea using tui-textarea for multi-line fields (signing key)
- Implement validation (required fields, format checks)
- Create confirmation dialog for delete operations
- Wire up menu shortcuts (F2 for accounts, 'n' for new, 'e' for edit, 'd' for delete)
- Update sidebar in real-time when accounts change

**Must NOT do:**
- Do NOT store signing keys in plain text without warning users
- Do NOT allow duplicate account IDs
- Do NOT skip validation on account creation (all fields required)
- Do NOT leave form data in memory after form closes (clear sensitive fields)
- Do NOT allow editing account ID (create new instead) - it's a foreign key

**Recommended Agent Profile:**
- **Category**: visual-engineering
  - Reason: Form UI design with input fields, validation feedback, and dialogs
- **Skills**: ["ratatui"]
  - ratatui: Widgets for forms (Paragraph, Block), dialog overlays
- **Skills Evaluated but Omitted**:
  - None

**Parallelization:**
- **Can Run In Parallel**: NO
- **Parallel Group**: Sequential - Phase 5
- **Blocks**: Phase 6 (Task CRUD - tasks reference accounts)
- **Blocked By**: Phase 4 (UI Layout - need forms to render)

**References:**
- `tui-input` crate docs: Input widget for single-line text
- `tui-textarea` crate docs: TextArea widget for multi-line text
- Ratatui skill widgets: `./skills/widgets/SKILL.md` - Block, Paragraph for form layout
- `src/config.rs:33-39` - CredentialsConfig structure (model account fields)

**Why Each Reference Matters:**
- tui-input: For JWT token and account name inputs (single line, limited width)
- tui-textarea: For Ed25519 signing key (often multi-line PEM format)
- Ratatui widgets: Block for form container with title, Paragraph for labels/help text
- CredentialsConfig: Shows existing credential structure to model persisted account after

**Acceptance Criteria:**

**Account Data Model:**
- [ ] `Account` struct defined (distinct from PersistedAccount for UI layer):
  - `id: String` (UUID v4, immutable after creation)
  - `name: String` (display name, editable)
  - `jwt_token: String` (sensitive, masked in UI)
  - `signing_key: String` (sensitive, masked in UI, multi-line capable)
  - `created_at: DateTime<Utc>`
  - `updated_at: DateTime<Utc>`
- [ ] `Account::new(name, jwt_token, signing_key)` constructor
- [ ] `Account::validate()` method that checks:
  - Name is non-empty and <= 50 chars
  - JWT token is non-empty and looks like JWT (three base64 parts separated by dots)
  - Signing key is non-empty (format validation optional, can accept PEM or base64)

**Account Form Component:**
- [ ] `AccountForm` struct for modal dialog:
  - `mode: FormMode` (Create, Edit)
  - `account: Account` (the account being edited, cloned for form)
  - `name_input: Input` (tui-input)
  - `jwt_input: Input` (tui-input)
  - `key_textarea: TextArea` (tui-textarea, 5 lines tall)
  - `error_message: Option<String>` (validation errors)
- [ ] `render()` method:
  - Centered dialog with border and title ("Create Account" or "Edit Account")
  - Label "Name:" followed by name_input.render()
  - Label "JWT Token:" followed by jwt_input.render()
  - Label "Signing Key:" followed by key_textarea.render()
  - Error message displayed in red if present
  - Help text at bottom: "Tab to switch fields, Enter to save, Esc to cancel"
- [ ] `handle_key(event: KeyEvent)` method:
  - Tab: Cycle through fields (name → jwt → key → name)
  - Enter: Validate, if valid emit `AccountFormSubmit` event, if invalid show error
  - Esc: Emit `AccountFormCancel` event
  - Character keys: Delegated to active input field
- [ ] `validate()` method that calls `account.validate()` and returns any errors

**Account CRUD Operations:**
- [ ] `AccountService` struct that wraps `Storage`:
  - `storage: Arc<Storage>`
  - `event_tx: mpsc::Sender<AppEvent>` (to notify UI of changes)
- [ ] `create_account(form: AccountForm)` → `Result<Account>`:
  - Validate form data
  - Create Account from form
  - Call `storage.create_account()`
  - Emit `AppEvent::AccountCreated(account)`
  - Return created account
- [ ] `update_account(id: &str, form: AccountForm)` → `Result<Account>`:
  - Find existing account
  - Validate form data
  - Apply updates (preserve created_at, update updated_at)
  - Call `storage.update_account()`
  - Emit `AppEvent::AccountUpdated(account)`
  - Return updated account
- [ ] `delete_account(id: &str)` → `Result<()>`:
  - Find account
  - Check for associated tasks (if any, return error with message "Cannot delete account with active tasks")
  - Call `storage.delete_account()`
  - Emit `AppEvent::AccountDeleted(id)`
- [ ] `list_accounts()` → `Vec<Account>`: Return all accounts from storage cache
- [ ] `get_account(id: &str)` → `Option<Account>`: Return single account by ID

**UI Integration:**
- [ ] Sidebar subscribes to `AppEvent::AccountCreated/Updated/Deleted` and refreshes
- [ ] Account detail view shows masked credentials:
  - JWT token shown as "eyJhbGci...last8chars"
  - Signing key shown as "-----BEGIN...last3lines"
  - "Reveal" option (F4) to temporarily show full credentials with warning
- [ ] Menu shortcuts:
  - F2: Switch sidebar to Accounts mode
  - 'n' (in Accounts mode): Open create account form
  - 'e' (with account selected): Open edit account form
  - 'd' (with account selected): Open delete confirmation dialog
- [ ] Delete confirmation dialog:
  - "Delete account 'Test Account'?"
  - "This action cannot be undone."
  - "Associated tasks: 3 (will also be deleted)" (if applicable)
  - Options: [y] Yes [n] No

**Commit**: YES
- Message: `feat(accounts): implement account CRUD with in-TUI forms`
- Files: `src/state/account.rs`, `src/ui/forms/account_form.rs`, `src/app/account_service.rs`
- Pre-commit: `cargo test --package standx-point-mm-strategy account::`

---

### Phase 6: Task CRUD with Start/Stop Controls

**What to do:**
- Implement Task CRUD similar to Account CRUD but with additional complexity:
  - Tasks reference accounts (foreign key relationship)
  - Tasks have lifecycle states (Stopped, Starting, Running, Stopping, Failed)
  - Tasks can be started/stopped through TUI controls
- Create task form with all configuration fields (symbol, risk params, sizing)
- Implement task lifecycle management integration with existing TaskManager
- Create task detail view showing configuration, runtime metrics, and controls
- Wire up menu shortcuts (F3 for tasks, 's' to start, 'x' to stop, etc.)

**Must NOT do:**
- Do NOT duplicate TaskConfig structure (extend or wrap it for persistence)
- Do NOT allow editing task ID after creation (it's a foreign key in runtime)
- Do NOT allow deleting a running task (must stop first)
- Do NOT block UI thread during task start/stop (use async integration)
- Do NOT lose existing TaskManager integration (bridge to it, don't replace)

**Recommended Agent Profile:**
- **Category**: ultrabrain
  - Reason: Complex integration with existing TaskManager and async lifecycle
- **Skills**: ["ratatui", "rust"]
  - ratatui: Forms and dialogs for task CRUD
  - rust: Async integration with TaskManager
- **Skills Evaluated but Omitted**:
  - None

**Parallelization:**
- **Can Run In Parallel**: NO
- **Parallel Group**: Sequential - Phase 6
- **Blocks**: Phase 7 (Integration - needs both accounts and tasks working)
- **Blocked By**: Phase 5 (Account CRUD - similar patterns to follow)

**References:**
- `src/task.rs:49-56` - TaskState enum (use for UI state display)
- `src/task.rs:59-69` - TaskManager structure (integrate with for start/stop)
- `src/config.rs:17-30` - TaskConfig structure (model persisted task after this)
- `src/task.rs:100-105` - spawn_from_config pattern (adapt for TUI-driven spawn)

**Why Each Reference Matters:**
- TaskState: Shows runtime task states - display these in TUI with appropriate colors
- TaskManager: Has spawn/shutdown methods - bridge TUI controls to these methods
- TaskConfig: Shows all task configuration fields - include all in task form
- spawn_from_config: Shows how tasks are spawned from config - adapt to spawn from persisted task

**Acceptance Criteria:**

**Task Data Model:**
- [ ] `Task` struct defined (distinct from TaskConfig for UI layer):
  - `id: String` (user-provided, like "btc-mm-1", unique across all tasks)
  - `symbol: String` (e.g., "BTC-USD")
  - `account_id: String` (foreign key to Account)
  - `risk_level: String` ("conservative", "moderate", "aggressive")
  - `max_position_usd: String` (decimal as string)
  - `price_jump_threshold_bps: u32`
  - `base_qty: String`
  - `tiers: u8`
  - `state: TaskState` (Stopped, Starting, Running, Stopping, Failed(String))
  - `created_at: DateTime<Utc>`
  - `updated_at: DateTime<Utc>`
  - `runtime_metrics: Option<TaskRuntimeMetrics>` (populated when running)
- [ ] `TaskRuntimeMetrics` struct:
  - `started_at: DateTime<Utc>`
  - `uptime_seconds: u64`
  - `orders_placed: u64`
  - `orders_filled: u64`
  - `volume_traded: Decimal`
  - `pnl: Decimal`
  - `last_error: Option<String>`
- [ ] `Task::new(id, symbol, account_id, ...)` constructor
- [ ] `Task::validate()` method that checks:
  - ID is non-empty, alphanumeric with hyphens, unique (check storage)
  - Symbol matches pattern [A-Z]{2,5}-[A-Z]{3} (e.g., BTC-USD)
  - Account ID references existing account
  - Risk level is one of: "conservative", "moderate", "aggressive"
  - Numeric fields are valid decimals (use rust_decimal parsing)
  - Tiers is 1, 2, or 3

**Task Form Component:**
- [ ] `TaskForm` struct for modal dialog:
  - `mode: FormMode` (Create, Edit)
  - `task: Task` (the task being edited, cloned for form)
  - `id_input: Input` (tui-input, only editable in Create mode)
  - `symbol_input: Input`
  - `account_selector: ListState` (dropdown-like selection)
  - `risk_selector: ListState` (dropdown for risk levels)
  - `max_pos_input: Input`
  - `price_jump_input: Input`
  - `base_qty_input: Input`
  - `tiers_input: Input`
  - `error_message: Option<String>`
  - `focused_field: usize` (0-8 for tab navigation)
- [ ] `render()` method:
  - Centered dialog with border and title ("Create Task" or "Edit Task")
  - Two-column layout if terminal wide enough:
    - Left column: ID, Symbol, Account, Risk Level
    - Right column: Max Position, Price Jump, Base Qty, Tiers
  - Or single-column layout for narrow terminals
  - Error message displayed in red at bottom if present
  - Help text: "Tab to switch fields, Enter to save, Esc to cancel"
- [ ] `handle_key(event: KeyEvent)` method:
  - Tab: Increment focused_field (wrap around)
  - Shift+Tab: Decrement focused_field
  - Enter: Validate, if valid emit `TaskFormSubmit`, if invalid show error
  - Esc: Emit `TaskFormCancel`
  - Character keys: Delegated to focused input field
  - Up/Down: If account or risk selector focused, change selection
- [ ] `validate()` method:
  - Call `task.validate()`
  - Additional form-specific validation:
    - Confirm account ID exists in storage
    - Confirm ID is unique (for Create mode)
  - Return first error encountered

**Task CRUD Service:**
- [ ] `TaskService` struct that wraps `Storage` and integrates with `TaskManager`:
  - `storage: Arc<Storage>`
  - `task_manager: Arc<Mutex<TaskManager>>`
  - `market_data_hub: Arc<Mutex<MarketDataHub>>`
  - `event_tx: mpsc::Sender<AppEvent>`
  - `active_tasks: HashMap<String, TaskHandle>` (runtime task handles)
- [ ] `create_task(form: TaskForm)` → `Result<Task>`:
  - Validate form
  - Create Task from form (state = Stopped)
  - Call `storage.create_task()`
  - Emit `AppEvent::TaskCreated(task)`
  - Return created task
- [ ] `update_task(id: &str, form: TaskForm)` → `Result<Task>`:
  - Find existing task
  - Verify task is not Running (cannot edit running task - must stop first)
  - Validate form
  - Apply updates (preserve id, created_at, update updated_at)
  - Call `storage.update_task()`
  - Emit `AppEvent::TaskUpdated(task)`
- [ ] `delete_task(id: &str)` → `Result<()>`:
  - Find task
  - Verify task is not Running (cannot delete running task - must stop first)
  - Call `storage.delete_task()`
  - Emit `AppEvent::TaskDeleted(id)`
- [ ] `list_tasks()` → `Vec<Task>`: Return all tasks from storage
- [ ] `get_task(id: &str)` → `Option<Task>`: Return single task

**Task Lifecycle (Start/Stop):**
- [ ] `start_task(id: &str)` → `Result<()>`:
  - Find task by ID
  - Verify task is in Stopped state (not already running)
  - Find associated account
  - Update task state to Starting
  - Emit `AppEvent::TaskStatusChanged(id, Starting)`
  - Build TaskConfig from persisted task + account credentials
  - Call `TaskManager::spawn_from_config()` with the task config
  - Store returned JoinHandle in `active_tasks` HashMap
  - Monitor task in background:
    - Spawn async task that awaits the JoinHandle
    - On completion: Update task state to Stopped or Failed
    - On error: Update task state to Failed with error message
  - Update task state to Running when task confirms startup
  - Emit `AppEvent::TaskStatusChanged(id, Running)`
- [ ] `stop_task(id: &str)` → `Result<()>`:
  - Find task by ID
  - Verify task is in Running state
  - Get JoinHandle from `active_tasks`
  - Update task state to Stopping
  - Emit `AppEvent::TaskStatusChanged(id, Stopping)`
  - Call `TaskManager::shutdown_and_wait()` for this specific task
  - Await task shutdown with timeout (30 seconds)
  - Remove from `active_tasks`
  - Update task state to Stopped
  - Emit `AppEvent::TaskStatusChanged(id, Stopped)`
- [ ] `force_stop_task(id: &str)` → `Result<()>` (for when graceful shutdown fails):
  - Abort the task JoinHandle
  - Update state to Stopped
  - Log warning about force stop

**Task Detail View:**
- [ ] Rich task detail view in right pane:
  - Header: Task ID, current state (color-coded: green=Running, gray=Stopped, red=Failed)
  - Configuration section: All task parameters (symbol, risk level, position limits, etc.)
  - Account section: Linked account name (clickable to view account)
  - Runtime section (only when running):
    - Uptime: "2 hours 15 minutes"
    - Orders: Placed 150, Filled 45
    - Volume: $125,000
    - PnL: +$234.56 (green) or -$123.45 (red)
    - Last error: None (or error message if failed)
  - Action bar at bottom:
    - [Start (s)] - Shown when Stopped
    - [Stop (x)] - Shown when Running
    - [Edit (e)] - Shown when Stopped
    - [Delete (d)] - Shown when Stopped

**Menu Integration:**
- [ ] F3 key switches sidebar to "Tasks" mode (shows tasks instead of accounts)
- [ ] When in Tasks mode:
  - 'n': Open new task form
  - 'e' (with task selected): Open edit task form
  - 'd' (with task selected): Open delete confirmation
  - 's' (with task selected): Start task
  - 'x' (with task selected): Stop task
  - Enter: View task details in right pane

**Commit**: YES
- Message: `feat(tasks): implement task CRUD with start/stop lifecycle`
- Files: `src/state/task.rs`, `src/ui/forms/task_form.rs`, `src/app/task_service.rs`
- Pre-commit: `cargo test --package standx-point-mm-strategy task::`

---

### Phase 6: Keyboard Navigation and Menu System

**What to do:**
- Implement comprehensive keyboard shortcuts for all operations
- Create help overlay showing all available shortcuts (F1 key)
- Implement vi-like navigation (j/k for up/down, h/l for sidebar/detail)
- Add Tab/Shift+Tab navigation for forms
- Create menu highlighting (show active menu item)
- Implement command mode (':' for commands like Vim)
- Add visual feedback for key presses (flash status message)

**Must NOT do:**
- Do NOT require mouse (all operations must be keyboard-accessible)
- Do NOT use single-key shortcuts that could be typed accidentally (use F-keys + letters)
- Do NOT skip the help system (users must be able to discover shortcuts)
- Do NOT make navigation modal without clear visual indicator
- Do NOT conflict with terminal shortcuts (avoid Ctrl+C, Ctrl+Z for in-app use)

**Recommended Agent Profile:**
- **Category**: visual-engineering
  - Reason: Keyboard navigation is a UX design problem
- **Skills**: ["ratatui"]
  - ratatui: Event handling, key event patterns

**Parallelization:**
- **Can Run In Parallel**: NO
- **Parallel Group**: Sequential - Phase 6
- **Blocks**: Phase 7 (Integration Testing - need all features for E2E tests)
- **Blocked By**: Phase 5 (Task CRUD - keyboard shortcuts for tasks)

**References:**
- Ratatui skill basics: `./skills/basics/SKILL.md` - Event handling patterns
- crossterm docs: `KeyEvent`, `KeyCode`, `KeyModifiers` for key handling
- Vim/Neovim documentation for vi-like navigation patterns (hjkl, gg, G)

**Acceptance Criteria:**

**Global Shortcuts (Always Active):**
- [x] F1: Open help overlay showing all shortcuts
- [x] F2: Switch sidebar to Accounts mode
- [x] F3: Switch sidebar to Tasks mode
- [x] F4: Toggle credentials visibility (show/hide sensitive data)
- [x] q: Quit application (with confirmation if tasks running)
- [ ] Ctrl+C: Force quit (same as 'q' but immediate)

**Navigation Shortcuts (Normal Mode):**
- [x] j or ↓: Move selection down in sidebar
- [x] k or ↑: Move selection up in sidebar
- [x] h or ←: Focus sidebar (when in detail view)
- [x] l or →: Focus detail view (when in sidebar)
- [x] gg: Jump to first item in sidebar
- [x] G: Jump to last item in sidebar
- [ ] Enter: Open selected item in detail view
- [x] Tab: Cycle focus between sidebar → detail view → menu bar

**Account Mode Shortcuts (F2 active):**
- [ ] n: New account (open create form)
- [ ] e: Edit selected account (open edit form)
- [ ] d: Delete selected account (open confirmation dialog)
- [ ] v: View account details (same as Enter)

**Task Mode Shortcuts (F3 active):**
- [ ] n: New task (open create form)
- [ ] e: Edit selected task (open edit form)
- [ ] d: Delete selected task (open confirmation dialog)
- [ ] s: Start selected task (if Stopped)
- [ ] x: Stop selected task (if Running)
- [ ] v: View task details (same as Enter)

**Form Navigation (Form Mode):**
- [ ] Tab: Move to next field
- [ ] Shift+Tab: Move to previous field
- [ ] Enter: Submit form (if valid)
- [ ] Esc: Cancel form
- [ ] Ctrl+A: Select all text in current field
- [ ] Ctrl+K: Clear current field

**Help System:**
- [ ] F1 opens full-screen help overlay
- [ ] Help shows all shortcuts organized by category (Global, Navigation, Account, Task, Form)
- [ ] Each shortcut shows key combination and description
- [ ] Help is scrollable if terminal is small
- [x] Press F1 again or Esc to close help
- [ ] Context-sensitive help: when in a form, F1 shows form-specific help

**Visual Feedback:**
- [x] Status bar shows "Key pressed: j" briefly when keys are pressed (flash for 500ms)
- [ ] Invalid key combinations show error message in status bar (e.g., "Cannot start task: already running")
- [ ] Mode indicator in status bar: "-- NORMAL --", "-- INSERT --", "-- FORM --"
- [x] When waiting for async operation (e.g., task starting), show spinner in status bar

**Commit**: YES
- Message: `feat(navigation): implement keyboard shortcuts and help system`
- Files: `src/ui/navigation.rs`, `src/ui/help.rs`, `src/app/keybindings.rs`
- Pre-commit: `cargo test --package standx-point-mm-strategy` and manual test of all shortcuts

---

### Phase 7: Integration Testing and Polish

**What to do:**
- Write comprehensive integration tests for TUI flows
- Test account CRUD end-to-end (create → edit → delete)
- Test task lifecycle (create → start → run → stop → delete)
- Test graceful shutdown with running tasks
- Test keyboard navigation and shortcuts
- Test terminal resize handling
- Performance testing (TUI remains responsive with 100+ tasks)
- Add logging for TUI operations (for debugging)
- Create user documentation (README section on TUI usage)

**Must NOT do:**
- Do NOT skip testing error paths (validation failures, disk full, etc.)
- Do NOT leave TODO comments without issue tracking
- Do NOT skip documentation (users need to know how to use the TUI)
- Do NOT ignore performance (TUI must stay responsive with many tasks)
- Do NOT skip backward compatibility testing (existing CLI must still work)

**Recommended Agent Profile:**
- **Category**: deep
  - Reason: Integration testing requires thoroughness and edge case exploration
- **Skills**: ["ratatui"]
  - ratatui: TestBackend for TUI testing
- **Skills Evaluated but Omitted**:
  - None

**Parallelization:**
- **Can Run In Parallel**: NO
- **Parallel Group**: Sequential - Phase 7 (final phase)
- **Blocks**: Nothing (final phase)
- **Blocked By**: Phase 6 (Navigation - all features must be complete for testing)

**References:**
- Ratatui test docs: `TestBackend` for testing TUI rendering
- `tests/` directory: Existing integration tests (maintain compatibility)
- `src/main.rs` CLI mode: Ensure backward compatibility with --config flag

**Why Each Reference Matters:**
- Ratatui test docs: Shows how to write automated tests for TUI components
- tests/ directory: Must maintain existing test compatibility
- main.rs: Ensure existing CLI mode still works (TUI is additive, not replacement)

**Acceptance Criteria:**

**Integration Tests:**
- [ ] Test: TUI startup without --config shows TUI mode (not CLI error)
- [ ] Test: CLI mode with --config still works (backward compatibility)
- [ ] Test: Account CRUD flow:
  - Create account → Verify appears in sidebar
  - Edit account → Verify changes saved
  - Delete account → Verify removed from sidebar
- [ ] Test: Task CRUD flow:
  - Create task → Verify appears in sidebar
  - Start task → Verify state changes to Running
  - Stop task → Verify state changes to Stopped
  - Delete task → Verify removed from sidebar
- [ ] Test: Keyboard navigation:
  - F1 opens help
  - F2/F3 switch sidebar modes
  - j/k move selection
  - Enter opens detail view
  - q quits application
- [ ] Test: Graceful shutdown:
  - Start multiple tasks
  - Press 'q' to quit
  - Verify all tasks stopped cleanly
  - Verify no orphaned processes
- [ ] Test: Terminal resize:
  - Start TUI
  - Resize terminal to 40x10 (small)
  - Verify "Terminal too small" message shown
  - Resize to 120x40 (large)
  - Verify normal UI restored
- [ ] Test: Error handling:
  - Try creating account with duplicate ID → Verify error shown
  - Try deleting account with tasks → Verify error shown
  - Try starting task with invalid symbol → Verify error shown
- [ ] Test: Performance:
  - Create 100 accounts
  - Create 100 tasks
  - Verify TUI remains responsive (keyboard input processed within 100ms)
  - Verify sidebar scrolls smoothly

**Manual Testing Checklist:**
- [ ] TUI starts and shows correct layout
- [ ] Can create account with form
- [ ] Can edit account
- [ ] Can delete account
- [ ] Can create task with form
- [ ] Can start task and see it running
- [ ] Can stop task
- [ ] Can delete task
- [ ] Can navigate with keyboard only
- [ ] Help screen shows all shortcuts
- [ ] Graceful shutdown works
- [ ] CLI mode with --config still works
- [ ] No crashes or panics

**Documentation:**
- [ ] README section added: "Using the TUI"
- [ ] Description of layout (sidebar, detail view, status bar, menu)
- [ ] List of all keyboard shortcuts with descriptions
- [ ] Step-by-step guide: Creating your first account and task
- [ ] Explanation of task lifecycle (start, run, stop)
- [ ] Troubleshooting section (common issues and solutions)

**Performance Benchmarks:**
- [ ] Startup time: TUI shows first frame within 500ms
- [ ] Input latency: Key press to action < 100ms
- [ ] Frame rate: Consistent 60 FPS during normal operation
- [ ] Memory usage: < 100 MB for 100 accounts + 100 tasks
- [ ] Scroll performance: Smooth scrolling in sidebar with 1000 items

**Logging:**
- [ ] TUI operations logged at INFO level:
  - "Creating account: Test Account"
  - "Task started: btc-mm-1"
  - "Shutting down TUI"
- [ ] Errors logged at ERROR level with context
- [ ] Debug logging available with RUST_LOG=debug

**Commit**: YES
- Message: `feat(integration): add tests, docs, and polish for TUI`
- Files: `tests/tui_integration_tests.rs`, `README.md` (TUI section), logging improvements
- Pre-commit: `cargo test --package standx-point-mm-strategy --test tui_integration_tests` and `cargo doc --package standx-point-mm-strategy`

---

## Success Criteria

### Verification Commands
```bash
# Build verification
cargo build --package standx-point-mm-strategy --release

# Test verification
cargo test --package standx-point-mm-strategy

# TUI startup verification
cargo run --package standx-point-mm-strategy
# (should show TUI, not error about missing --config)

# CLI backward compatibility
cargo run --package standx-point-mm-strategy -- --config examples/single_task.yaml --dry-run
# (should work as before)

# Clippy linting
cargo clippy --package standx-point-mm-strategy -- -D warnings

# Documentation build
cargo doc --package standx-point-mm-strategy --no-deps
```

### Final Checklist
- [ ] All "Must Have" features present and working
- [ ] All "Must NOT Have" guardrails respected (no violations)
- [ ] All tests pass (`cargo test` succeeds)
- [ ] TUI mode works (start without --config)
- [ ] CLI backward compatibility maintained (--config flag works)
- [ ] No compiler warnings or clippy lints
- [ ] Documentation complete (README section + inline docs)
- [ ] Code review ready (clean, idiomatic Rust)

---

## Final Choice

### How to Proceed

Plan is ready. Two options for execution:

**Option 1: Start Work (Recommended)**
- Run `/start-work` to begin execution
- Sisyphus will execute tasks sequentially
- You'll get progress updates after each phase

**Option 2: High Accuracy Review**
- Submit plan for Momus review
- Rigorous verification of all details
- Adds review loop but guarantees precision

### Recommendation

**Start with Option 1 (Start Work)**. This plan is comprehensive and ready for execution. The sequential nature of the phases means each builds on the last, so starting immediately is the most efficient path.

If at any point you need to pause, the `/stop-work` command will save progress and you can resume later with `/start-work`.

---

**Plan Location**: `.sisyphus/plans/ratatui-tui-refactor.md`

**Draft Location**: `.sisyphus/drafts/ratatui-tui-refactor-draft.md` (will be deleted after plan execution begins)
