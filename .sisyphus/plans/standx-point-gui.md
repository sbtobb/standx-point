# StandX Point GUI - Desktop Trading Dashboard

## TL;DR

> **Quick Summary**: Create a GPUI-based desktop application for managing trading tasks with real-time price monitoring, multi-account support, and SQLite persistence.
> 
> **Deliverables**:
> - New crate `standx-point-gui` with GPUI framework
> - Left sidebar with task card list (CRUD operations)
> - Right panel for task details (account info, positions, orders, history)
> - Top status bar with real-time prices and task statistics
> - SQLite database for persistence (encrypted credentials)
> 
> **Estimated Effort**: Large
> **Parallel Execution**: YES - 3 waves
> **Critical Path**: Task 1 → Task 3 → Task 5 → Task 8 → Task 11

---

## Context

### Original Request
Create `standx-point-gui` crate with:
1. GPUI + gpui-component framework
2. Left sidebar with task card list, right panel for task details
3. Top status bar with real-time prices and task statistics
4. Task CRUD, start/stop controls, status display
5. Account info: address, alias, positions, orders, trade history
6. SQLite storage

### Interview Summary
**Key Discussions**:
- Task config: Reuse mm-strategy's TaskConfig structure
- Account management: Multi-account support (max 5 in MVP)
- Price data: Reuse adapter's StandxWebSocket for real-time updates
- Storage: SQLite with encrypted credentials
- Task execution: GUI independent (directly call adapter API)
- Crash recovery: Manual recovery (tasks marked as paused)
- MVP scope: System tray and notifications deferred to Phase 2

**Research Findings**:
- Adapter provides: StandxClient, StandxWebSocket, Order, Position, Balance, Trade types
- GPUI patterns: Render trait for views, RenderOnce for components, Entity<T> for state
- gpui-component: Sidebar, List, Table, Button, Input, Modal components available

### Metis Review
**Identified Gaps** (addressed):
- Task state machine: Defined states (Draft → Pending → Running → Paused → Stopped)
- Credential security: Encrypted SQLite storage confirmed
- WebSocket source: Reuse adapter's StandxWebSocket
- Test automation: Manual QA only (no HTTP API)
- Crash recovery: Manual recovery policy confirmed

---

## Work Objectives

### Core Objective
Build a desktop GUI application using GPUI framework that provides a visual interface for managing trading tasks, monitoring real-time prices, and viewing account information.

### Concrete Deliverables
- `crates/standx-point-gui/` - New crate with GPUI application
- `crates/standx-point-gui/src/main.rs` - Application entry point
- `crates/standx-point-gui/src/db/` - SQLite database layer
- `crates/standx-point-gui/src/state/` - Application state management
- `crates/standx-point-gui/src/ui/` - UI components (sidebar, cards, panels)
- `crates/standx-point-gui/src/task/` - Task execution engine

### Definition of Done
- [x] Application launches and displays main window
- [x] Can create, edit, delete task configurations
- [x] Can start/stop tasks from UI
- [x] Real-time price updates displayed in status bar
- [x] Account positions and orders displayed in detail panel
- [x] Data persisted to SQLite across restarts

### Must Have
- GPUI 0.2.2 + gpui-component 0.5.0 framework
- Sidebar + main content layout with resizable panels
- Task card list with CRUD operations
- Task state machine (Draft → Pending → Running → Paused → Stopped)
- Real-time price display via WebSocket
- Account info display (address, positions, orders, trades)
- SQLite persistence with encrypted credentials
- Async task execution (non-blocking UI)

### Must NOT Have (Guardrails)
- ❌ Multi-exchange support (StandX only)
- ❌ Strategy parameter editing UI (use config files)
- ❌ System tray support (Phase 2)
- ❌ Desktop notifications (Phase 2)
- ❌ Backtesting/simulation mode
- ❌ P&L analytics/charts
- ❌ Export to CSV/Excel
- ❌ Dark/light theme switching
- ❌ Internationalization (i18n)
- ❌ Auto-update mechanism
- ❌ Plaintext credential storage
- ❌ Blocking UI thread during API calls

---

## Verification Strategy (MANDATORY)

