## Architecture
- **Position**: TUI 组件层，负责各 UI 区块与对话框的渲染。
- **Logic**: AppState/ModalType -> render 函数 -> ratatui widgets 输出。
- **Constraints**: 仅做展示与布局；不执行业务逻辑、不做 I/O 与网络访问。

## Members
- `account_form.rs`: 账户创建/编辑对话框渲染与表单验证逻辑。
- `detail_view.rs`: 详情区渲染（根据焦点与侧栏模式显示提示或详情）。
- `help.rs`: 帮助弹层与快捷键说明渲染。
- `menu_bar.rs`: 底部菜单栏快捷键提示渲染。
- `modal.rs`: 模态窗口渲染入口占位。
- `sidebar.rs`: 侧栏列表（账户/任务）渲染。
- `status_bar.rs`: 顶部状态栏（模式与状态消息）渲染。
- `mod.rs`: 组件模块声明聚合。

## Conventions (Optional)
- 渲染函数保持纯函数输入（Frame/Rect/State），避免隐藏副作用。
- 文本与颜色样式保持全局一致性。
