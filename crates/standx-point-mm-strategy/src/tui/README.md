# TUI Module

## 架构
- `app.rs` - 应用状态与任务控制
- `events.rs` - 键盘事件路由与快捷键处理
- `state.rs` - 数据刷新与快照构建
- `runtime.rs` - 运行时循环与绘制编排
- `terminal.rs` - 终端生命周期守卫
- `ui/` - UI 组件与布局
  - `modal/` - 弹窗组件与输入处理

## 快捷键
- `Tab` / `l` - 切换 Tab
- `1` / `2` / `3` - 直接跳转 Tab
- `a` - 创建 Account
- `t` - 创建 Task
- `s` - 启动任务
- `x` - 停止任务
- `r` - 刷新
- `q` - 退出
- `Esc` - 关闭弹窗

## 运行验证
```bash
cargo run -p standx-point-mm-strategy -- --tui
```