> **UNIVERSAL RULE: ZERO HUMAN INTERVENTION**
>
> ALL tasks in this plan MUST be verifiable WITHOUT any human action.
> This is NOT conditional — it applies to EVERY task, regardless of test strategy.

### Test Decision
- **Infrastructure exists**: NO (new crate)
- **Automated tests**: None (manual QA only per user decision)
- **Framework**: N/A

### Agent-Executed QA Scenarios (MANDATORY — ALL tasks)

> Since this is a native GPUI application without HTTP API, QA scenarios will use:
> - **Bash**: Build verification, process checks, file existence
> - **interactive_bash (tmux)**: Launch app, observe startup behavior
> - **File inspection**: SQLite database verification, config file checks

**Verification Tool by Deliverable Type:**

| Type | Tool | How Agent Verifies |
|------|------|-------------------|
| **Build** | Bash (cargo build) | Compile without errors, binary exists |
| **App Launch** | interactive_bash (tmux) | Process starts, no crash in first 5s |
| **Database** | Bash (sqlite3) | Schema exists, tables created |
| **Config** | Bash (file checks) | Config files in expected locations |

**Evidence Requirements:**
- Build output captured for compilation tasks
- Process list for app launch verification
- SQLite schema dumps for database tasks
- All evidence in `.sisyphus/evidence/`

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately):
├── Task 1: Crate scaffolding + Cargo.toml
├── Task 2: SQLite schema design
└── Task 3: Application state types

Wave 2 (After Wave 1):
├── Task 4: Database layer implementation [depends: 2]
├── Task 5: GPUI app skeleton + main window [depends: 1, 3]
└── Task 6: Task state machine [depends: 3]

Wave 3 (After Wave 2):
├── Task 7: Sidebar + task card list UI [depends: 5]
├── Task 8: Task execution engine [depends: 4, 6]
├── Task 9: WebSocket price stream [depends: 5]
└── Task 10: Account info panel [depends: 5]

Wave 4 (After Wave 3):
├── Task 11: Task CRUD operations [depends: 4, 7, 8]
├── Task 12: Status bar with prices [depends: 9]
└── Task 13: Task detail panel [depends: 10, 11]

Wave 5 (Final):
└── Task 14: Integration + polish [depends: 11, 12, 13]

Critical Path: Task 1 → Task 5 → Task 7 → Task 11 → Task 14
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 1 | None | 5 | 2, 3 |
| 2 | None | 4 | 1, 3 |
| 3 | None | 5, 6 | 1, 2 |
| 4 | 2 | 8, 11 | 5, 6 |
| 5 | 1, 3 | 7, 9, 10 | 4, 6 |
| 6 | 3 | 8 | 4, 5 |
| 7 | 5 | 11 | 8, 9, 10 |
| 8 | 4, 6 | 11 | 7, 9, 10 |
| 9 | 5 | 12 | 7, 8, 10 |
| 10 | 5 | 13 | 7, 8, 9 |
| 11 | 4, 7, 8 | 13, 14 | 12 |
| 12 | 9 | 14 | 11, 13 |
| 13 | 10, 11 | 14 | 12 |
| 14 | 11, 12, 13 | None | None (final) |

### Agent Dispatch Summary

| Wave | Tasks | Recommended Approach |
|------|-------|---------------------|
| 1 | 1, 2, 3 | Parallel: 3 independent foundation tasks |
| 2 | 4, 5, 6 | Parallel: Core infrastructure |
| 3 | 7, 8, 9, 10 | Parallel: UI + Engine components |
| 4 | 11, 12, 13 | Parallel: Feature integration |
| 5 | 14 | Sequential: Final polish |

---

## TODOs

### Wave 1: Foundation (Parallel)

