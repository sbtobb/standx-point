# TUI 模块重构计划

## TL;DR

> **目标**: 重构 standx-point-mm-strategy 的 TUI 模块，规范化文件架构，增强功能（独立日志窗口、扩展订单展示、价格详情、创建弹窗）
>
> **关键改进**:
> - 将单文件 mod.rs (831 行) 拆分为模块化架构
> - 添加 Tab 切换系统（Dashboard / Full Logs / Create）
> - 扩展 Open Orders 展示（TP/SL 价格、仅减仓、挂单时间）
> - 添加价格详情（mark/last/min）
> - Account/Task 创建弹窗，使用选择组件减少输入
>
> **估计工作量**: Medium (2-3 天)
> **并行执行**: YES - 4 个 Wave
> **关键路径**: Wave 1 (基础拆分) → Wave 2 (Tab 系统) → Wave 3 (Modal) → Wave 4 (数据扩展)

---

## 上下文

### 原始请求
用户要求重构 TUI 模块：
1. 规范化文件架构
2. Log 窗口独立化，快捷键切换 Tab 展示
3. Open Orders 添加止盈/止损价格、仅减仓、挂单时间
4. Positions 缩小（通常仅有一个仓位）
5. 添加 task symbol 价格详情（mark, last, min）
6. 添加 account/task 创建窗口，快捷键呼出
7. 选项项使用选择框减少用户输入

### 当前架构
- **文件**: `crates/standx-point-mm-strategy/src/tui/mod.rs` (831 行，单一文件)
- **依赖**: ratatui 0.24+，crossterm 0.27+
- **布局**: 垂直 4 区块 - Account Summary / Middle(Task List + Positions/Orders) / Logs / Footer
- **快捷键**: Up/Down(选择), s(启动), x(停止), r(刷新), q(退出)

### Metis 审查发现
**已整合到本计划中的改进**:
1. 明确 UI 状态机：Idle / LogsTab / Modal(Account/Task) / Popup
2. 热键冲突解决：Modal 开启时禁用其他热键，Esc 关闭 Modal
3. 数据字段 fallback：TP/SL 从 payload 解析失败时显示 "-"
4. 最小终端尺寸：80x24，低于此尺寸显示简化提示
5. 模块单向依赖：ui/ 组件不互相调用，通过 AppState 传递数据

---

## 工作目标

### 核心目标
重构 TUI 模块为可维护的模块化架构，同时保持现有功能并添加新特性。

### 具体交付物
- [x] 模块化文件结构（10+ 个文件）
- [x] Tab 切换系统（Dashboard / Full Logs / Create）
- [x] Open Orders 扩展字段展示
- [x] 价格详情展示（mark/last/min）
- [x] Account 创建弹窗
- [x] Task 创建弹窗
- [x] 选择组件（Chain, RiskLevel, Account, Symbol）

### 定义完成
- [ ] 所有现有功能保持可用（启动/停止任务、刷新、导航）
- [x] 新增快捷键工作正常（Tab 切换, a/t 打开弹窗, Esc 关闭）
- [ ] 布局在 80x24 及以上终端正常工作
- [ ] `cargo clippy --workspace` 无警告
- [x] `cargo build -p standx-point-mm-strategy` 成功

### 必须包含
- 模块化文件架构
- Tab 切换系统
- 扩展订单展示
- 创建弹窗

### 明确排除（Guardrails）
- 日志搜索/过滤功能（Phase 2）
- 可配置列/排序（Phase 2）
- 多语言支持
- 鼠标支持（保持纯键盘）

---

## 验证策略

### 测试决策
- **测试基础设施**: 存在（cargo test）
- **自动化测试**: NO（TUI 交互以人工验证为主）
- **Agent-Executed QA**: YES（启动 TUI 验证各界面）

### Agent-Executed QA Scenarios

#### Scenario 1: Dashboard Tab 正常显示
**Tool**: interactive_bash (tmux)
**Steps**:
1. `cargo run -p standx-point-mm-strategy -- --tui`
2. Wait for TUI to load (timeout: 10s)
3. Assert: Dashboard tab visible with "Tasks" list
4. Assert: Positions table shows headers (Symbol, Qty, Entry, Mark, uPnL)
5. Assert: Open Orders table shows headers (Symbol, Side, Type, Price, Qty, Status, TP, SL, Reduce, Time)
6. Screenshot: `.sisyphus/evidence/tui-dashboard-tab.png`

#### Scenario 2: Tab 切换到 Full Logs
**Tool**: interactive_bash (tmux)
**Steps**:
1. From Dashboard, press `l` key
2. Wait for render (timeout: 1s)
3. Assert: Full screen log view visible
4. Assert: Header shows "Logs [l]"
5. Press `Tab` key
6. Assert: Returns to Dashboard
7. Screenshot: `.sisyphus/evidence/tui-logs-tab.png`

