## Architecture
- **Position**: TUI UI 组件层，提供布局与面板渲染。
- **Logic**: 组件状态/快照 -> Widget 渲染输出。
- **Constraints**: 仅渲染与布局，不做 IO 或网络调用。

## Members
- `mod.rs`: UI 模块入口与子模块声明。
- `layout.rs`: 布局辅助占位。
- `account.rs`: 账户汇总面板渲染。
- `task_list.rs`: 任务列表面板渲染。
- `positions.rs`: 持仓表格渲染。
- `orders.rs`: 订单表格渲染。
- `logs.rs`: 日志面板渲染。
- `modal/`: 模态框组件。