- [x] 1. Crate Scaffolding + Cargo.toml

  **What to do**:
  - Create `crates/standx-point-gui/` directory structure
  - Create `Cargo.toml` with dependencies: gpui, gpui-component, gpui-component-assets, standx-point-adapter, rusqlite, tokio, serde, anyhow
  - Create `src/main.rs` with minimal GPUI app skeleton
  - Add crate to workspace `Cargo.toml`
  - Verify build passes

  **Must NOT do**:
  - Add unnecessary dependencies
  - Implement any business logic yet

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple file creation and configuration
  - **Skills**: [`fractal-context`]
    - `fractal-context`: Create proper AGENTS.md for new crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2, 3)
  - **Blocks**: Task 5
  - **Blocked By**: None

  **References**:
  - `crates/standx-point-mm-strategy/Cargo.toml` - Workspace dependency pattern
  - `Cargo.toml:1-9` - Workspace member registration pattern

  **Acceptance Criteria**:

  ```
  Scenario: Crate builds successfully
    Tool: Bash
    Steps:
      1. cargo build -p standx-point-gui
      2. Assert: exit code 0
      3. Assert: no compilation errors in output
    Expected Result: Build succeeds
    Evidence: .sisyphus/evidence/task-1-build.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): scaffold standx-point-gui crate with GPUI dependencies`
  - Files: `crates/standx-point-gui/`, `Cargo.toml`

---

- [x] 2. SQLite Schema Design

  **What to do**:
  - Create `crates/standx-point-gui/src/db/schema.sql` with tables:
    - `accounts` (id, address, alias, encrypted_jwt, encrypted_signing_key, chain, created_at)
    - `tasks` (id, account_id, name, symbol, config_json, status, created_at, updated_at)
    - `order_history` (id, task_id, order_id, symbol, side, price, qty, status, created_at)
    - `trade_history` (id, task_id, trade_id, order_id, symbol, side, price, qty, fee, pnl, created_at)
    - `operation_logs` (id, task_id, action, details, created_at)
  - Add indexes for common queries
  - Document schema in comments

  **Must NOT do**:
  - Implement database access code (Task 4)
  - Store credentials in plaintext

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: SQL schema design is straightforward
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3)
  - **Blocks**: Task 4
  - **Blocked By**: None

  **References**:
  - `crates/standx-point-adapter/src/types/models.rs:49-172` - Order, Position, Balance, Trade field definitions
  - `crates/standx-point-mm-strategy/src/config.rs` - TaskConfig structure

  **Acceptance Criteria**:

  ```
  Scenario: Schema file exists and is valid SQL
    Tool: Bash
    Steps:
      1. cat crates/standx-point-gui/src/db/schema.sql
      2. Assert: file exists
      3. Assert: contains CREATE TABLE accounts
      4. Assert: contains CREATE TABLE tasks
      5. Assert: contains CREATE TABLE order_history
      6. Assert: contains CREATE TABLE trade_history
      7. Assert: contains CREATE TABLE operation_logs
    Expected Result: All tables defined
    Evidence: .sisyphus/evidence/task-2-schema.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): add SQLite schema for accounts, tasks, and history`
  - Files: `crates/standx-point-gui/src/db/schema.sql`

---

- [x] 3. Application State Types

  **What to do**:
  - Create `crates/standx-point-gui/src/state/mod.rs`
  - Define core state types:
    - `AppState` - Global application state
    - `Account` - Account with encrypted credentials
    - `Task` - Task configuration and runtime state
    - `TaskStatus` enum (Draft, Pending, Running, Paused, Stopped, Failed)
    - `PriceData` - Real-time price information
  - Implement `Default` and `Clone` where appropriate
  - Add serde derives for persistence

  **Must NOT do**:
  - Implement state mutation logic
  - Add UI-specific code

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Type definitions are straightforward
  - **Skills**: [`typescript-advanced-types`]
    - Note: Rust types, but similar patterns apply

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2)
  - **Blocks**: Tasks 5, 6
  - **Blocked By**: None

  **References**:
  - `crates/standx-point-adapter/src/types/models.rs` - Order, Position, Balance, Trade types
  - `crates/standx-point-adapter/src/types/enums.rs` - Side, OrderType, OrderStatus enums
  - `crates/standx-point-mm-strategy/src/config.rs` - TaskConfig, CredentialsConfig patterns

  **Acceptance Criteria**:

  ```
  Scenario: State types compile and are usable
    Tool: Bash
    Steps:
      1. cargo check -p standx-point-gui
      2. Assert: exit code 0
      3. grep -r "pub struct AppState" crates/standx-point-gui/src/
      4. Assert: AppState defined
      5. grep -r "pub enum TaskStatus" crates/standx-point-gui/src/
      6. Assert: TaskStatus enum defined
    Expected Result: Types defined and compile
    Evidence: .sisyphus/evidence/task-3-types.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): define application state types and TaskStatus enum`
  - Files: `crates/standx-point-gui/src/state/`