#### Scenario 3: Account 创建弹窗
**Tool**: interactive_bash (tmux)
**Steps**:
1. From Dashboard, press `a` key
2. Wait for modal (timeout: 1s)
3. Assert: "Create Account" modal visible
4. Assert: Chain field shows selection list [BSC, Solana]
5. Press `Esc` key
6. Assert: Modal closes, back to Dashboard
7. Screenshot: `.sisyphus/evidence/tui-create-account.png`

#### Scenario 4: Task 创建弹窗
**Tool**: interactive_bash (tmux)
**Steps**:
1. From Dashboard, press `t` key
2. Wait for modal (timeout: 1s)
3. Assert: "Create Task" modal visible
4. Assert: Risk Level shows selection list [low, medium, high, xhigh]
5. Assert: Account dropdown shows existing accounts
6. Press `Esc` key
7. Assert: Modal closes
8. Screenshot: `.sisyphus/evidence/tui-create-task.png`

#### Scenario 5: 价格详情展示
**Tool**: interactive_bash (tmux)
**Steps**:
1. Start TUI with at least one running task
2. Wait for data refresh (timeout: 5s)
3. Assert: Account Summary shows "Mark: X.XX | Last: X.XX | Min: X.XX"
4. Screenshot: `.sisyphus/evidence/tui-price-details.png`

---

## 执行策略

### 并行执行 Waves

```
Wave 1: 基础架构拆分
├── 任务 1: 创建新文件结构 + 迁移 TerminalGuard
└── 任务 2: 迁移 AppState 和事件循环

Wave 2: UI 组件和 Tab 系统
├── 任务 3: 拆分 UI 渲染组件（account, task_list, positions, orders, logs）
├── 任务 4: 实现 Tab 切换系统（Dashboard/Logs/Create）
└── 任务 5: 更新布局和样式

Wave 3: Modal 弹窗
├── 任务 6: 实现通用 Modal 框架
├── 任务 7: 实现 CreateAccount 弹窗
└── 任务 8: 实现 CreateTask 弹窗

Wave 4: 数据扩展
├── 任务 9: 添加价格详情（mark/last/min）到 Account Summary
└── 任务 10: 扩展 Open Orders 字段（TP/SL, reduce_only, created_at）
```

### 依赖矩阵

| 任务 | 依赖 | 阻塞 | 可并行 |
|------|------|------|--------|
| 1 | None | 2 | None |
| 2 | 1 | 3,4 | None |
| 3 | 2 | 5 | 4 |
| 4 | 2 | 5 | 3 |
| 5 | 3,4 | 6 | None |
| 6 | 5 | 7,8 | None |
| 7 | 6 | 9 | 8 |
| 8 | 6 | 9 | 7 |
| 9 | 5 | 10 | None |
| 10 | 5 | None | 9 |

---

## TODOs

### Wave 1: 基础架构拆分

- [x] **1. 创建新文件结构 + 迁移 TerminalGuard**

  **做什么**:
  - 创建新的目录结构:
    ```
    src/tui/
    ├── mod.rs
    ├── terminal.rs      (新增)
    ├── app.rs           (新增)
    ├── events.rs        (新增)
    ├── state.rs         (新增)
    └── ui/
        ├── mod.rs       (新增)
        ├── layout.rs    (新增)
        ├── account.rs   (新增)
        ├── task_list.rs (新增)
        ├── positions.rs (新增)
        ├── orders.rs    (新增)
        ├── logs.rs      (新增)
        └── modal/
            ├── mod.rs   (新增)
            ├── create_account.rs (新增)
            └── create_task.rs    (新增)
    ```
  - 迁移 `TerminalGuard` 到 `terminal.rs`
  - 在新 `mod.rs` 中导出公共 API

  **必须不做**:
  - 不要删除原 mod.rs 内容（作为参考保留）
  - 不要修改 Cargo.toml 依赖

  **参考**:
  - 原 `mod.rs:800-831` TerminalGuard 实现

  **验收标准**:
  - [x] 所有新文件创建成功
  - [x] `cargo build -p standx-point-mm-strategy` 编译通过
  - [x] `mod.rs` 只包含模块导出

  **Agent-Executed QA**:
  ```
  Scenario: 文件结构创建完成
    Tool: Bash
    Steps:
      1. ls crates/standx-point-mm-strategy/src/tui/
      2. Assert: 包含 terminal.rs, app.rs, events.rs, state.rs, ui/
      3. cargo check -p standx-point-mm-strategy
      4. Assert: 编译成功，无错误
  ```

  **提交**: YES
  - Message: `refactor(tui): create modular file structure`
  - Files: `src/tui/*.rs`, `src/tui/ui/**/*.rs`

