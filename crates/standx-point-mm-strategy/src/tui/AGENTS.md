## Architecture
- **Position**: TUI 模块目录，负责终端界面与交互入口。
- **Logic**: 运行态数据 -> 本地状态 -> UI 渲染 -> 输入事件驱动。
- **Constraints**: 仅处理 TUI 逻辑；协议/请求调用必须通过现有模块封装。

## Members
- `mod.rs`: TUI 模块入口与公开 re-export。
- `runtime.rs`: TUI 事件循环、渲染编排与日志缓冲实现。
- `terminal.rs`: 终端生命周期守卫（raw mode 与 alternate screen）。
- `app.rs`: TUI AppState 与运行时快照/任务控制逻辑。
- `events.rs`: TUI 事件路由占位。
- `state.rs`: TUI 状态模型占位。
- `ui/`: UI 组件与布局模块。