---

### Wave 2: Core Infrastructure (Parallel)

- [x] 4. Database Layer Implementation

  **What to do**:
  - Create `crates/standx-point-gui/src/db/mod.rs` with Database struct
  - Implement connection pool with rusqlite
  - Add migration runner to apply schema.sql
  - Implement CRUD operations for accounts, tasks, history
  - Add credential encryption/decryption using ring or aes-gcm
  - Enable WAL mode for crash safety

  **Must NOT do**:
  - Store credentials in plaintext
  - Block on database operations (use spawn_blocking)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Database layer with encryption requires careful implementation
  - **Skills**: [`supabase-postgres-best-practices`]
    - Applicable patterns for SQLite as well

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6)
  - **Blocks**: Tasks 8, 11
  - **Blocked By**: Task 2

  **References**:
  - `crates/standx-point-gui/src/db/schema.sql` - Schema to implement (from Task 2)
  - `crates/standx-point-mm-strategy/src/config.rs:CredentialsConfig` - Credential structure

  **Acceptance Criteria**:

  ```
  Scenario: Database initializes with schema
    Tool: Bash
    Steps:
      1. cargo build -p standx-point-gui
      2. Create test: initialize Database, verify tables exist
      3. sqlite3 test.db ".tables" | grep -E "accounts|tasks"
      4. Assert: tables exist
    Expected Result: Database layer functional
    Evidence: .sisyphus/evidence/task-4-db.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): implement SQLite database layer with encrypted credentials`
  - Files: `crates/standx-point-gui/src/db/`

---

- [x] 5. GPUI App Skeleton + Main Window

  **What to do**:
  - Update `src/main.rs` with full GPUI application setup
  - Initialize gpui-component theme
  - Create main window with h_resizable layout (sidebar + content)
  - Create `src/ui/mod.rs` for UI module organization
  - Create `src/ui/root.rs` with RootView implementing Render
  - Set up Entity<AppState> for global state

  **Must NOT do**:
  - Implement actual UI components (Tasks 7, 10, 12, 13)
  - Add business logic

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UI framework setup and layout
  - **Skills**: [`frontend-ui-ux`]
    - GPUI layout patterns

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 6)
  - **Blocks**: Tasks 7, 9, 10
  - **Blocked By**: Tasks 1, 3

  **References**:
  - GPUI docs: Application::new().run() pattern
  - gpui-component: h_resizable, Sidebar components

  **Acceptance Criteria**:

  ```
  Scenario: App launches and shows main window
    Tool: interactive_bash (tmux)
    Steps:
      1. cargo build --release -p standx-point-gui
      2. tmux new-session -d -s gui-test "./target/release/standx-point-gui"
      3. sleep 3
      4. pgrep -f standx-point-gui
      5. Assert: process exists
      6. tmux kill-session -t gui-test
    Expected Result: App starts without crash
    Evidence: .sisyphus/evidence/task-5-launch.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): implement GPUI app skeleton with resizable layout`
  - Files: `crates/standx-point-gui/src/main.rs`, `crates/standx-point-gui/src/ui/`

---

- [x] 6. Task State Machine

  **What to do**:
  - Create `crates/standx-point-gui/src/task/mod.rs`
  - Create `crates/standx-point-gui/src/task/state_machine.rs`
  - Implement TaskStateMachine with states: Draft, Pending, Running, Paused, Stopped, Failed
  - Define valid transitions:
    - Draft → Pending (on save)
    - Pending → Running (on start)
    - Running → Paused (on pause)
    - Running → Stopped (on stop)
    - Running → Failed (on error)
    - Paused → Running (on resume)
    - Paused → Stopped (on stop)
    - Any → Draft (on edit)
  - Add transition validation and error handling

  **Must NOT do**:
  - Implement actual task execution (Task 8)
  - Add UI code

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: State machine is well-defined pattern
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 5)
  - **Blocks**: Task 8
  - **Blocked By**: Task 3

  **References**:
  - `crates/standx-point-gui/src/state/mod.rs` - TaskStatus enum (from Task 3)
  - `crates/standx-point-mm-strategy/src/task.rs` - Task lifecycle patterns

  **Acceptance Criteria**:

  ```
  Scenario: State machine validates transitions
    Tool: Bash
    Steps:
      1. cargo check -p standx-point-gui
      2. grep -r "impl TaskStateMachine" crates/standx-point-gui/src/
      3. Assert: TaskStateMachine implemented
      4. grep -r "fn transition" crates/standx-point-gui/src/
      5. Assert: transition method exists
    Expected Result: State machine compiles
    Evidence: .sisyphus/evidence/task-6-state.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): implement task state machine with transition validation`
  - Files: `crates/standx-point-gui/src/task/`

