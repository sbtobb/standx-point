## Architecture
- **Position**: 策略核心实现目录（配置解析、市场数据分发、任务生命周期与策略骨架）。
- **Logic**: 配置 -> 任务构建 -> 订阅价格 -> 任务 loop 处理 -> 退出清理。
- **Constraints**: `task.rs` 只负责生命周期与清理动作，不实现做市报价/下单策略；对 StandX REST/WS 的调用必须通过 `standx-point-adapter`。

## Members
- `lib.rs`: crate 模块声明与对外 re-export。
- `main.rs`: 二进制入口（CLI 解析、配置加载、日志初始化与优雅退出）。
- `config.rs`: YAML 配置解析与 `StrategyConfig`/`TaskConfig` 定义。
- `market_data.rs`: MarketDataHub（watch channel 分发价格给多个任务）。
- `task.rs`: Task/TaskManager 生命周期管理（startup/shutdown、panic isolation、graceful shutdown）。
- `strategy.rs`: 做市策略骨架（报价逻辑占位）。
- `risk.rs`: 风险管理实现（价格跳变/深度/仓位/成交速率/点差守卫）。
- `order_state.rs`: 订单状态与本地视图占位（用于后续幂等/撤单跟踪）。

## Conventions (Optional)
- 文件头部使用 Fractal Context header（[INPUT]/[OUTPUT]/[POS]/[UPDATE]）。
- 单元测试优先使用 `wiremock` 注入 HTTP base URL，避免真实 API 依赖。