- [x] **2. 迁移 AppState 和事件循环**

  **做什么**:
  - 将 `AppState` (原 mod.rs:163-325) 迁移到 `app.rs`
  - 将事件处理逻辑迁移到 `events.rs`
  - 将数据刷新逻辑迁移到 `state.rs`
  - 添加 `AppMode` 枚举用于状态机:
    ```rust
    pub enum AppMode {
        Dashboard,      // 主界面
        LogsTab,        // 全屏日志
        CreateAccount,  // 创建账户弹窗
        CreateTask,     // 创建任务弹窗
    }
    ```

  **必须不做**:
  - 不要修改 AppState 的字段（保持兼容）
  - 不要删除原 mod.rs（留作备份）

  **参考**:
  - 原 `mod.rs:163-325` AppState 实现
  - 原 `mod.rs:618-694` 事件循环

  **验收标准**:
  - [x] AppState 完整迁移到 app.rs
  - [x] 事件处理迁移到 events.rs
  - [x] 状态刷新迁移到 state.rs
  - [x] `cargo build` 成功

  **Agent-Executed QA**:
  ```
  Scenario: AppState 和事件循环迁移完成
    Tool: Bash
    Steps:
      1. cargo check -p standx-point-mm-strategy
      2. Assert: 无编译错误
      3. rg "pub struct AppState" crates/standx-point-mm-strategy/src/tui/app.rs
      4. Assert: 找到定义
  ```

  **提交**: YES
  - Message: `refactor(tui): migrate AppState and event loop`
  - Files: `src/tui/app.rs`, `src/tui/events.rs`, `src/tui/state.rs`

### Wave 2: UI 组件和 Tab 系统

- [x] **3. 拆分 UI 渲染组件**

  **做什么**:
  - `ui/account.rs`: `draw_account_summary` (原 mod.rs:327-390)
  - `ui/task_list.rs`: `draw_task_list` (原 mod.rs:727-768)
  - `ui/positions.rs`: `draw_positions_table` (原 mod.rs:392-445)
  - `ui/orders.rs`: `draw_open_orders_table` (原 mod.rs:447-504)
  - `ui/logs.rs`: `draw_logs` (原 mod.rs:770-790)
  - `ui/mod.rs`: 导出所有渲染函数

  **必须不做**:
  - 不要修改渲染逻辑（保持行为一致）
  - 不要添加新功能（仅迁移）

  **参考**:
  - 原 `mod.rs` 对应函数实现

  **验收标准**:
  - [x] 所有渲染函数迁移到对应文件
  - [x] `ui/mod.rs` 正确导出
  - [x] `cargo build` 成功

  **Agent-Executed QA**:
  ```
  Scenario: UI 组件拆分完成
    Tool: Bash
    Steps:
      1. cargo build -p standx-point-mm-strategy
      2. Assert: 编译成功
      3. ls crates/standx-point-mm-strategy/src/tui/ui/
      4. Assert: 包含所有组件文件
  ```

  **提交**: YES
  - Message: `refactor(tui): split UI rendering components`
  - Files: `src/tui/ui/*.rs`

- [x] **4. 实现 Tab 切换系统**

  **做什么**:
  - 在 `app.rs` 添加 `current_tab: Tab` 字段:
    ```rust
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Tab {
        Dashboard,  // Tab 1
        Logs,       // Tab 2
        Create,     // Tab 3
    }
    ```
  - 添加 `Tab` 显示栏在 Footer 上方
  - 实现快捷键:
    - `Tab` / `l` - 循环切换 Tab
    - `1` - Dashboard
    - `2` - Logs
    - `3` - Create
  - 在 `ui/layout.rs` 实现 `Tab` 渲染:
    ```rust
    pub fn draw_tabs(frame: &mut Frame, area: Rect, current: Tab) {
        // 渲染 Tab 栏，高亮当前 Tab
    }
    ```

  **必须不做**:
  - Create Tab 暂时只显示占位符（后续任务实现）

  **参考**:
  - ratatui::widgets::Tabs 组件

  **验收标准**:
  - [x] Tab 栏显示在 Footer 上方
  - [x] `Tab` / `l` 键切换 Tab
  - [x] Dashboard Tab 保持原有布局
  - [x] Logs Tab 显示全屏日志
  - [x] Create Tab 显示占位信息

  **Agent-Executed QA**:
  ```
  Scenario: Tab 切换系统工作正常
    Tool: interactive_bash (tmux)
    Preconditions: TUI 可启动
    Steps:
      1. cargo run -p standx-point-mm-strategy -- --tui &
      2. Wait for TUI (timeout: 5s)
      3. Assert: Tab 栏可见 [Dashboard] [Logs] [Create]
      4. Send keys: "l"
      5. Wait: 500ms
      6. Assert: Logs tab active (full screen logs)
      7. Send keys: "l"
      8. Wait: 500ms
      9. Assert: Create tab active
      10. Send keys: "1"
      11. Wait: 500ms
      12. Assert: Dashboard tab active
      13. Screenshot: .sisyphus/evidence/tui-tabs.png
    Expected Result: Tab 切换流畅，高亮正确
    Evidence: .sisyphus/evidence/tui-tabs.png
  ```

  **提交**: YES
  - Message: `feat(tui): add tab switching system`
  - Files: `src/tui/app.rs`, `src/tui/ui/layout.rs`, `src/tui/ui/mod.rs`