---

### Wave 3: UI + Engine Components (Parallel)

- [x] 7. Sidebar + Task Card List UI

  **What to do**:
  - Create `crates/standx-point-gui/src/ui/sidebar.rs`
  - Create `crates/standx-point-gui/src/ui/task_card.rs`
  - Implement SidebarView with gpui-component Sidebar
  - Implement TaskCard component (RenderOnce) showing:
    - Task name and symbol
    - Status indicator (color-coded)
    - Start/Stop/Pause buttons
  - Implement task list with click-to-select behavior
  - Wire up selection to update main content panel

  **Must NOT do**:
  - Implement CRUD logic (Task 11)
  - Add complex animations

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UI component implementation
  - **Skills**: [`frontend-ui-ux`, `ui-ux-pro-max`]
    - GPUI component patterns

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 8, 9, 10)
  - **Blocks**: Task 11
  - **Blocked By**: Task 5

  **References**:
  - gpui-component: Sidebar, SidebarGroup, SidebarMenuItem
  - `crates/standx-point-gui/src/state/mod.rs` - Task, TaskStatus types

  **Acceptance Criteria**:

  ```
  Scenario: Sidebar renders with task cards
    Tool: Bash
    Steps:
      1. cargo build -p standx-point-gui
      2. grep -r "impl.*Render.*for SidebarView" crates/standx-point-gui/src/
      3. Assert: SidebarView implements Render
      4. grep -r "impl RenderOnce for TaskCard" crates/standx-point-gui/src/
      5. Assert: TaskCard implements RenderOnce
    Expected Result: UI components compile
    Evidence: .sisyphus/evidence/task-7-sidebar.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): implement sidebar with task card list UI`
  - Files: `crates/standx-point-gui/src/ui/sidebar.rs`, `crates/standx-point-gui/src/ui/task_card.rs`

---

- [x] 8. Task Execution Engine

  **What to do**:
  - Create `crates/standx-point-gui/src/task/executor.rs`
  - Implement TaskExecutor that:
    - Creates StandxClient from adapter
    - Manages task lifecycle (start, stop, pause)
    - Queries positions and orders periodically
    - Handles errors gracefully (no crash)
  - Use tokio::spawn for async execution
  - Implement graceful shutdown with order cancellation
  - Store execution state in database

  **Must NOT do**:
  - Implement market making strategy logic
  - Block UI thread

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Async execution with error handling
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 7, 9, 10)
  - **Blocks**: Task 11
  - **Blocked By**: Tasks 4, 6

  **References**:
  - `crates/standx-point-adapter/src/http/client.rs` - StandxClient usage
  - `crates/standx-point-mm-strategy/src/task.rs:271-451` - Client creation, order/position queries
  - `crates/standx-point-gui/src/task/state_machine.rs` - State transitions

  **Acceptance Criteria**:

  ```
  Scenario: Executor compiles with adapter integration
    Tool: Bash
    Steps:
      1. cargo build -p standx-point-gui
      2. grep -r "use standx_point_adapter" crates/standx-point-gui/src/task/
      3. Assert: adapter imported
      4. grep -r "StandxClient" crates/standx-point-gui/src/task/
      5. Assert: StandxClient used
    Expected Result: Executor integrates with adapter
    Evidence: .sisyphus/evidence/task-8-executor.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): implement async task execution engine with adapter integration`
  - Files: `crates/standx-point-gui/src/task/executor.rs`

