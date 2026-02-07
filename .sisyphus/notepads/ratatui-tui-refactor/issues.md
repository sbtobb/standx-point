- 2026-02-04: 任务测试中 wiremock 池化服务器可能提前关闭连接，改为 builder 启动独立实例并等待请求计数以避免关闭竞态。

- 2026-02-04: `cargo test --lib` 偶发 reqwest/wiremock 连接中断；在 `task.rs` 的 wiremock-heavy tests 上加全局 async mutex 锁序列化后，连续 3 次全量测试通过。
- 2026-02-05: 任务测试间歇性失败 `connection closed before message completed`，通过添加 static async lock 序列化 wiremock 测试和在 send_json 方法中添加重试机制消除跨测试干扰。

# Phase 6 Implementation Status (Keyboard Navigation and Menu System)

- [x] F1 help overlay: Partially implemented. Help overlay exists (`src/ui/components/help.rs`) and can be toggled (`src/app/mod.rs:419`), but it's triggered by `AppState::show_help` flag, which isn't currently cleared by a second F1 press (it only closes on "any key" in logic).
- [x] vi keys (hjkl): Implemented in Normal mode (`src/app/mod.rs:393`).
- [ ] vi keys (j/k) for Sidebar: Only 'j'/'k' and arrow keys work; 'g'/'G' for top/bottom are missing.
- [x] focus cycling (Tab): Implemented for form fields (`src/app/state.rs:73, 189`) but NOT for top-level panes (hjkl used instead).
- [ ] gg/G (Jump to Top/Bottom): Missing in sidebar navigation.
- [x] mode indicator: Implemented in status bar (`src/ui/components/status_bar.rs:19`).
- [ ] keypress flash: Missing. No visual feedback on keypress currently implemented in `src/app/mod.rs` or `src/ui/render`.
- [ ] spinner: Missing. No loading or activity spinner implemented in status bar.


## Terminal Resize and Min-Size Handling Audit

- **Resize Handling**: Not explicitly implemented in the application event loop. The app uses crossterm_event::poll and read() in a blocking task, but it only forwards AppEvent::Key to the main loop. Event::Resize is currently ignored in crates/standx-point-mm-strategy/src/app/mod.rs (lines 78-87).
- **Minimum Terminal Size Check**: No implementation found for a minimum terminal size check (e.g., 80x24). The UI rendering logic in crates/standx-point-mm-strategy/src/ui/mod.rs uses ratatui::layout with Constraint::Length and Constraint::Fill, which will attempt to render regardless of the terminal dimensions.
- **Overlay/Message**: No "Terminal too small" message or overlay exists in the current codebase.