- [x] **5. 更新布局和样式**

  **做什么**:
  - 在 `ui/layout.rs` 定义统一的布局约束:
    ```rust
    pub mod constraints {
        pub const TAB_HEIGHT: u16 = 3;
        pub const FOOTER_HEIGHT: u16 = 4;
        pub const MIN_MIDDLE_HEIGHT: u16 = 10;
    }
    ```
  - Dashboard 布局（4 区块）:
    ```
    ┌─────────────────────────────┐
    │ Account Summary             │ ← 4 lines
    ├─────────────────────────────┤
    │ Tasks │ Positions           │ ← 50%
    │       │ Orders              │ ← 50%
    ├─────────────────────────────┤
    │ Logs (summary)              │ ← 7 lines (compact)
    ├─────────────────────────────┤
    │ [Dashboard] [Logs] [Create] │ ← Tabs
    ├─────────────────────────────┤
    │ Hotkeys                     │ ← Footer
    └─────────────────────────────┘
    ```
  - 更新 Footer 显示新的快捷键:
    - `[Tab/l]` Switch Tab  `[1/2/3]` Go to Tab
    - `[a]` Create Account  `[t]` Create Task
    - `[s]` Start  `[x]` Stop  `[r]` Refresh  `[q]` Quit

  **必须不做**:
  - 不要改变颜色主题
  - 保持边框样式一致

  **验收标准**:
  - [x] Dashboard 布局符合设计
  - [x] Tab 栏显示正确
  - [x] Footer 显示所有快捷键
  - [ ] 最小 80x24 终端正常工作

  **Agent-Executed QA**:
  ```
  Scenario: 布局在最小尺寸下正常
    Tool: interactive_bash (tmux)
    Steps:
      1. tmux new-session -d -s tui-test
      2. tmux resize-window -t tui-test -x 80 -y 24
      3. tmux send-keys -t tui-test "cargo run -p standx-point-mm-strategy -- --tui" Enter
      4. Wait: 5s
      5. tmux capture-pane -t tui-test -p > /tmp/tui-output.txt
      6. Assert: /tmp/tui-output.txt contains "Account Summary"
      7. Assert: /tmp/tui-output.txt contains "[Tab/l]"
      8. tmux kill-session -t tui-test
    Expected Result: 最小尺寸下界面完整显示
  ```

  **提交**: YES
  - Message: `feat(tui): update layout and styling`
  - Files: `src/tui/ui/layout.rs`, `src/tui/ui/*.rs`

### Wave 3: Modal 弹窗

- [x] **6. 实现通用 Modal 框架**

  **做什么**:
  - 在 `ui/modal/mod.rs` 定义通用 Modal 框架:
    ```rust
    pub struct Modal {
        pub title: String,
        pub focus_index: usize,
        pub fields: Vec<Field>,
    }

    pub enum Field {
        TextInput { label: String, value: String },
        Select { label: String, options: Vec<String>, selected: usize },
        Button { label: String, action: Action },
    }
    ```
  - 实现 `draw_modal(frame, area, modal)` 渲染函数
  - 实现事件处理:
    - `Tab` - 切换字段焦点
    - `Up/Down` - 选择字段/选项
    - `Enter` - 确认
    - `Esc` - 关闭 Modal
  - Modal 显示时禁用其他热键（除 Esc）

  **必须不做**:
  - 不要实现具体表单逻辑（在子任务中）

  **参考**:
  - ratatui::widgets::Clear (清除背景)
  - ratatui::widgets::Block (边框和标题)

  **验收标准**:
  - [x] Modal 框架可渲染带边框的弹窗
  - [x] 支持 TextInput 和 Select 字段类型
  - [x] Esc 键关闭 Modal
  - [x] Modal 开启时主界面热键被禁用

  **Agent-Executed QA**:
  ```
  Scenario: Modal 框架工作正常
    Tool: interactive_bash (tmux)
    Steps:
      1. cargo run -p standx-point-mm-strategy -- --tui &
      2. Wait: 3s
      3. Send keys: "a" (假设已绑定到测试 Modal)
      4. Wait: 500ms
      5. Assert: Modal 边框可见
      6. Assert: Modal 标题显示
      7. Send keys: Esc
      8. Wait: 500ms
      9. Assert: Modal 关闭
      10. Screenshot: .sisyphus/evidence/tui-modal-framework.png
    Expected Result: Modal 正常显示和关闭
  ```

  **提交**: YES
  - Message: `feat(tui): add modal framework`
  - Files: `src/tui/ui/modal/mod.rs`