---

- [x] 9. WebSocket Price Stream

  **What to do**:
  - Create `crates/standx-point-gui/src/price/mod.rs`
  - Implement PriceService that:
    - Uses adapter's StandxWebSocket for price subscriptions
    - Maintains tokio::sync::watch channels for price updates
    - Supports subscribing to multiple symbols (max 10)
    - Handles reconnection with exponential backoff
  - Integrate with AppState for UI updates

  **Must NOT do**:
  - Implement custom WebSocket client (use adapter)
  - Subscribe to more than 10 symbols

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Wrapper around existing adapter functionality
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 7, 8, 10)
  - **Blocks**: Task 12
  - **Blocked By**: Task 5

  **References**:
  - `crates/standx-point-adapter/src/ws/` - StandxWebSocket, PriceData types
  - `crates/standx-point-mm-strategy/src/market_data.rs` - Watch channel broadcast pattern

  **Acceptance Criteria**:

  ```
  Scenario: Price service compiles with WebSocket integration
    Tool: Bash
    Steps:
      1. cargo build -p standx-point-gui
      2. grep -r "StandxWebSocket" crates/standx-point-gui/src/price/
      3. Assert: WebSocket imported
      4. grep -r "watch::channel" crates/standx-point-gui/src/price/
      5. Assert: watch channel used
    Expected Result: Price service integrates with adapter
    Evidence: .sisyphus/evidence/task-9-price.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): implement WebSocket price stream service`
  - Files: `crates/standx-point-gui/src/price/`

---

- [x] 10. Account Info Panel

  **What to do**:
  - Create `crates/standx-point-gui/src/ui/account_panel.rs`
  - Implement AccountPanel view showing:
    - Account address and alias
    - Balance summary (equity, available, margin)
    - Position list (symbol, qty, entry price, PnL)
    - Open orders list (symbol, side, price, qty, status)
  - Use gpui-component Table for lists
  - Add refresh button to reload data

  **Must NOT do**:
  - Implement order placement UI
  - Add P&L charts

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UI component with data display
  - **Skills**: [`frontend-ui-ux`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 7, 8, 9)
  - **Blocks**: Task 13
  - **Blocked By**: Task 5

  **References**:
  - `crates/standx-point-adapter/src/types/models.rs:86-150` - Position, Balance types
  - gpui-component: Table component

  **Acceptance Criteria**:

  ```
  Scenario: Account panel renders with data tables
    Tool: Bash
    Steps:
      1. cargo build -p standx-point-gui
      2. grep -r "impl.*Render.*for AccountPanel" crates/standx-point-gui/src/
      3. Assert: AccountPanel implements Render
      4. grep -r "Table" crates/standx-point-gui/src/ui/account_panel.rs
      5. Assert: Table component used
    Expected Result: Account panel compiles
    Evidence: .sisyphus/evidence/task-10-account.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): implement account info panel with positions and orders`
  - Files: `crates/standx-point-gui/src/ui/account_panel.rs`

---

### Wave 4: Feature Integration (Parallel)

- [x] 11. Task CRUD Operations

  **What to do**:
  - Create `crates/standx-point-gui/src/ui/task_form.rs` - Modal form for task creation/editing
  - Implement create task flow:
    - Select account from dropdown
    - Enter task name and symbol
    - Configure risk/sizing parameters (JSON editor or simple fields)
    - Save to database
  - Implement edit task flow (only when task is stopped)
  - Implement delete task with confirmation dialog
  - Wire up sidebar buttons to CRUD operations

  **Must NOT do**:
  - Allow editing running tasks
  - Implement complex strategy configuration UI

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: Form UI with database integration
  - **Skills**: [`frontend-ui-ux`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 12, 13)
  - **Blocks**: Tasks 13, 14
  - **Blocked By**: Tasks 4, 7, 8

  **References**:
  - gpui-component: Modal, Input, Button, Select components
  - `crates/standx-point-gui/src/db/mod.rs` - Database CRUD methods

  **Acceptance Criteria**:

  ```
  Scenario: Task form modal renders
    Tool: Bash
    Steps:
      1. cargo build -p standx-point-gui
      2. grep -r "Modal" crates/standx-point-gui/src/ui/task_form.rs
      3. Assert: Modal component used
      4. grep -r "fn create_task" crates/standx-point-gui/src/
      5. Assert: create_task function exists
    Expected Result: CRUD operations implemented
    Evidence: .sisyphus/evidence/task-11-crud.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): implement task CRUD operations with modal form`
  - Files: `crates/standx-point-gui/src/ui/task_form.rs`

