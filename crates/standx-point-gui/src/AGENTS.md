## Architecture
- **Position**: GUI crate 源码根目录，承载界面、状态与本地持久化入口。
- **Logic**: main -> ui -> state/task -> db persistence.
- **Constraints**: UI 使用 GPUI；业务逻辑尽量在 adapter/core；本地存储通过 db 模块。

## Members
- `main.rs`: GPUI 应用入口与窗口初始化。
- `lib.rs`: 模块公开入口（state/task/db）。
- `ui/`: 视图层与组件构建。
- `state/`: GUI 全局状态与运行时模型。
- `task/`: 任务状态机与运行逻辑。
- `db/`: SQLite 持久化与迁移。
