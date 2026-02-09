## Architecture
- **Position**: TUI 模态框组件目录。
- **Logic**: 模态状态 -> 弹窗渲染与交互输出。
- **Constraints**: 仅渲染与输入处理，不做 IO 或网络调用。

## Members
- `mod.rs`: 模态模块入口与子模块声明。
- `create_account.rs`: 创建账户模态占位。
- `create_task.rs`: 创建任务模态占位。