---

- [x] 12. Status Bar with Prices

  **What to do**:
  - Create `crates/standx-point-gui/src/ui/status_bar.rs`
  - Implement StatusBar view showing:
    - Real-time prices for subscribed symbols (from PriceService)
    - Task statistics (running/paused/stopped counts)
    - Connection status indicator (WebSocket health)
  - Subscribe to price updates via watch channel
  - Auto-refresh on price changes

  **Must NOT do**:
  - Add complex charts or graphs
  - Show more than 5 prices simultaneously

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: UI component with real-time data
  - **Skills**: [`frontend-ui-ux`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 11, 13)
  - **Blocks**: Task 14
  - **Blocked By**: Task 9

  **References**:
  - `crates/standx-point-gui/src/price/mod.rs` - PriceService (from Task 9)
  - `crates/standx-point-adapter/src/types/models.rs:SymbolPrice` - Price data structure

  **Acceptance Criteria**:

  ```
  Scenario: Status bar displays price data
    Tool: Bash
    Steps:
      1. cargo build -p standx-point-gui
      2. grep -r "impl.*Render.*for StatusBar" crates/standx-point-gui/src/
      3. Assert: StatusBar implements Render
      4. grep -r "PriceService" crates/standx-point-gui/src/ui/status_bar.rs
      5. Assert: PriceService integrated
    Expected Result: Status bar compiles with price integration
    Evidence: .sisyphus/evidence/task-12-statusbar.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): implement status bar with real-time prices and task stats`
  - Files: `crates/standx-point-gui/src/ui/status_bar.rs`

---

- [x] 13. Task Detail Panel

  **What to do**:
  - Create `crates/standx-point-gui/src/ui/task_detail.rs`
  - Implement TaskDetailPanel showing:
    - Task configuration summary
    - Current status with state machine visualization
    - Associated account info (from AccountPanel)
    - Order history table (from database)
    - Trade history table (from database)
    - Operation log (recent actions)
  - Update panel when different task is selected in sidebar

  **Must NOT do**:
  - Allow inline editing (use modal from Task 11)
  - Show unlimited history (cap at 100 records)

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
    - Reason: Complex UI panel with multiple data sources
  - **Skills**: [`frontend-ui-ux`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 11, 12)
  - **Blocks**: Task 14
  - **Blocked By**: Tasks 10, 11

  **References**:
  - `crates/standx-point-gui/src/ui/account_panel.rs` - Account info display (from Task 10)
  - `crates/standx-point-gui/src/db/mod.rs` - History query methods
  - gpui-component: Table, Tabs components

  **Acceptance Criteria**:

  ```
  Scenario: Task detail panel renders with history
    Tool: Bash
    Steps:
      1. cargo build -p standx-point-gui
      2. grep -r "impl.*Render.*for TaskDetailPanel" crates/standx-point-gui/src/
      3. Assert: TaskDetailPanel implements Render
      4. grep -r "order_history" crates/standx-point-gui/src/ui/task_detail.rs
      5. Assert: order history displayed
    Expected Result: Detail panel compiles
    Evidence: .sisyphus/evidence/task-13-detail.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): implement task detail panel with history tables`
  - Files: `crates/standx-point-gui/src/ui/task_detail.rs`

---

### Wave 5: Final Integration