- [x] **7. 实现 CreateAccount 弹窗**

  **做什么**:
  - 在 `ui/modal/create_account.rs` 实现:
    ```rust
    pub struct CreateAccountModal {
        pub name: String,
        pub private_key: String,
        pub chain: Chain,  // Select: BSC / Solana
        pub focus: FieldFocus,
    }
    ```
  - 字段顺序:
    1. Name (TextInput)
    2. Private Key (TextInput，密码风格显示)
    3. Chain (Select: BSC, Solana)
    4. [Create] Button
    5. [Cancel] Button
  - 快捷键 `a` 从 Dashboard 打开此弹窗
  - 提交时:
    - 验证所有字段非空
    - 调用 `storage.create_account()` 保存
    - 显示状态消息
    - 关闭 Modal

  **必须不做**:
  - 不要实现 JWT/Signing Key 生成（后端处理）
  - 不要验证私钥格式（简化）

  **参考**:
  - `src/state/storage.rs` Account struct
  - Modal 框架（任务 6）

  **验收标准**:
  - [x] `a` 键打开 Create Account Modal
  - [x] 字段：Name, Private Key, Chain(选择框)
  - [x] Chain 使用选择框（BSC/Solana）
  - [x] 提交后 Account 出现在列表
  - [x] Esc 或 Cancel 关闭弹窗

  **Agent-Executed QA**:
  ```
  Scenario: Create Account 弹窗工作正常
    Tool: interactive_bash (tmux)
    Steps:
      1. cargo run -p standx-point-mm-strategy -- --tui &
      2. Wait: 3s
      3. Send keys: "a"
      4. Wait: 500ms
      5. Assert: "Create Account" modal visible
      6. Assert: Chain field shows "BSC" (default)
      7. Send keys: Down Down  (navigate to Cancel)
      8. Send keys: Enter
      9. Wait: 500ms
      10. Assert: Modal closed
      11. Screenshot: .sisyphus/evidence/tui-create-account.png
    Expected Result: Modal 导航和关闭工作正常
  ```

  **提交**: YES
  - Message: `feat(tui): add create account modal`
  - Files: `src/tui/ui/modal/create_account.rs`, `src/tui/events.rs`

- [x] **8. 实现 CreateTask 弹窗**

  **做什么**:
  - 在 `ui/modal/create_task.rs` 实现:
    ```rust
    pub struct CreateTaskModal {
        pub id: String,
        pub symbol: String,
        pub account_id: String,      // Select: from existing accounts
        pub risk_level: RiskLevel,   // Select: low/medium/high/xhigh
        pub budget_usd: String,
        pub focus: FieldFocus,
    }
    ```
  - 字段顺序:
    1. ID (TextInput，可选，默认生成)
    2. Symbol (Select: BTC-USD, ETH-USD, etc.)
    3. Account (Select: 从现有 accounts 加载)
    4. Risk Level (Select: low, medium, high, xhigh)
    5. Budget USD (TextInput)
    6. [Create] Button
    7. [Cancel] Button
  - 快捷键 `t` 从 Dashboard 打开此弹窗
  - 提交时:
    - 验证必填字段
    - 自动生成 ID（如果为空）
    - 调用 `storage.create_task()` 保存
    - 刷新任务列表

  **必须不做**:
  - 不要自动启动新任务（仅创建配置）

  **参考**:
  - `src/state/storage.rs` Task struct
  - `src/config.rs` RiskLevel enum

  **验收标准**:
  - [x] `t` 键打开 Create Task Modal
  - [x] Symbol 使用选择框（常用交易对）
  - [x] Account 从现有账户下拉选择
  - [x] Risk Level 使用选择框
  - [x] 提交后 Task 出现在列表

  **Agent-Executed QA**:
  ```
  Scenario: Create Task 弹窗工作正常
    Tool: interactive_bash (tmux)
    Steps:
      1. cargo run -p standx-point-mm-strategy -- --tui &
      2. Wait: 3s
      3. Send keys: "t"
      4. Wait: 500ms
      5. Assert: "Create Task" modal visible
      6. Assert: Risk Level shows "low" (default)
      7. Send keys: Down  (navigate fields)
      8. Send keys: Esc
      9. Wait: 500ms
      10. Assert: Modal closed
      11. Screenshot: .sisyphus/evidence/tui-create-task.png
    Expected Result: Modal 字段和选择框工作正常
  ```

  **提交**: YES
  - Message: `feat(tui): add create task modal`
  - Files: `src/tui/ui/modal/create_task.rs`, `src/tui/events.rs`

### Wave 4: 数据扩展

