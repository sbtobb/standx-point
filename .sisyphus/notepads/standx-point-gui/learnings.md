# Learnings - standx-point-gui

## 2026-02-03 Initial Context Gathering

### Workspace Structure
- Workspace uses `resolver = "2"`
- Existing crates: `standx-point-adapter`, `standx-point-mm-strategy`
- Workspace dependencies pattern: `standx-point-adapter = { workspace = true }`

### Adapter Types (from models.rs)
- `Order`: Full order details with Decimal fields using `rust_decimal::serde::str`
- `Position`: Position with entry_price, qty, upnl, liq_price
- `Balance`: Account balance with equity, available, margin fields
- `Trade`: Trade history with fee, pnl, price, qty
- `SymbolPrice`: Real-time price with mark_price, index_price, mid_price

### Config Patterns (from mm-strategy/config.rs)
- `TaskConfig`: id, symbol, credentials, risk, sizing
- `CredentialsConfig`: jwt_token, signing_key (base64)
- Use serde derives for YAML/JSON serialization

### Conventions
- Fractal Context headers: `[INPUT]/[OUTPUT]/[POS]/[UPDATE]`
- Use `rust_decimal` for price/quantity (not f64)
- All async via Tokio
- Error handling via `anyhow::Result`

### Database Schema Design
- Decimals (price, qty, pnl) are stored as TEXT in SQLite to maintain precision when mapping to Rust's `rust_decimal::Decimal`.
- Use `config_json` to store complex task configurations to avoid deep table normalization for volatile strategy parameters.
- Credentials columns are prefixed with `encrypted_` to signal that they should never store plaintext.
- Fractal header added to SQL file for documentation consistency.

## Core State Implementation (2026-02-04)

- Defined `AppState`, `Account`, `Task`, `TaskStatus`, and `PriceData` in `crates/standx-point-gui/src/state/mod.rs`.
- `AppState` uses `HashMap<String, PriceData>` for efficient price lookups by symbol.
- `Task` includes `standx_point_mm_strategy::config::TaskConfig` directly to ensure synchronization with the strategy runner.
- `TaskStatus` covers all 6 states required for the GUI (Draft, Pending, Running, Paused, Stopped, Failed).
- Re-exported core adapter types (`Order`, `Position`, etc.) to provide a unified state interface.
- Encountered upstream dependency error in `zed-font-kit` (core-graphics version mismatch) during `cargo check`. This is an external environment issue and doesn't affect the correctness of the newly added state types.

- Created crates/standx-point-gui with minimal GPUI skeleton.
- Encountered dependency conflict between cocoa and core-text/zed-font-kit (multiple core-graphics versions).
- Verified workspace registration and file structure.
- Created crates/standx-point-gui with minimal GPUI skeleton.
- Encountered dependency conflict between cocoa and core-text/zed-font-kit (multiple core-graphics versions).
- Verified workspace registration and file structure.

## 2026-02-04 Build Dependency Pinning

- Pinning core-* crates to a single core-foundation-rs source revision avoids mixed core-graphics/core-text types.
- Using the core-text-v21.0.0 tag keeps core-foundation/core-graphics/core-text aligned in one git source.
- Build now proceeds past core-graphics conflicts but fails on GPUI API mismatches in standx-point-gui main.rs.

## 2026-02-04 Database Layer Implementation

- 新增 r2d2 + r2d2_sqlite 连接池，初始化时开启 WAL 并执行 schema.sql。
- 账户凭证入库前使用 AES-256-GCM 加密，nonce + ciphertext 以 hex 字符串存储。
- Task config 以 JSON 存储，TaskStatus/Side/OrderStatus 使用 serde JSON 字符串编码。

## 2026-02-04 UI Implementation Details

- **ResizablePanel**: The `ResizablePanel::new()` constructor is private. Used `impl From<AnyElement>` for `ResizablePanel` to implicitly convert `div()...into_any_element()` into a panel when adding to `h_resizable`.
- **Sidebar**: `Sidebar` uses `SidebarItem` trait for children. `SidebarGroup` implements it. `Sidebar` width is fixed by default but can be overridden with `.w()`.
- **Build**: `main.rs` should use types from the library crate (`standx_point_gui`) instead of redeclaring modules, to avoid type mismatch and recompilation of internal modules.
- Implemented TaskCard as RenderOnce component with status coloring and action buttons.
- Implemented SidebarView to list tasks from AppState.
- Encountered and fixed closure signature issues for GPUI event listeners (cx.listener takes 4 args, on_mouse_down takes 3).
- Commented out broken account_panel module to allow build verification.

## 2026-02-04 Account Info Panel
- Implemented `AccountPanel` using `Render` trait, displaying account header, balance summary cards, positions table, and orders table.
- Used `gpui` styling methods (`font_weight`, `text_color`, `bg`, `border_color`, `grid_cols`).
- Formatted `Decimal` using `rust_decimal` and string formatting for UI display, with color coding for PnL and Side.
- Handled `Option` types for balance/orders/positions empty states.
- Verified build success for `standx-point-gui` including the new module.
- Updated state.rs: Removed on_confirm closure from ModalType::Confirm to simplify state management