- [x] 14. Integration + Polish

  **What to do**:
  - Wire up all components in RootView
  - Implement proper error handling with user-friendly messages
  - Add loading states for async operations
  - Implement crash recovery (mark interrupted tasks as Paused on startup)
  - Add logging with tracing crate
  - Test full workflow: create account → create task → start → stop → view history
  - Fix any integration issues

  **Must NOT do**:
  - Add new features
  - Implement system tray or notifications (Phase 2)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Integration requires understanding all components
  - **Skills**: [`clean-code-reviewer`]
    - Final code quality check

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential (final task)
  - **Blocks**: None (final)
  - **Blocked By**: Tasks 11, 12, 13

  **References**:
  - All previous task files
  - `crates/standx-point-gui/src/ui/root.rs` - Main view to integrate

  **Acceptance Criteria**:

  ```
  Scenario: Full application workflow
    Tool: interactive_bash (tmux)
    Steps:
      1. cargo build --release -p standx-point-gui
      2. Assert: build succeeds
      3. tmux new-session -d -s gui-final "./target/release/standx-point-gui"
      4. sleep 5
      5. pgrep -f standx-point-gui
      6. Assert: process running
      7. ls ~/.standx-point-gui/data.db 2>/dev/null || ls ./data.db
      8. Assert: database file exists
      9. tmux kill-session -t gui-final
    Expected Result: App runs and creates database
    Evidence: .sisyphus/evidence/task-14-integration.txt
  ```

  **Commit**: YES
  - Message: `feat(gui): integrate all components and add error handling`
  - Files: `crates/standx-point-gui/src/`

---

## Commit Strategy

| After Task | Message | Files | Verification |
|------------|---------|-------|--------------|
| 1 | `feat(gui): scaffold standx-point-gui crate with GPUI dependencies` | `crates/standx-point-gui/`, `Cargo.toml` | `cargo build -p standx-point-gui` |
| 2 | `feat(gui): add SQLite schema for accounts, tasks, and history` | `src/db/schema.sql` | File exists |
| 3 | `feat(gui): define application state types and TaskStatus enum` | `src/state/` | `cargo check` |
| 4 | `feat(gui): implement SQLite database layer with encrypted credentials` | `src/db/` | `cargo build` |
| 5 | `feat(gui): implement GPUI app skeleton with resizable layout` | `src/main.rs`, `src/ui/` | App launches |
| 6 | `feat(gui): implement task state machine with transition validation` | `src/task/` | `cargo check` |
| 7 | `feat(gui): implement sidebar with task card list UI` | `src/ui/sidebar.rs`, `src/ui/task_card.rs` | `cargo build` |
| 8 | `feat(gui): implement async task execution engine with adapter integration` | `src/task/executor.rs` | `cargo build` |
| 9 | `feat(gui): implement WebSocket price stream service` | `src/price/` | `cargo build` |
| 10 | `feat(gui): implement account info panel with positions and orders` | `src/ui/account_panel.rs` | `cargo build` |
| 11 | `feat(gui): implement task CRUD operations with modal form` | `src/ui/task_form.rs` | `cargo build` |
| 12 | `feat(gui): implement status bar with real-time prices and task stats` | `src/ui/status_bar.rs` | `cargo build` |
| 13 | `feat(gui): implement task detail panel with history tables` | `src/ui/task_detail.rs` | `cargo build` |
| 14 | `feat(gui): integrate all components and add error handling` | `src/` | Full workflow test |

---

## Success Criteria

### Verification Commands
```bash
# Build verification
cargo build --release -p standx-point-gui  # Expected: success

# Binary exists
ls target/release/standx-point-gui  # Expected: file exists

# App launches (manual verification)
./target/release/standx-point-gui &
sleep 3
pgrep standx-point-gui  # Expected: process exists
```

### Final Checklist
- [x] All "Must Have" present:
  - [ ] GPUI 0.2.2 + gpui-component 0.5.0 framework
  - [ ] Sidebar + main content layout with resizable panels
  - [ ] Task card list with CRUD operations
  - [ ] Task state machine (Draft → Pending → Running → Paused → Stopped → Failed)
  - [ ] Real-time price display via WebSocket
  - [ ] Account info display (address, positions, orders, trades)
  - [ ] SQLite persistence with encrypted credentials
  - [ ] Async task execution (non-blocking UI)
- [x] All "Must NOT Have" absent:
  - [ ] No multi-exchange support
  - [ ] No strategy editing UI
  - [ ] No system tray
  - [ ] No desktop notifications
  - [ ] No plaintext credentials
- [x] All tests pass: N/A (manual QA only)
- [x] App launches without crash
- [x] Database created on first run
- [x] Can complete full workflow: create account → create task → start → stop → view history