- [x] **9. 添加价格详情（mark/last/min）**

  **做什么**:
  - 扩展 `LiveTaskData` 结构体:
    ```rust
    pub struct LiveTaskData {
        pub balance: Option<Balance>,
        pub positions: Vec<Position>,
        pub open_orders: Vec<Order>,
        pub price_data: Option<PriceData>,  // 新增
        pub last_update: Option<Instant>,
        pub last_error: Option<String>,
    }

    pub struct PriceData {
        pub mark_price: Decimal,
        pub last_price: Decimal,
        pub min_price: Decimal,
    }
    ```
  - 在 `state.rs` 添加价格数据刷新逻辑:
    - 查询 symbol 最新价格（复用现有 API 或添加新端点）
    - 更新频率：3 秒（与现有 live_data 刷新同步）
  - 在 `ui/account.rs` 渲染价格详情:
    ```rust
    // 在 Account Summary 添加一行
    "Mark: {mark} | Last: {last} | Min: {min}"
    ```
  - 格式：`format_decimal(value, 4)`

  **必须不做**:
  - 不要添加 WebSocket 订阅（轮询即可）

  **参考**:
  - `src/types/models.rs` 中的价格相关类型
  - `StandxClient::query_symbol_info()` 或类似 API

  **验收标准**:
  - [x] LiveTaskData 包含 price_data
  - [x] Account Summary 显示 Mark/Last/Min
  - [x] 价格随 live_data 刷新更新（3s 间隔）
  - [x] 价格格式化为 4 位小数

  **Agent-Executed QA**:
  ```
  Scenario: 价格详情显示正常
    Tool: interactive_bash (tmux)
    Preconditions: 至少一个运行的任务
    Steps:
      1. cargo run -p standx-point-mm-strategy -- --tui &
      2. Wait: 3s
      3. Select a running task (Up/Down)
      4. Wait: 5s (for price refresh)
      5. Assert: Account Summary shows "Mark:" "Last:" "Min:"
      6. Wait: 3s
      7. Assert: Prices updated (values may change)
      8. Screenshot: .sisyphus/evidence/tui-price-details.png
    Expected Result: 三个价格字段可见且定期刷新
  ```

  **提交**: YES
  - Message: `feat(tui): display price details (mark/last/min)`
  - Files: `src/tui/state.rs`, `src/tui/app.rs`, `src/tui/ui/account.rs`

- [x] **10. 扩展 Open Orders 字段（TP/SL, reduce_only, created_at）**

  **做什么**:
  - 在 `ui/orders.rs` 扩展订单表格列:
    ```rust
    // 新表头
    let header = Row::new(vec![
        Cell::from("Symbol"),
        Cell::from("Side"),
        Cell::from("Type"),
        Cell::from("Price"),
        Cell::from("Qty"),
        Cell::from("TP"),      // 新增
        Cell::from("SL"),      // 新增
        Cell::from("Reduce"),  // 新增 (reduce_only)
        Cell::from("Time"),    // 新增 (created_at)
        Cell::from("Status"),
    ]);
    ```
  - 数据提取逻辑:
    - `reduce_only`: `order.reduce_only` → 显示 "Yes" / "No"
    - `created_at`: `order.created_at` → 格式化为 "HH:MM:SS"（取时间部分）
    - TP/SL: 从 `order.payload` 解析（如果格式为 JSON）:
      ```rust
      // 尝试解析 payload 中的 tp/sl
      let (tp, sl) = if let Some(ref payload) = order.payload {
          parse_tp_sl_from_payload(payload)
      } else {
          (None, None)
      };
      ```
  - 列宽调整（适配新列）:
    ```rust
    let widths = [
        Constraint::Length(10), // Symbol
        Constraint::Length(6),  // Side
        Constraint::Length(8),  // Type
        Constraint::Length(10), // Price
        Constraint::Length(10), // Qty
        Constraint::Length(8),  // TP
        Constraint::Length(8),  // SL
        Constraint::Length(6),  // Reduce
        Constraint::Length(8),  // Time
        Constraint::Length(8),  // Status
    ];
    ```

  **必须不做**:
  - 如果 API 不支持 TP/SL 字段，显示 "-"
  - 不要为解析失败 panic（显示 "-")

  **参考**:
  - `standx_point_adapter::Order` 结构体（reduce_only, created_at, payload）
  - 原 `mod.rs:447-504` draw_open_orders_table

  **验收标准**:
  - [x] Orders 表格显示 TP, SL, Reduce, Time 列
  - [x] reduce_only 显示 Yes/No
  - [x] created_at 显示为 HH:MM:SS 格式
  - [x] TP/SL 从 payload 解析，失败显示 "-"
  - [x] 所有列对齐正确

  **Agent-Executed QA**:
  ```
  Scenario: Open Orders 扩展字段显示正常
    Tool: interactive_bash (tmux)
    Preconditions: 有挂单的任务
    Steps:
      1. cargo run -p standx-point-mm-strategy -- --tui &
      2. Wait: 3s
      3. Select a task with open orders
      4. Wait: 3s
      5. Assert: Open Orders table headers include "TP" "SL" "Reduce" "Time"
      6. Assert: reduce_only shows "Yes" or "No"
      7. Assert: Time column shows HH:MM:SS format
      8. Screenshot: .sisyphus/evidence/tui-orders-extended.png
    Expected Result: 所有新列可见且数据正确
  ```

  **提交**: YES
  - Message: `feat(tui): extend open orders display with TP/SL and metadata`
  - Files: `src/tui/ui/orders.rs`

