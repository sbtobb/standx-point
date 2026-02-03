# StandX Market Making Strategy Bot - Work Plan

> **Plan ID**: mm-strategy  
> **Created**: 2026-02-03  
> **Status**: Ready for Execution  
> **Estimated Effort**: Large (~10-14 hours)  
> **Parallel Execution**: YES - 4 waves

---

## TL;DR

Build a **multi-account market making bot** for StandX with task-driven architecture. Each task runs independent strategy logic while sharing a single market data stream via `tokio::sync::watch` channels.

**Key Decisions**:
- Complete WebSocket adapter first (clean architecture)
- Use `watch` channel for price distribution (latest-value semantics)
- Pause trading during WebSocket reconnection
- Cancel all orders on startup (clean slate)
- Full exit on shutdown (cancel orders + close positions)
- Task isolation: one task failure doesn't affect others
- Scale: <5 concurrent tasks (simple architecture)

**Deliverables**:
1. Completed `standx-point-adapter` WebSocket module
2. New `standx-mm-strategy` crate with library + binary
3. YAML-based task configuration
4. Production-ready market making bot

---

## Context

### Original Request
Build an automated market making bot for StandX with:
- Task-driven architecture (multiple accounts, shared market data)
- Per-task configuration (symbol, risk level, capital)
- Conservative risk profile by default
- Single machine deployment
- Runnable binary included

### User Decisions (Confirmed)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| WebSocket implementation | Complete adapter first | Cleaner architecture, reusable |
| Price broadcast | `watch` channel | Latest price semantics, no lock contention |
| Reconnection behavior | Pause trading | Safest option |
| Startup handling | Cancel all existing orders | Clean slate approach |
| Shutdown behavior | Cancel orders + close positions | Full exit, no leftovers |
| Task failure isolation | Others continue | High availability |
| Expected scale | <5 tasks | Simple architecture sufficient |

### Metis Review Findings (Addressed)
- ✅ WebSocket adapter completion: Explicit task added
- ✅ Arc<Mutex> replacement: Using `watch` channel
- ✅ Reconnection state machine: Designed in Task 3
- ✅ Order idempotency: Using `cl_ord_id` UUID
- ✅ Graceful shutdown: Coordinator with timeout
- ✅ Task failure isolation: Panic handling per task

---

## Work Objectives

### Core Objective
Implement a production-ready market making bot that maximizes StandX Maker Points while controlling risk through conservative positioning (5-10 bps from mark price).

### Concrete Deliverables
1. **standx-point-adapter**: 
   - HTTP trading endpoints implementation (`http/user.rs`, `http/trade.rs`, `http/signature.rs`)
   - WebSocket implementation (`ws/client.rs`)
2. **standx-mm-strategy**: New crate with:
   - `lib.rs`: Public API (TaskManager, StrategyConfig)
   - `market_data.rs`: Shared market data hub with watch channels
   - `task.rs`: Per-task execution logic
   - `order_state.rs`: Order lifecycle state machine
   - `strategy.rs`: Conservative market making logic
   - `risk.rs`: Risk management guards
   - `config.rs`: YAML configuration parsing
   - `main.rs`: Runnable binary
3. **Configuration schema**: Example `config.yaml` with multi-task setup
4. **Integration tests**: WebSocket reconnection, order state transitions

### Definition of Done
- [x] `cargo test --package standx-point-adapter` passes (WebSocket tests)
- [x] `cargo test --package standx-mm-strategy` passes (all tests)
- [x] `cargo run --example basic_mm` executes without errors
- [x] Example config can define 2+ tasks with different accounts
- [x] Bot gracefully handles WebSocket reconnection
- [x] Bot cancels all orders and closes positions on SIGTERM

