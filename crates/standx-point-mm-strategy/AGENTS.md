## Architecture
- **Position**: 该 crate 的入口目录，承载做市策略机器人（mm-strategy）的可执行与库形态。
- **Logic**: `examples/*.yaml` -> `StrategyConfig` -> `TaskManager` -> per-task lifecycle (startup/cancel -> run loop -> shutdown cleanup).
- **Constraints**: StandX 协议细节与鉴权/签名逻辑必须留在 `standx-point-adapter`；本 crate 只做策略编排与生命周期管理；测试不得调用真实外部网络。

## Members
- `Cargo.toml`: crate 依赖与构建配置。
- `examples/config.yaml`: 示例配置文件（任务列表、symbol、鉴权信息、风险与 sizing 参数）。
- `examples/single_task.yaml`: 单任务示例配置。
- `examples/multi_task.yaml`: 多任务示例配置。
- `src/`: 核心实现（配置、市场数据分发、任务生命周期、策略骨架）。

## Conventions (Optional)
- 生命周期逻辑与策略下单逻辑分离：`task.rs` 负责启动/退出/清理，不负责做市报价细节。
- 异步运行时使用 Tokio；错误上抛使用 `anyhow::Result`。