### Wave 5: 收尾和验证

- [ ] **11. 清理原 mod.rs 并验证完整功能**

  **做什么**:
  - 用新模块化代码替换原 `mod.rs` 内容
  - 删除原 `mod.rs` 中的旧实现（保留备份注释）
  - 确保 `pub use` 导出正确
  - 运行完整测试:
    ```bash
    cargo clippy -p standx-point-mm-strategy --all-targets -- -D warnings
    cargo test -p standx-point-mm-strategy
    cargo build -p standx-point-mm-strategy --release
    ```

  **必须不做**:
  - 不要删除原 mod.rs 文件（只是替换内容）

  **验收标准**:
  - [x] 原 mod.rs 已替换为新实现
  - [ ] `cargo clippy` 无警告
  - [x] `cargo test` 全通过
  - [x] Release 构建成功

  **Agent-Executed QA**:
  ```
  Scenario: 完整构建和测试通过
    Tool: Bash
    Steps:
      1. cargo clippy -p standx-point-mm-strategy --all-targets -- -D warnings
      2. Assert: exit code 0
      3. cargo test -p standx-point-mm-strategy
      4. Assert: all tests pass
      5. cargo build -p standx-point-mm-strategy --release
      6. Assert: build success
  ```

  **提交**: YES
  - Message: `refactor(tui): finalize modular architecture`
  - Files: `src/tui/mod.rs`

- [ ] **12. 完整功能验证和文档**

  **做什么**:
  - 创建 `crates/standx-point-mm-strategy/src/tui/README.md`:
    ```markdown
    # TUI Module

    ## 架构
    - `app.rs` - 应用状态和主循环
    - `events.rs` - 事件处理
    - `state.rs` - 数据刷新
    - `ui/` - UI 渲染组件
      - `modal/` - 弹窗组件

    ## 快捷键
    - `Tab/l` - 切换 Tab
    - `1/2/3` - 跳转到指定 Tab
    - `a` - 创建 Account
    - `t` - 创建 Task
    - `s` - 启动任务
    - `x` - 停止任务
    - `r` - 刷新
    - `q` - 退出
    - `Esc` - 关闭弹窗
    ```
  - 更新 AGENTS.md（如有 TUI 相关说明）
  - 完整 TUI 测试验证:
    - 所有 Tab 切换
    - Account/Task 创建流程
    - 价格详情刷新
    - 订单字段显示

  **验收标准**:
  - [x] README.md 创建完成
  - [ ] 所有快捷键工作正常
  - [x] 所有 Tab 工作正常
  - [ ] Modal 流程完整测试

  **Agent-Executed QA**:
  ```
  Scenario: 完整功能验证
    Tool: interactive_bash (tmux)
    Steps:
      1. cargo run -p standx-point-mm-strategy -- --tui &
      2. Test all tabs: Tab, 1, 2, 3
      3. Test modals: a (Account), t (Task), Esc to close
      4. Test task controls: s (start), x (stop)
      5. Test navigation: Up/Down
      6. Test quit: q
      7. Screenshot: .sisyphus/evidence/tui-final-verification.png
    Expected Result: 所有功能正常工作
  ```

  **提交**: YES
  - Message: `docs(tui): add module documentation and final verification`
  - Files: `src/tui/README.md`

---

## 提交策略