### Must Have
- WebSocket market data streaming with automatic reconnection
- Shared price feed via `tokio::sync::watch` channels
- Per-task isolated execution with independent order books
- Conservative market making (5-10 bps from mark price)
- Order state tracking with `cl_ord_id` correlation
- Startup reconciliation (cancel existing orders)
- Graceful shutdown with position closure
- Task failure isolation (panic doesn't crash bot)

### Must NOT Have (Guardrails)
- NO aggressive high-frequency trading (out of scope)
- NO database persistence (in-memory only)
- NO complex strategy plugins (single strategy type)
- NO multiple risk profiles beyond conservative (YAGNI)
- NO position holding across restarts (clean slate)

---

## Verification Strategy

### Test Infrastructure Assessment
- **Infrastructure exists**: YES - `standx-point-adapter` has `#[cfg(test)]` modules
- **Framework**: Built-in `cargo test` with `tokio-test` and `wiremock`
- **User wants tests**: YES (TDD for critical paths)

### Test-Driven Development (TDD) Workflow

Each task follows RED-GREEN-REFACTOR:
1. **RED**: Write failing test first
2. **GREEN**: Implement minimum code to pass
3. **REFACTOR**: Clean up while keeping tests green

### Test Coverage Requirements
- **WebSocket reconnection**: Test recovery after connection drop
- **Order state machine**: Test all transitions (Pending → Sent → Acked → Filled/Cancelled)
- **Task isolation**: Test that one task panic doesn't affect others
- **Graceful shutdown**: Test SIGTERM handling with timeout
- **Price distribution**: Test `watch` channel broadcasts to all tasks

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 0 (Prerequisites):
├── Task 0: Implement HTTP trading endpoints
└── Task 1: Complete WebSocket adapter

Wave 1 (Foundation):
└── Task 2: Create strategy crate skeleton

Wave 2 (Core Infrastructure):
├── Task 3: Implement market data hub with watch channels
├── Task 4: Implement task manager and lifecycle
└── Task 5: Implement order state machine

Wave 3 (Business Logic):
├── Task 6: Implement conservative market making strategy
├── Task 7: Implement risk management guards
└── Task 8: Implement runnable binary

Wave 4 (Integration):
└── Task 9: Integration tests and examples
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 0 | None | 4, 6, 8 | 1 |
| 1 | None | 3 | 0 |
| 2 | None | 3, 4, 5 | 0, 1 |
| 3 | 1, 2 | 6 | 4, 5 |
| 4 | 0, 2 | 6 | 3, 5 |
| 5 | 2 | 6 | 3, 4 |
| 6 | 0, 3, 4, 5 | 7, 8 | None |
| 7 | 6 | 8 | None |
| 8 | 0, 6, 7 | 9 | None |
| 9 | 8 | None | None |

### Critical Path
Task 0 → Task 4 → Task 6 → Task 8 → Task 9

---

## TODOs

### Task 0: Implement HTTP Trading Endpoints (Prerequisite)

**What to do**:
Implement the `todo!()` stubs in HTTP modules that are required for the strategy to function:
1. Implement `user.rs` methods:
   - `query_orders()` - Query orders with filters
   - `query_open_orders()` - Query open orders for symbol
   - `query_positions()` - Query positions
   - `query_balance()` - Query account balance
2. Implement `trade.rs` methods:
   - `new_order()` - Place new order with body signature
   - `cancel_order()` - Cancel existing order with body signature
   - `change_leverage()` - Change leverage for symbol
3. Implement `RequestSigner` for body signature generation
4. Implement `StandxClient` method to set credentials and request signer

**Key Implementation Notes**:
- Body signature format: "{version},{request_id},{timestamp},{payload}" signed with Ed25519
- Use `Ed25519Signer` from `auth` module for signing
- Add `Authorization: Bearer {jwt}` header for JWT endpoints
- Add signature headers for body-signature endpoints

**Must NOT do**:
- Don't implement complex retry logic (use simple exponential backoff)
- Don't add rate limiting (defer to future)

**Recommended Agent Profile**:
- **Category**: `ultrabrain` (HTTP + crypto signing is complex)
- **Skills**: 
  - `documenting-rust-code`: For comprehensive doc comments
  - `fractal-context`: For holographic headers

**Parallelization**:
- **Can Run In Parallel**: YES
- **Parallel Group**: Wave 0 (Preparatory, with Task 1)
- **Blocks**: Task 4, Task 6, Task 8
- **Blocked By**: None

**References**:
- `crates/standx-point-adapter/src/http/user.rs:17-79` - User query endpoints (todo stubs)
- `crates/standx-point-adapter/src/http/trade.rs:19-48` - Trading endpoints (todo stubs)
- `crates/standx-point-adapter/src/http/signature.rs` - Body signature generator
- `crates/standx-point-adapter/src/http/client.rs` - StandxClient structure
- `crates/standx-point-adapter/src/auth/signer.rs` - Ed25519Signer
- `crates/standx-point-adapter/AGENTS.md` - Body signature format documentation

**Acceptance Criteria**:

**RED (Test First)**:
```rust
#[tokio::test]
async fn test_new_order_sends_correct_request() {
    let mock_server = MockServer::start().await;
    let client = create_client_with_mock_base_url(&mock_server.uri());
    
    Mock::given(method("POST"))
        .and(path("/api/new_order"))
        .and(header("Authorization", "Bearer test-jwt"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "message": "success",
            "request_id": "req-123"
        })))
        .mount(&mock_server)
        .await;
    
    let response = client.new_order(NewOrderRequest {
        symbol: "BTC-USD".to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        qty: Decimal::from_str("0.1").unwrap(),
        price: Some(Decimal::from_str("50000").unwrap()),
        time_in_force: TimeInForce::Gtc,
        reduce_only: false,
        cl_ord_id: Some("test-123".to_string()),
        margin_mode: None,
        leverage: None,
        tp_price: None,
        sl_price: None,
    }).await;
    
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_cancel_order_queries_and_cancels() {
    // Test that cancel order works with either order_id or cl_ord_id
}

#[tokio::test]
async fn test_query_open_orders_returns_orders() {
    // Test query endpoints return proper data structures
}
```

**GREEN (Implementation)**:
- [x] `query_orders()` implemented with JWT auth
- [x] `query_open_orders()` implemented with JWT auth
- [x] `query_positions()` implemented with JWT auth
- [x] `query_balance()` implemented with JWT auth
- [x] `new_order()` implemented with body signature
- [x] `cancel_order()` implemented with body signature
- [x] `change_leverage()` implemented with body signature
- [x] `RequestSigner` generates correct signature headers

**REFACTOR**:
- [x] Extract common HTTP request building logic
- [x] Add consistent error handling across all endpoints

**Commit**: YES  
- Message: `feat(adapter): implement HTTP trading endpoints with body signature`
- Files: `crates/standx-point-adapter/src/http/user.rs`, `crates/standx-point-adapter/src/http/trade.rs`, `crates/standx-point-adapter/src/http/signature.rs`
- Pre-commit: `cargo test --package standx-point-adapter http -- --nocapture`

---

### Task 1: Complete WebSocket Adapter Implementation

**What to do**:
Complete the `todo!()` stubs in `standx-point-adapter/src/ws/`:
1. Implement `connect_stream()` - Establish WebSocket connection with proper URL
2. Implement `send_subscription()` - Send subscribe/unsubscribe messages over active connection
3. Implement `parse_message()` - Parse incoming text/binary messages into `WebSocketMessage` enum variants
4. Implement message loop that:
   - Reads messages from WebSocket
   - Parses them via `parse_message()`
   - Sends parsed messages through `message_tx` channel
5. Implement automatic ping/pong handling (tokio-tungstenite handles this, but verify)
6. Implement graceful close handling (drop connection cleanly)

**Important**: Current `WebSocketMessage` enum uses `serde_json::Value` for data fields. Task 1 should work with this design (keep generic JSON), and let the consumer (strategy crate) deserialize to specific types as needed.

**Must NOT do**:
- Don't implement reconnection logic here (handled in market data hub)
- Don't change the WebSocketMessage design to strong types (defer if desired)
- Don't add business logic (keep it data-only per adapter conventions)

**Recommended Agent Profile**:
- **Category**: `unspecified-high` (async networking is complex)
- **Skills**: 
  - `documenting-rust-code`: For comprehensive doc comments
  - `fractal-context`: For holographic headers in new files
- **Skills Evaluated but Omitted**:
  - `react-best-practices`: Not applicable (Rust project)

**Parallelization**:
- **Can Run In Parallel**: YES
- **Parallel Group**: Wave 0-1 (with Task 0, Task 2)
- **Blocks**: Task 3
- **Blocked By**: None

**References**:
- `crates/standx-point-adapter/src/ws/client.rs:45` - `StandxWebSocket::new()` constructor (no URL param)
- `crates/standx-point-adapter/src/ws/client.rs:103-120` - Methods with `todo!()` stubs
- `crates/standx-point-adapter/src/ws/client.rs:17-32` - `WebSocketMessage` enum with `serde_json::Value` payload
- `crates/standx-point-adapter/src/ws/AGENTS.md` - WebSocket constraints (24h max, ping/pong)
- `crates/standx-point-adapter/src/types/models.rs:SymbolPrice` - Type to deserialize from JSON
- `tokio-tungstenite` docs: https://docs.rs/tokio-tungstenite/latest/tokio_tungstenite/

**Acceptance Criteria**:

**RED (Test First)**:
```rust
#[tokio::test]
async fn test_websocket_connects_and_receives_price() {
    // Note: StandxWebSocket::new() takes no URL - URLs are constants in the module
    let mut ws = StandxWebSocket::new();
    ws.connect_market_stream().await.expect("connect should succeed");
    ws.subscribe_price("BTC-USD").await.expect("subscribe should succeed");
    
    // Take receiver before connection to get message stream
    let mut rx = ws.take_receiver().expect("receiver should exist");
    
    // Wait for message (in real test, would need mock server)
    let msg = rx.recv().await.expect("should receive message");
    assert!(matches!(msg, WebSocketMessage::Price { .. }));
}
```

**GREEN (Implementation)**:
- [x] `connect_stream()` establishes WebSocket connection to given URL
- [x] `connect_market_stream()` connects to MARKET_STREAM_URL
- [x] `connect_order_stream()` connects to ORDER_STREAM_URL with JWT token
- [x] `send_subscription()` sends JSON subscribe message over active WebSocket
- [x] `parse_message()` correctly parses incoming WsMessage into WebSocketMessage
- [x] Message loop runs continuously, forwarding parsed messages to channel
- [x] Connection errors properly propagated
- [x] Graceful shutdown closes WebSocket cleanly

**REFACTOR**:
- [x] Extract message parsing into pure functions (testable)
- [x] Add comprehensive error variants for WebSocket errors
- [x] Consider splitting read/write halves for better concurrency

**Commit**: YES  
- Message: `feat(adapter): complete WebSocket client implementation`
- Files: `crates/standx-point-adapter/src/ws/client.rs`
- Pre-commit: `cargo test --package standx-point-adapter ws -- --nocapture`

---

### Task 2: Create Strategy Crate Skeleton

**What to do**:
Create the `standx-mm-strategy` crate structure:
1. Create `crates/standx-mm-strategy/Cargo.toml` with dependencies
2. Create `crates/standx-mm-strategy/src/lib.rs` with module declarations
3. Create placeholder files for all modules
4. Add workspace member reference in root `Cargo.toml`
5. Create example configuration file `examples/config.yaml`

**Dependencies to add**:
- `standx-point-adapter` (workspace local)
- `tokio` (async runtime)
- `rust_decimal` (price calculations)
- `serde` + `serde_yaml` (configuration)
- `tracing` + `tracing-subscriber` (logging)
- `anyhow` (error handling)
- `tokio-util` (async utilities)
- `uuid` (cl_ord_id generation)

**Must NOT do**:
- Don't implement actual logic yet (placeholders only)
- Don't add unnecessary dependencies

**Recommended Agent Profile**:
- **Category**: `quick` (scaffolding task)
- **Skills**:
  - `fractal-context`: For holographic headers
  - `uv-package-manager`: For dependency management (though it's Rust, pattern applies)

**Parallelization**:
- **Can Run In Parallel**: YES
- **Parallel Group**: Wave 1 (with Task 1)
- **Blocks**: Tasks 3, 4, 5
- **Blocked By**: None

**References**:
- `Cargo.toml` (root) - Workspace member syntax
- `crates/standx-point-adapter/Cargo.toml` - Example crate manifest

**Acceptance Criteria**:

**Automated Verification**:
```bash
# Verify crate structure
cargo check --package standx-mm-strategy
# Expected: Compiles successfully (no logic yet)

# Verify workspace integration
cargo check --workspace
# Expected: All crates compile
```

**Manual Verification**:
- [x] `crates/standx-mm-strategy/` directory exists with all subdirs
- [x] `Cargo.toml` has correct dependencies
- [x] Root `Cargo.toml` includes new member
- [x] `cargo build` succeeds at workspace root

**Commit**: YES  
- Message: `chore(strategy): create crate skeleton and configuration`
- Files: All new crate files
- Pre-commit: `cargo check --package standx-mm-strategy`

---

### Task 3: Implement Market Data Hub

**What to do**:
Implement shared market data distribution using `tokio::sync::watch`:
1. Create `market_data.rs` module
2. Implement `MarketDataHub` struct that:
   - Manages single WebSocket connection to StandX
   - Subscribes to price and depth channels
   - Maintains `watch::Sender<SymbolPrice>` for each tracked symbol
   - Handles reconnection with exponential backoff
   - Broadcasts connection state changes (Connected/Disconnected)
3. Implement reconnection state machine:
   - On disconnect: enter Paused state, notify all tasks
   - On reconnect: resubscribe channels, resume trading
   - Max retry attempts before giving up
4. Implement graceful shutdown (close WebSocket cleanly)

**Key Design**:
```rust
pub struct MarketDataHub {
    ws_url: String,
    symbols: Vec<String>,
    price_txs: HashMap<String, watch::Sender<SymbolPrice>>,
    connection_state: watch::Sender<ConnectionState>,
    shutdown: CancellationToken,
}

pub enum ConnectionState {
    Connected,
    Disconnected { retry_count: u32 },
    Paused, // During reconnection
}
```

**Must NOT do**:
- Don't use `Arc<Mutex<T>>` for price data (use `watch`)
- Don't implement trading logic here (pure data distribution)

**Recommended Agent Profile**:
- **Category**: `ultrabrain` (complex async state machine)
- **Skills**:
  - `rust-best-practices` (if available): Async patterns
  - `fractal-context`: For holographic headers

**Parallelization**:
- **Can Run In Parallel**: YES
- **Parallel Group**: Wave 2 (with Tasks 4, 5)
- **Blocks**: Task 6
- **Blocked By**: Tasks 1, 2

**References**:
- `tokio::sync::watch` docs: https://docs.rs/tokio/latest/tokio/sync/watch/index.html
- `crates/standx-point-adapter/src/ws/client.rs` - WebSocket client (completed in Task 1)
- `crates/standx-point-adapter/src/types/models.rs:SymbolPrice` - Price data type
- `tokio-util` `CancellationToken` for graceful shutdown

**Acceptance Criteria**:

**RED (Tests)**:
```rust
#[tokio::test]
async fn test_price_broadcasts_to_multiple_receivers() {
    let hub = MarketDataHub::new("wss://test.standx.io/ws", vec!["BTC-USD"]);
    let rx1 = hub.subscribe_price("BTC-USD");
    let rx2 = hub.subscribe_price("BTC-USD");
    
    // Simulate price update
    hub.update_price("BTC-USD", test_price()).await;
    
    assert_eq!(*rx1.borrow(), test_price());
    assert_eq!(*rx2.borrow(), test_price());
}

#[tokio::test]
async fn test_reconnection_notifies_tasks() {
    let hub = MarketDataHub::new_with_mock(...);
    let mut state_rx = hub.subscribe_connection_state();
    
    // Simulate disconnect
    hub.force_disconnect().await;
    assert!(matches!(*state_rx.borrow(), ConnectionState::Disconnected { .. }));
    
    // Simulate reconnect
    hub.simulate_reconnect().await;
    assert!(matches!(*state_rx.borrow(), ConnectionState::Connected));
}
```

**GREEN (Implementation)**:
- [x] WebSocket connection established
- [x] Price data parsed and broadcast via `watch` channels
- [x] Reconnection with exponential backoff (1s, 2s, 4s, max 30s)
- [x] Connection state changes broadcast to tasks
- [x] Graceful shutdown closes WebSocket

**REFACTOR**:
- [x] Extract reconnection logic into separate module
- [x] Add metrics for reconnection frequency

**Evidence to Capture**:
- [x] Test output showing price broadcast to multiple receivers
- [x] Test output showing reconnection state transitions

**Commit**: YES  
- Message: `feat(strategy): implement market data hub with watch channels`
- Files: `crates/standx-mm-strategy/src/market_data.rs`
- Pre-commit: `cargo test --package standx-mm-strategy market_data -- --nocapture`

---

### Task 4: Implement Task Manager and Lifecycle

**What to do**:
Implement task lifecycle management:
1. Create `task.rs` and `task_manager.rs` modules
2. Implement `Task` struct that:
   - Holds task configuration (symbol, account, risk params)
   - Manages its own `StandxClient` instance
   - Runs strategy loop as a tokio task
   - Handles panics gracefully (catch_unwind or panic hook)
3. Implement `TaskManager` that:
   - Spawns N tasks from configuration
   - Subscribes each task to market data hub
   - Monitors task health (restart failed tasks?)
   - Coordinates graceful shutdown
4. Implement startup sequence:
   - Query existing orders for account
   - Cancel all existing orders
   - Begin trading
5. Implement shutdown sequence:
   - Signal tasks to stop
   - Cancel all open orders
   - Close all positions
   - Exit within timeout (30s)

**Key Design**:
```rust
pub struct Task {
    id: Uuid,
    config: TaskConfig,
    client: StandxClient,
    price_rx: watch::Receiver<SymbolPrice>,
    state: TaskState,
}

pub struct TaskManager {
    tasks: Vec<JoinHandle<Result<()>>>,
    market_data_hub: Arc<MarketDataHub>,
    shutdown: CancellationToken,
}
```

**Must NOT do**:
- Don't implement order execution logic here (Task 5 handles that)
- Don't add complex supervision (restart policy out of scope for <5 tasks)

**Recommended Agent Profile**:
- **Category**: `ultrabrain` (lifecycle management is complex)
- **Skills**:
  - `fractal-context`: For holographic headers
  - `documenting-rust-code`: For lifecycle docs

**Parallelization**:
- **Can Run In Parallel**: YES
- **Parallel Group**: Wave 2 (with Tasks 3, 5)
- **Blocks**: Task 6
- **Blocked By**: Task 2

**References**:
- `crates/standx-point-adapter/src/http/client.rs` - StandxClient usage
- `crates/standx-point-adapter/src/types/requests.rs` - CancelOrderRequest
- `crates/standx-point-adapter/src/http/user.rs` - Query orders API
- Tokio graceful shutdown patterns: https://tokio.rs/tokio/topics/shutdown

**Acceptance Criteria**:

**RED (Tests)**:
```rust
#[tokio::test]
async fn test_task_startup_cancels_existing_orders() {
    let mock_server = setup_mock_server_with_orders(vec![order1, order2]);
    let task = Task::new(test_config(), market_data_hub);
    
    task.start().await.expect("startup should succeed");
    
    // Verify cancel requests were sent for existing orders
    mock_server.assert_cancel_requested(order1.id);
    mock_server.assert_cancel_requested(order2.id);
}

#[tokio::test]
async fn test_graceful_shutdown_cancels_orders_and_closes_positions() {
    let task = setup_running_task().await;
    
    let shutdown_future = task.shutdown(Duration::from_secs(5));
    shutdown_future.await.expect("shutdown should succeed");
    
    // Verify cancel and close position requests
    assert!(task.all_orders_cancelled());
    assert!(task.all_positions_closed());
}

#[tokio::test]
async fn test_one_task_panic_doesnt_affect_others() {
    let manager = TaskManager::new(vec![config1, config2, config3]);
    manager.start_all().await;
    
    // Force panic in task 2
    manager.force_panic(1).await;
    
    // Verify tasks 1 and 3 still running
    assert!(manager.is_running(0));
    assert!(manager.is_running(2));
}
```

**GREEN (Implementation)**:
- [x] Task struct with isolated state
- [x] Startup sequence: query → cancel → trade
- [x] Shutdown sequence: signal → cancel → close → exit
- [x] Panic isolation (catch_unwind in task spawn)
- [x] TaskManager spawns and monitors tasks

**REFACTOR**:
- [x] Extract shutdown coordinator into separate module
- [x] Add task health monitoring (simple heartbeat)

**Evidence to Capture**:
- [x] Test output showing startup cancellation
- [x] Test output showing graceful shutdown
- [x] Test output showing panic isolation

**Commit**: YES  
- Message: `feat(strategy): implement task manager with lifecycle control`
- Files: `crates/standx-mm-strategy/src/task.rs`, `crates/standx-mm-strategy/src/task_manager.rs`
- Pre-commit: `cargo test --package standx-mm-strategy task -- --nocapture`

---

### Task 5: Implement Order State Machine

**What to do**:
Implement order lifecycle tracking:
1. Create `order_state.rs` module
2. Define `OrderState` enum with states:
   - `Pending` (local decision, not yet sent)
   - `Sent` (HTTP request in flight)
   - `Acknowledged` (exchange confirmed, has order_id)
   - `PartiallyFilled` (some quantity filled)
   - `Filled` (complete fill)
   - `Cancelling` (cancel request sent)
   - `Cancelled` (exchange confirmed cancel)
   - `Failed` (error, terminal state)
3. Implement `OrderTracker` that:
   - Tracks all orders by `cl_ord_id`
   - Correlates WebSocket updates with local orders
   - Handles timeout detection (Sent → Failed)
   - Prevents duplicate orders (idempotency via `cl_ord_id`)
4. Implement reconciliation:
   - Query exchange state on startup
   - Sync local state with exchange reality

**Key Design**:
```rust
pub enum OrderState {
    Pending { created_at: Instant },
    Sent { sent_at: Instant, cl_ord_id: String },
    Acknowledged { order_id: i64, acked_at: Instant },
    PartiallyFilled { filled_qty: Decimal, remaining_qty: Decimal },
    Filled { filled_at: Instant },
    Cancelling { cancel_sent_at: Instant },
    Cancelled { cancelled_at: Instant },
    Failed { error: String },
}

pub struct OrderTracker {
    orders: HashMap<String, TrackedOrder>, // keyed by cl_ord_id
    timeout: Duration,
}
```

**Must NOT do**:
- Don't implement execution logic here (Task 6)
- Don't persist state to disk (in-memory only)

**Recommended Agent Profile**:
- **Category**: `unspecified-high` (state machines require precision)
- **Skills**:
  - `fractal-context`: For holographic headers

**Parallelization**:
- **Can Run In Parallel**: YES
- **Parallel Group**: Wave 2 (with Tasks 3, 4)
- **Blocks**: Task 6
- **Blocked By**: Task 2

**References**:
- `crates/standx-point-adapter/src/types/models.rs:Order` - Order type
- `crates/standx-point-adapter/src/ws/message.rs:OrderUpdateData` - WebSocket updates
- `uuid::Uuid::new_v4()` for `cl_ord_id` generation

**Acceptance Criteria**:

**RED (Tests)**:
```rust
#[tokio::test]
async fn test_order_lifecycle_pending_to_filled() {
    let mut tracker = OrderTracker::new(Duration::from_secs(30));
    let cl_ord_id = tracker.create_pending_order(buy_order()).await;
    
    assert!(matches!(tracker.get_state(&cl_ord_id), OrderState::Pending { .. }));
    
    tracker.mark_sent(&cl_ord_id).await;
    assert!(matches!(tracker.get_state(&cl_ord_id), OrderState::Sent { .. }));
    
    tracker.handle_ack(&cl_ord_id, 12345).await;
    assert!(matches!(tracker.get_state(&cl_ord_id), OrderState::Acknowledged { .. }));
    
    tracker.handle_fill_update(12345, filled_qty()).await;
    assert!(matches!(tracker.get_state(&cl_ord_id), OrderState::Filled));
}

#[tokio::test]
async fn test_duplicate_cl_ord_id_prevention() {
    let tracker = OrderTracker::new(Duration::from_secs(30));
    let cl_ord_id = "test-id-123".to_string();
    
    tracker.create_pending_order_with_id(cl_ord_id.clone()).await.expect("first should succeed");
    let result = tracker.create_pending_order_with_id(cl_ord_id).await;
    
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), OrderError::DuplicateId));
}

#[tokio::test]
async fn test_timeout_marks_order_failed() {
    let tracker = OrderTracker::new(Duration::from_millis(100));
    let cl_ord_id = tracker.create_pending_order(buy_order()).await;
    tracker.mark_sent(&cl_ord_id).await;
    
    tokio::time::sleep(Duration::from_millis(150)).await;
    tracker.check_timeouts().await;
    
    assert!(matches!(tracker.get_state(&cl_ord_id), OrderState::Failed { .. }));
}
```

**GREEN (Implementation)**:
- [x] All order states defined and transitions implemented
- [x] Timeout detection works
- [x] Duplicate `cl_ord_id` prevention works
- [x] WebSocket updates correctly correlate to local orders

**REFACTOR**:
- [x] Add state transition logging
- [x] Extract timeout checking to background task

**Evidence to Capture**:
- [x] Test output showing full lifecycle
- [x] Test output showing duplicate prevention
- [x] Test output showing timeout detection

**Commit**: YES  
- Message: `feat(strategy): implement order state machine with idempotency`
- Files: `crates/standx-mm-strategy/src/order_state.rs`
- Pre-commit: `cargo test --package standx-mm-strategy order_state -- --nocapture`

---

### Task 6: Implement Conservative Market Making Strategy

**What to do**:
Implement the core market making logic:
1. Create `strategy.rs` module
2. Implement `MarketMakingStrategy` that:
   - Monitors mark price via `watch::Receiver`
   - Calculates bid/ask prices (mark_price ± bps_offset)
   - Maintains three price tiers (L1: 0-5bps, L2: 5-10bps, L3: 10-20bps)
   - Places PostOnly limit orders (guarantees maker status)
   - Adjusts order prices when mark price moves
   - Handles partial fills (recalculate remaining quantity)
   - Detects and handles full fills (stop trading briefly)
3. Implement quote refresh logic:
   - Check every 3-5 seconds
   - If order >3 seconds old and price drifted >1bps, cancel and replace
4. Implement Uptime tracking (for monthly 5M token pool)
5. Implement AGGRESSIVE vs SURVIVAL modes (per your analysis)

**Key Design**:
```rust
pub struct MarketMakingStrategy {
    symbol: String,
    base_qty: Decimal,
    risk_level: RiskLevel, // Conservative = 5-10bps
    price_rx: watch::Receiver<SymbolPrice>,
    order_tracker: Arc<Mutex<OrderTracker>>,
    uptime_tracker: UptimeTracker,
    mode: StrategyMode,
}

pub enum StrategyMode {
    Aggressive { target_bps: (Decimal, Decimal) }, // (0, 8)
    Survival { target_bps: (Decimal, Decimal) },   // (2, 9)
}

impl MarketMakingStrategy {
    pub async fn run(&mut self, shutdown: CancellationToken) -> Result<()> {
        loop {
            tokio::select! {
                _ = self.price_rx.changed() => {
                    let price = *self.price_rx.borrow();
                    self.on_price_update(price).await?;
                }
                _ = shutdown.cancelled() => {
                    return self.graceful_exit().await;
                }
            }
        }
    }
}
```

**Must NOT do**:
- Don't implement complex multi-symbol logic (single symbol per task)
- Don't add aggressive high-frequency features (out of scope)

**Recommended Agent Profile**:
- **Category**: `ultrabrain` (strategy logic is core business value)
- **Skills**:
  - `fractal-context`: For holographic headers

**Parallelization**:
- **Can Run In Parallel**: NO
- **Parallel Group**: Wave 3 (sequential)
- **Blocks**: Tasks 7, 8
- **Blocked By**: Tasks 3, 4, 5

**References**:
- `crates/standx-point-adapter/src/types/requests.rs:NewOrderRequest` - Order request
- `crates/standx-point-adapter/src/types/enums.rs:TimeInForce::PostOnly` - Maker order type
- `rust_decimal::Decimal` for all price math (never use f64)
- StandX Maker Points rules (bps tiers: 0-10=100%, 10-30=50%, 30-100=10%)

**Acceptance Criteria**:

**RED (Tests)**:
```rust
#[tokio::test]
async fn test_strategy_places_bilateral_orders_on_price_update() {
    let (price_tx, price_rx) = watch::channel(test_price(50000));
    let mut strategy = MarketMakingStrategy::new(
        "BTC-USD",
        config(),
        price_rx,
    );
    
    // Simulate price update
    price_tx.send(test_price(50100)).expect("send should succeed");
    
    // Verify orders placed
    let orders = strategy.get_pending_orders();
    assert_eq!(orders.len(), 2); // bid + ask
    assert!(orders[0].price < Decimal::from(50100)); // bid below mark
    assert!(orders[1].price > Decimal::from(50100)); // ask above mark
}

#[tokio::test]
async fn test_strategy_cancels_and_replaces_on_price_drift() {
    let mut strategy = setup_strategy_with_order(price: 50000).await;
    
    // Price moves significantly
    strategy.update_price(Decimal::from(50200)).await;
    
    // Verify old order cancelled, new order placed
    assert!(strategy.was_cancel_requested(old_order_id));
    assert!(strategy.has_new_order_at_price(~50200 - bps_offset));
}

#[tokio::test]
async fn test_strategy_detects_partial_fill_and_adjusts() {
    let mut strategy = setup_strategy_with_order(qty: 1.0).await;
    
    // Simulate partial fill of 0.3
    strategy.handle_fill_update(partial_qty: 0.3).await;
    
    // Verify remaining 0.7 still in market
    let remaining = strategy.get_remaining_qty();
    assert_eq!(remaining, Decimal::from_str("0.7").unwrap());
}
```

**GREEN (Implementation)**:
- [x] Strategy places bilateral PostOnly orders
- [x] Orders priced at configured bps offset from mark
- [x] Price drift detection cancels and replaces
- [x] Partial fill handling
- [x] Uptime tracking for token rewards
- [x] AGGRESSIVE/SURVIVAL mode switching

**REFACTOR**:
- [x] Extract price calculation to pure functions
- [x] Add strategy metrics (orders placed/cancelled per minute)

**Evidence to Capture**:
- [x] Test output showing bilateral order placement
- [x] Test output showing price drift replacement
- [x] Test output showing partial fill handling

**Commit**: YES  
- Message: `feat(strategy): implement conservative market making with uptime tracking`
- Files: `crates/standx-mm-strategy/src/strategy.rs`
- Pre-commit: `cargo test --package standx-mm-strategy strategy -- --nocapture`

---

### Task 7: Implement Risk Management Guards

**What to do**:
Implement safety guards to prevent catastrophic losses:
1. Create `risk.rs` module
2. Implement `RiskManager` with guards:
   - **Price jump protection**: If price changes > X bps/second, pause trading
   - **Depth monitoring**: If order book depth drops below threshold, pause
   - **Maximum position limit**: If position exceeds configured size, stop new orders
   - **Fill rate monitoring**: If fills exceed threshold, pause (avoiding too much trading)
   - **Spread monitoring**: If spread > Y bps, don't quote (avoid adverse selection)
3. Implement risk state:
   - `Safe` - Normal operation
   - `Caution` - Some metrics elevated (log warnings)
   - `Halt` - Trading paused (notify tasks)
4. Integrate with strategy (check risk state before placing orders)

**Key Design**:
```rust
pub struct RiskManager {
    max_price_velocity_bps: Decimal,      // e.g., 5 bps/second
    min_depth_threshold: Decimal,         // e.g., 10000 USD
    max_position_size: Decimal,           // e.g., 50000 USD
    max_fill_rate_per_minute: u32,        // e.g., 5 fills/minute
    max_spread_bps: Decimal,              // e.g., 20 bps
    price_history: VecDeque<(Instant, Decimal)>,
    fills_history: VecDeque<Instant>,
}

pub enum RiskState {
    Safe,
    Caution { reasons: Vec<String> },
    Halt { reasons: Vec<String> },
}
```

**Must NOT do**:
- Don't implement complex risk models (VaR, etc.) - simple guards only
- Don't add automatic position closure (just halt new orders)

**Recommended Agent Profile**:
- **Category**: `unspecified-high` (risk code must be correct)
- **Skills**:
  - `fractal-context`: For holographic headers

**Parallelization**:
- **Can Run In Parallel**: NO
- **Parallel Group**: Wave 3 (sequential)
- **Blocks**: Task 8
- **Blocked By**: Task 6

**References**:
- `crates/standx-point-adapter/src/types/models.rs:DepthBook` - Depth data
- `crates/standx-point-adapter/src/types/models.rs:Position` - Position tracking

**Acceptance Criteria**:

**RED (Tests)**:
```rust
#[tokio::test]
async fn test_price_jump_triggers_halt() {
    let mut risk = RiskManager::new(config());
    
    // Normal price
    risk.update_price(Decimal::from(50000), Instant::now()).await;
    assert!(matches!(risk.check_state(), RiskState::Safe));
    
    // Sudden jump
    risk.update_price(Decimal::from(50500), Instant::now()).await; // 100 bps jump
    assert!(matches!(risk.check_state(), RiskState::Halt { .. }));
}

#[tokio::test]
async fn test_depth_drop_triggers_caution() {
    let mut risk = RiskManager::new(config());
    
    risk.update_depth(DepthBook { total_value: Decimal::from(5000) }).await;
    
    assert!(matches!(risk.check_state(), RiskState::Caution { .. }));
}

#[tokio::test]
async fn test_position_limit_blocks_new_orders() {
    let risk = RiskManager::new(RiskConfig { max_position: Decimal::from(1000) });
    
    let can_trade = risk.can_place_new_order(current_position: Decimal::from(1200));
    
    assert!(!can_trade);
}
```

**GREEN (Implementation)**:
- [x] Price jump detection works
- [x] Depth monitoring works
- [x] Position limit enforcement works
- [x] Fill rate tracking works
- [x] Risk state transitions work
- [x] Strategy respects risk halt

**REFACTOR**:
- [x] Add risk metrics logging
- [x] Extract thresholds to configuration

**Evidence to Capture**:
- [x] Test output showing price jump halt
- [x] Test output showing depth caution
- [x] Test output showing position limit

**Commit**: YES  
- Message: `feat(strategy): implement risk management guards`
- Files: `crates/standx-mm-strategy/src/risk.rs`
- Pre-commit: `cargo test --package standx-mm-strategy risk -- --nocapture`

---

### Task 8: Implement Runnable Binary

**What to do**:
Create the executable entry point:
1. Create `main.rs` with CLI parsing
2. Implement argument parsing:
   - `--config <path>`: Path to YAML config file
   - `--log-level <level>`: tracing log level
   - `--dry-run`: Validate config without trading
3. Implement main flow:
   - Parse config
   - Initialize tracing subscriber
   - Create MarketDataHub
   - Create TaskManager with tasks from config
   - Handle SIGTERM/SIGINT for graceful shutdown
   - Run until shutdown signal
4. Add logging/tracing:
   - Log all state transitions
   - Log order placements/cancellations
   - Log risk events
   - Log WebSocket reconnections
5. Create example configuration files:
   - `examples/single_task.yaml`: Single account example
   - `examples/multi_task.yaml`: Multiple accounts example

**Key Design**:
```rust
#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Load config
    let config = load_config(&args.config_path)?;
    
    // Create shared market data hub
    let market_data_hub = Arc::new(MarketDataHub::new(...));
    
    // Create task manager
    let task_manager = TaskManager::new(config.tasks, market_data_hub.clone());
    
    // Setup signal handlers
    let shutdown = CancellationToken::new();
    setup_signal_handlers(shutdown.clone());
    
    // Start tasks
    task_manager.start_all().await?;
    
    // Wait for shutdown signal
    shutdown.cancelled().await;
    
    // Graceful shutdown
    task_manager.shutdown(Duration::from_secs(30)).await?;
    
    Ok(())
}
```

**Must NOT do**:
- Don't add complex CLI features (config file is primary interface)
- Don't add interactive mode (headless only)

**Recommended Agent Profile**:
- **Category**: `unspecified-high` (main.rs needs to be robust)
- **Skills**:
  - `fractal-context`: For holographic headers
  - `documenting-rust-code`: For usage documentation

**Parallelization**:
- **Can Run In Parallel**: NO
- **Parallel Group**: Wave 3 (sequential)
- **Blocks**: Task 9
- **Blocked By**: Tasks 6, 7

**References**:
- `clap` crate for CLI parsing: https://docs.rs/clap/latest/clap/
- `tracing` docs: https://docs.rs/tracing/latest/tracing/
- `tokio::signal` for SIGTERM handling: https://docs.rs/tokio/latest/tokio/signal/index.html

**Acceptance Criteria**:

**Automated Verification**:
```bash
# Build binary
cargo build --release --package standx-mm-strategy
# Expected: Binary created at target/release/standx-mm-strategy

# Test CLI parsing
./target/release/standx-mm-strategy --help
# Expected: Shows usage information

# Test dry-run mode
./target/release/standx-mm-strategy --config examples/single_task.yaml --dry-run
# Expected: Parses config, validates structure, exits without trading
```

**GREEN (Implementation)**:
- [x] CLI argument parsing works
- [x] Config file loading works
- [x] Logging initialization works
- [x] SIGTERM handler triggers graceful shutdown
- [x] Main flow runs without errors
- [x] Example config files are valid

**REFACTOR**:
- [x] Add structured logging (JSON output option)
- [x] Add startup banner with version info

**Evidence to Capture**:
- [x] Binary build output
- [x] Help text output
- [x] Dry-run output

**Commit**: YES  
- Message: `feat(strategy): implement runnable binary with CLI and config`
- Files: `crates/standx-mm-strategy/src/main.rs`, `examples/*.yaml`
- Pre-commit: `cargo build --release --package standx-mm-strategy`

---

### Task 9: Integration Tests and Examples

**What to do**:
Add comprehensive integration tests and usage examples:
1. Create integration tests in `tests/`:
   - `integration_test.rs`: End-to-end test with mock server
   - `reconnection_test.rs`: WebSocket reconnection scenario
   - `shutdown_test.rs`: Graceful shutdown verification
2. Create example files:
   - `examples/basic_mm.rs`: Simple usage example
   - Document expected behavior in comments
3. Add documentation:
   - `README.md` with setup instructions
   - Configuration reference
   - Architecture overview

**Must NOT do**:
- Don't add too many examples (1-2 good ones is enough)
- Don't test against live exchange (use mocks)

**Recommended Agent Profile**:
- **Category**: `writing` (documentation focus)
- **Skills**:
  - `fractal-context`: For holographic headers

**Parallelization**:
- **Can Run In Parallel**: NO
- **Parallel Group**: Wave 4 (final)
- **Blocks**: None
- **Blocked By**: Task 8

**References**:
- `wiremock` for HTTP mocking: https://docs.rs/wiremock/latest/wiremock/
- `crates/standx-point-adapter/tests/` - Example test patterns

**Acceptance Criteria**:

**Automated Verification**:
```bash
# Run all tests
cargo test --package standx-mm-strategy
# Expected: All tests pass

# Run integration tests only
cargo test --package standx-mm-strategy --test integration_test
# Expected: Integration tests pass

# Run example (dry-run mode)
cargo run --example basic_mm -- --dry-run
# Expected: Example executes without errors
```

**GREEN (Implementation)**:
- [x] Integration test with mock server passes
- [x] Reconnection test passes
- [x] Shutdown test passes
- [x] Example compiles and runs
- [x] README.md is complete

**Commit**: YES  
- Message: `test(strategy): add integration tests and documentation`
- Files: `crates/standx-mm-strategy/tests/*.rs`, `README.md`
- Pre-commit: `cargo test --package standx-mm-strategy`

---

## Commit Strategy

| After Task | Message | Files | Verification |
|------------|---------|-------|--------------|
| 1 | `feat(adapter): complete WebSocket client implementation` | `ws/client.rs`, `ws/message.rs` | `cargo test --package standx-point-adapter ws` |
| 2 | `chore(strategy): create crate skeleton and configuration` | New crate files | `cargo check --package standx-mm-strategy` |
| 3 | `feat(strategy): implement market data hub with watch channels` | `market_data.rs` | `cargo test --package standx-mm-strategy market_data` |
| 4 | `feat(strategy): implement task manager with lifecycle control` | `task.rs`, `task_manager.rs` | `cargo test --package standx-mm-strategy task` |
| 5 | `feat(strategy): implement order state machine with idempotency` | `order_state.rs` | `cargo test --package standx-mm-strategy order_state` |
| 6 | `feat(strategy): implement conservative market making with uptime tracking` | `strategy.rs` | `cargo test --package standx-mm-strategy strategy` |
| 7 | `feat(strategy): implement risk management guards` | `risk.rs` | `cargo test --package standx-mm-strategy risk` |
| 8 | `feat(strategy): implement runnable binary with CLI and config` | `main.rs`, examples | `cargo build --release` |
| 9 | `test(strategy): add integration tests and documentation` | tests, README | `cargo test --package standx-mm-strategy` |

---

## Success Criteria

### Verification Commands
```bash
# All tests pass
cargo test --workspace
# Expected: test result: ok. XX passed, 0 failed

# Binary builds successfully
cargo build --release --package standx-mm-strategy
# Expected: Compiles without warnings

# Clippy clean
cargo clippy --package standx-mm-strategy -- -D warnings
# Expected: No warnings

# Example runs
cargo run --example basic_mm -- --dry-run
# Expected: Parses config, exits cleanly
```

### Final Checklist
- [x] HTTP endpoints implemented (Task 0)
- [x] WebSocket adapter completed (Task 1)
- [x] All 10 tasks implemented with passing tests
- [x] Binary can be built and run
- [x] Example configurations are valid
- [x] Documentation is complete
- [x] No compiler warnings
- [x] No TODO!() stubs remaining (except in tests)

---

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────┐
│                     standx-mm-strategy                      │
│                          (Binary)                            │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────────────────────┐  │
│  │   CLI / main    │  │         Task Manager             │  │
│  │   - Arg parse   │──│  - Spawns N tasks               │  │
│  │   - Signal      │  │  - Monitors health               │  │
│  │   - Shutdown    │  │  - Coordinates exit              │  │
│  └─────────────────┘  └──────────────┬────────────────────┘  │
│                                      │                       │
│                    ┌─────────────────┼─────────────────┐     │
│                    │                 │                 │     │
│  ┌─────────────────▼──┐  ┌───────────▼────────┐  ┌────▼────┐│
│  │      Task 1        │  │      Task 2        │  │ Task N  ││
│  │  ┌──────────────┐  │  │  ┌──────────────┐  │  ┌─────┐  ││
│  │  │   Strategy   │  │  │  │   Strategy   │  │  │ ... │  ││
│  │  │  (Conserv)   │  │  │  │  (Conserv)   │  │  │     │  ││
│  │  └──────┬───────┘  │  │  └──────┬───────┘  │  └──┬──┘  ││
│  │  ┌──────▼───────┐  │  │  ┌──────▼───────┐  │  ┌──▼──┐  ││
│  │  │ Risk Manager │  │  │  │ Risk Manager │  │  │ ... │  ││
│  │  └──────┬───────┘  │  │  └──────┬───────┘  │  └──┬──┘  ││
│  │  ┌──────▼───────┐  │  │  ┌──────▼───────┐  │  ┌──▼──┐  ││
│  │  │Order Tracker │  │  │  │Order Tracker │  │  │ ... │  ││
│  │  └──────┬───────┘  │  │  └──────┬───────┘  │  └──┬──┘  ││
│  │  ┌──────▼───────┐  │  │  ┌──────▼───────┐  │  ┌──▼──┐  ││
│  │  │ StandxClient │  │  │  │ StandxClient │  │  │ ... │  ││
│  │  └──────────────┘  │  │  └──────────────┘  │  └─────┘  ││
│  └────────────────────┘  └────────────────────┘  └────────┘│
│                         │                                   │
│  ┌──────────────────────┼──────────────────────┐            │
│  │     Market Data Hub  │                      │            │
│  │  ┌───────────────────▼───────────────────┐  │            │
│  │  │   Single WebSocket Connection          │  │            │
│  │  │   - Broadcasts via watch channels      │  │            │
│  │  │   - Handles reconnection               │  │            │
│  │  └─────────────────────────────────────────┘  │            │
│  └───────────────────────────────────────────────┘            │
└───────────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────────┐
│              standx-point-adapter (Dependency)                  │
│  ┌────────────────┐  ┌────────────────┐  ┌─────────────────┐ │
│  │  HTTP Client   │  │ WebSocket Client│  │     Types      │ │
│  │  - REST API    │  │ - Real-time     │  │ - Order        │ │
│  │  - Auth        │  │ - Reconnect     │  │ - SymbolPrice  │ │
│  └────────────────┘  └────────────────┘  └─────────────────┘ │
└───────────────────────────────────────────────────────────────┘
```

---

## Next Steps

1. **Review this plan** - Does it match your expectations?
2. **Choose accuracy mode**:
   - **Standard**: Run `/start-work` to begin execution
   - **High Accuracy**: I'll submit to Momus for rigorous review first
3. **Begin execution** - Sisyphus will execute tasks in order

Run `/start-work` when ready to begin.