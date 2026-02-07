## Architecture
- **Position**: 该目录存放 mm-strategy 的示例配置文件。
- **Logic**: YAML 示例 -> `StrategyConfig` 反序列化 -> 任务启动流程。
- **Constraints**: 只允许占位符凭据，禁止真实密钥；内容需与配置结构保持一致。

## Members
- `config.yaml`: 旧版单文件示例（含单/多任务注释块）。
- `single_task.yaml`: 单任务示例配置。
- `multi_task.yaml`: 多任务示例配置。

## Conventions (Optional)
- 使用占位符凭据与 ASCII 字符，便于复制修改。