| 任务 | 提交信息 | 文件 | 验证 |
|------|----------|------|------|
| 1 | `refactor(tui): create modular file structure` | tui/*.rs | cargo check |
| 2 | `refactor(tui): migrate AppState and event loop` | app.rs, events.rs, state.rs | cargo check |
| 3 | `refactor(tui): split UI rendering components` | ui/*.rs | cargo check |
| 4 | `feat(tui): add tab switching system` | app.rs, ui/layout.rs | 手动测试 Tab |
| 5 | `feat(tui): update layout and styling` | ui/layout.rs | 手动测试布局 |
| 6 | `feat(tui): add modal framework` | ui/modal/mod.rs | 手动测试 Modal |
| 7 | `feat(tui): add create account modal` | ui/modal/create_account.rs | 手动测试弹窗 |
| 8 | `feat(tui): add create task modal` | ui/modal/create_task.rs | 手动测试弹窗 |
| 9 | `feat(tui): display price details` | state.rs, ui/account.rs | 手动测试价格显示 |
| 10 | `feat(tui): extend open orders display` | ui/orders.rs | 手动测试订单表格 |
| 11 | `refactor(tui): finalize modular architecture` | mod.rs | cargo test |
| 12 | `docs(tui): add module documentation` | README.md | 文档完整 |

---

## 成功标准

### 验证命令
```bash
# 格式检查
cargo fmt --all -- --check

# 静态检查
cargo clippy -p standx-point-mm-strategy --all-targets -- -D warnings

# 测试
cargo test -p standx-point-mm-strategy

# 构建
cargo build -p standx-point-mm-strategy --release

# TUI 运行测试
cargo run -p standx-point-mm-strategy -- --tui
```

### 最终检查清单
- [x] 文件架构规范化（10+ 模块文件）
- [x] Tab 切换系统工作正常（Dashboard/Logs/Create）
- [x] Account 创建弹窗可用（快捷键 `a`）
- [x] Task 创建弹窗可用（快捷键 `t`）
- [x] 选择组件工作正常（Chain/RiskLevel 等）
- [x] 价格详情显示（Mark/Last/Min）
- [x] Open Orders 扩展字段（TP/SL/Reduce/Time）
- [ ] 原有功能完整保留（启动/停止/刷新/导航）
- [ ] 最小 80x24 终端正常工作
- [ ] 所有测试通过
- [ ] clippy 无警告

---

## 风险与缓解

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|----------|
| TP/SL 解析失败 | 中 | 低 | 失败时显示 "-"，不 panic |
| 终端尺寸不足 | 低 | 中 | 最小 80x24 要求，不足时提示 |
| 热键冲突 | 低 | 中 | Modal 状态禁用其他热键 |
| 文件拆分导致编译错误 | 中 | 中 | 每任务后验证 cargo check |
| 数据刷新性能问题 | 低 | 低 | 保持 3s 刷新间隔，不复用 WebSocket |

---

## 附录

### A. 模块依赖图
```
tui/
├── mod.rs (入口，导出公共 API)
│   ├── app.rs (依赖: state, events, ui)
│   ├── terminal.rs (无依赖)
│   ├── events.rs (依赖: app, state, ui/modal)
│   ├── state.rs (依赖: app, storage, adapter)
│   └── ui/
│       ├── mod.rs (导出所有渲染函数)
│       ├── layout.rs (依赖: 无)
│       ├── account.rs (依赖: app, state)
│       ├── task_list.rs (依赖: app, state)
│       ├── positions.rs (依赖: app, state)
│       ├── orders.rs (依赖: app, state)
│       ├── logs.rs (依赖: app)
│       └── modal/
│           ├── mod.rs (导出 Modal 组件)
│           ├── create_account.rs (依赖: app, state, storage)
│           └── create_task.rs (依赖: app, state, storage)
```

### B. UI 状态机
```
Idle ──a──→ CreateAccountModal ──Esc/Submit──→ Idle
  │
  ├──t──→ CreateTaskModal ──Esc/Submit──→ Idle
  │
  ├──Tab/1/2/3──→ Switch Tab
  │
  └──q──→ Quit

CreateAccountModal 和 CreateTaskModal 状态下:
- 禁用全局热键 (s/x/r/q)
- 仅响应: Tab/Up/Down/Enter/Esc
```

### C. 快捷键映射
| 键 | 全局 | Dashboard | Logs Tab | Create Tab | Modal |
|----|------|-----------|----------|------------|-------|
| `q` | 退出 | ✓ | ✓ | ✓ | - |
| `r` | 刷新 | ✓ | ✓ | - | - |
| `Tab` / `l` | 切换 Tab | ✓ | ✓ | ✓ | - |
| `1/2/3` | 跳转 Tab | ✓ | ✓ | ✓ | - |
| `s` | 启动任务 | ✓ | - | - | - |
| `x` | 停止任务 | ✓ | - | - | - |
| `a` | 创建 Account | ✓ | - | - | - |
| `t` | 创建 Task | ✓ | - | - | - |
| `Up/Down` | 导航 | ✓ | - | - | ✓ |
| `Esc` | 关闭 | - | - | - | ✓ |
| `Enter` | 确认 | - | - | - | ✓ |

### D. 数据字段来源
| 字段 | 来源 | 备注 |
|------|------|------|
| Mark Price | API: query_symbol_info | 3s 刷新 |
| Last Price | API: query_symbol_info | 3s 刷新 |
| Min Price | API: query_symbol_info | 3s 刷新 |
| TP Price | Order.payload JSON | 解析失败显示 "-" |
| SL Price | Order.payload JSON | 解析失败显示 "-" |
| Reduce Only | Order.reduce_only | bool → Yes/No |
| Created At | Order.created_at | 格式化为 HH:MM:SS |
