## Architecture
- **Position**: GUI 本地持久化层，负责 SQLite 数据落盘与读取。
- **Logic**: Database pool -> schema migrations -> CRUD -> state models.
- **Constraints**: 必须使用 schema.sql；凭证入库前加密/取回解密；连接开启 WAL。

## Members
- `schema.sql`: SQLite 表结构与索引定义。
- `mod.rs`: 数据库连接池、迁移与 CRUD/加解密实现。
