# StandX API Adapter - Work Plan

## TL;DR

> **Quick Summary**: 构建一个 Rust HTTP/WebSocket 客户端库，封装 StandX Perps 永续合约 API。提供认证、交易、查询、市场数据接口，支持 EVM/Solana 双链钱包签名。
>
> **Deliverables**:
> - `StandxClient` - HTTP API 客户端（认证 + 所有端点）
> - `StandxWebSocket` - WebSocket 客户端（市场数据 + 订单流）
> - `AuthManager` - JWT + Ed25519 签名管理
> - 完整类型定义（请求/响应/枚举）
> - 错误处理体系
> - 单元测试套件
>
> **Estimated Effort**: Large (~3-4 天)
> **Parallel Execution**: YES - 分为 4 个波次
> **Critical Path**: 类型定义 → HTTP 客户端基础 → 认证模块 → 端点实现

---

## Context

### Original Request
用户需要将 StandX Perps API 封装到 `crates/standx-point-adapter`，提供 Rust API 访问永续合约交易功能。

### API Structure Overview
- **Base URLs**: `https://api.standx.com` (认证), `https://perps.standx.com` (交易)
- **WebSocket**: `wss://perps.standx.com/ws-stream/v1` (市场流), `/ws-api/v1` (订单响应)
- **Authentication**: 多步骤流程（Ed25519 临时密钥 + 钱包签名 + JWT）
- **Endpoints**: ~30 个 HTTP 端点 + WebSocket 订阅

### Metis Review Findings (已处理)
**识别的关键差距**:
1. 需要明确的端点范围界定（v1 必须实现 vs 可选）
2. 钱包签名应通过 callback/trait 实现，不内置钱包逻辑
3. 客户端必须是 stateless，不做订单/仓位状态跟踪
4. 使用 `rust_decimal` 而非 `f64` 处理价格/数量
5. 暂时不实现自动重试逻辑

**应用的防护栏**:
- 范围限定：仅 HTTP/WS 客户端 + 认证模块，不构建订单管理系统
- 抽象层级：薄封装，1:1 映射 API 端点
- 状态策略：无状态客户端，不跟踪仓位/订单
- 运行时：仅支持 Tokio，不做运行时无关设计
- 钱包集成：通过 trait/callback 接收签名，不自实现钱包

---

## Work Objectives

### Core Objective
实现一个完整的、类型安全的 Rust API 客户端，支持：
1. EVM/Solana 双链钱包认证
2. 所有核心交易操作（下单、撤单、改杠杆）
3. 用户数据查询（订单、仓位、余额）
4. 市场数据访问（深度、价格、K线）
5. WebSocket 实时数据流

### Concrete Deliverables
- `StandxClient` 结构体及方法（HTTP）
- `StandxWebSocket` 结构体及方法（WebSocket）
- 完整的请求/响应类型定义（~50+ structs）
- 枚举类型（Side, OrderType, TimeInForce, Status, MarginMode, Chain）
- `StandxError` 错误类型
- `WalletSigner` trait（钱包签名回调）
- `AuthManager` 认证管理器
- `RequestSigner` Ed25519 请求签名器

### Definition of Done
- [x] 所有 v1 必需端点实现并通过单元测试
- [x] 认证流程完整实现（prepare-signin → login → token refresh）
- [x] WebSocket 连接/订阅/消息处理正常工作
- [x] 所有公共类型有完整 Rustdoc 文档
- [x] 示例代码可编译运行
- [x] 测试覆盖率 > 70%

### Must Have (v1 必需)
**认证模块**:
- Ed25519 密钥对生成
- `prepare-signin` 获取签名数据
- `login` 获取 JWT
- 请求体签名（Body Signature）

**交易端点** (Trade Endpoints):
- `POST /api/new_order` - 创建订单
- `POST /api/cancel_order` - 取消订单
- `POST /api/change_leverage` - 修改杠杆

**用户端点** (User Endpoints):
- `GET /api/query_orders` - 查询用户订单
- `GET /api/query_open_orders` - 查询开放订单
- `GET /api/query_positions` - 查询用户仓位
- `GET /api/query_balance` - 查询用户余额

**公共端点** (Public Endpoints):
- `GET /api/query_symbol_info` - 交易对信息
- `GET /api/query_symbol_price` - 价格查询
- `GET /api/query_depth_book` - 深度数据
- `GET /api/kline/history` - K线历史

**WebSocket**:
- 市场流连接
- 价格/深度/交易订阅
- 订单响应流连接
- 自动重连机制

### Must NOT Have (Guardrails)
- ❌ 订单状态机管理（自动跟踪订单生命周期）
- ❌ 仓位计算器/PnL 计算
- ❌ 自动重试逻辑（v1 不做）
- ❌ 响应缓存（返回最新数据）
- ❌ 多账户支持（一个客户端 = 一个账户）
- ❌ 批量下单（除非 API 原生支持）
- ❌ 运行时无关设计（仅 Tokio）
- ❌ 内置钱包实现（仅提供签名 trait）
- ❌ 历史数据分析/统计端点

---

## Verification Strategy

### Test Infrastructure Assessment
**当前状态**: 无现有测试基础设施
**决策**: 设置测试基础设施并使用 TDD 模式

### Test Setup
**Task 0**: 配置测试基础设施
- 安装: `cargo add --dev tokio wiremock rstest`
- 配置: 创建 `tests/` 目录结构
- 验证: `cargo test` → 测试框架正常工作

### TDD Pattern (RED-GREEN-REFACTOR)
每个任务遵循:
1. **RED**: 先写测试（预期失败）
2. **GREEN**: 实现代码使测试通过
3. **REFACTOR**: 清理代码，保持测试通过

### 自动化验证方法

**HTTP 端点测试** (使用 wiremock):
```rust
// mock server 验证请求格式
// 验证响应解析
// 验证错误处理
```

**认证测试**:
```rust
// 使用确定性 Ed25519 密钥（非随机）
// 验证签名算法正确性
// 验证 JWT 解析
```

**WebSocket 测试** (使用 tokio-test):
```rust
// mock WebSocket server
// 验证订阅/取消订阅
// 验证消息解析
```

**类型测试**:
```rust
// 验证 serde 序列化/反序列化
// 验证 decimal 精度处理
```

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 - 基础与类型 (可立即开始):
├── Task 1: 配置依赖和 crate 结构
├── Task 2: 定义核心类型和枚举
└── Task 3: 配置测试基础设施

Wave 2 - HTTP 客户端基础 (依赖 Wave 1):
├── Task 4: HTTP 客户端基础结构
└── Task 5: 请求/响应中间件

Wave 3 - 认证模块 (依赖 Wave 2):
├── Task 6: Ed25519 签名器
├── Task 7: JWT 管理器
└── Task 8: 认证流程实现

Wave 4 - 端点实现 (依赖 Wave 3):
├── Task 9: 公共端点 (market data)
├── Task 10: 交易端点 (orders)
├── Task 11: 用户端点 (positions/balances)
└── Task 12: WebSocket 客户端

Wave 5 - 完善与文档 (依赖 Wave 4):
├── Task 13: 错误处理完善
├── Task 14: 示例和文档
└── Task 15: 集成测试

Critical Path: 1 → 4 → 6 → 8 → 10 → 12
Parallel Speedup: ~35% faster than sequential
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 1 | None | 2, 3, 4 | 2, 3 |
| 2 | None | 4, 5, 6 | 1, 3 |
| 3 | None | 4, 6, 7, 8 | 1, 2 |
| 4 | 1, 2 | 5, 6, 7 | 3 |
| 5 | 4 | 8, 9, 10, 11 | 6, 7 |
| 6 | 3, 4 | 8 | 5, 7 |
| 7 | 3, 4 | 8 | 5, 6 |
| 8 | 5, 6, 7 | 9, 10, 11 | None |
| 9 | 8 | 12 | 10, 11 |
| 10 | 8 | 12 | 9, 11 |
| 11 | 8 | 12 | 9, 10 |
| 12 | 9, 10, 11 | 14, 15 | None |
| 13 | 12 | 14, 15 | None |
| 14 | 12, 13 | 15 | None |
| 15 | 12, 13, 14 | None | None |

---

## TODOs

### Task 0: 配置测试基础设施

**What to do**:
- 添加测试依赖到 Cargo.toml
- 创建测试目录结构
- 配置 wiremock 用于 HTTP mock
- 创建第一个通过测试作为基线

**Must NOT do**:
- 不要写实际业务代码
- 不要跳过此任务

**Recommended Agent Profile**:
- **Category**: `quick`
- **Skills**: `uv-package-manager`（管理依赖）

**Parallelization**:
- **Can Run In Parallel**: NO（必须在所有测试任务之前）
- **Blocked By**: None
- **Blocks**: All tasks with tests

**Acceptance Criteria**:
- [x] `cargo test` 命令可执行
- [x] `cargo test` 显示测试框架正常（0 tests run 但无错误）
- [x] 依赖 `tokio-test`, `wiremock`, `rstest` 已安装

**Commit**: YES
- Message: `chore(adapter): setup test infrastructure`
- Files: `Cargo.toml`, `tests/common/mod.rs`

---

### Task 1: 配置依赖和 crate 结构

**What to do**:
- 更新 `Cargo.toml` 添加所有必要依赖
- 创建模块文件结构 (`src/auth/`, `src/http/`, `src/ws/`, `src/types/`)
- 创建 `lib.rs` 导出公共 API

**依赖清单**:
```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1.43", features = ["full"] }
tokio-tungstenite = { version = "0.26", features = ["native-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
ed25519-dalek = { version = "2.1", features = ["rand_core"] }
rand = "0.8"
base64 = "0.22"
bs58 = "0.5"
uuid = { version = "1.12", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2.0"
url = "2.5"
tracing = "0.1"
rust_decimal = { version = "1.36", features = ["serde"] }

[dev-dependencies]
tokio-test = "0.4"
wiremock = "0.6"
rstest = "0.24"
```

**Must NOT do**:
- 不要实现具体逻辑
- 不要添加未使用的依赖

**Recommended Agent Profile**:
- **Category**: `quick`

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 1)
- **Parallel Group**: Wave 1 (with Task 2, 3)
- **Blocks**: Task 4, 5
- **Blocked By**: None

**Acceptance Criteria**:
- [x] `cargo check` 通过（无代码但依赖解析成功）
- [x] 目录结构创建完成
- [x] 模块声明正确（无编译错误）

**Commit**: YES
- Message: `chore(adapter): setup crate structure and dependencies`
- Files: `Cargo.toml`, `src/lib.rs`, `src/auth/mod.rs`, `src/http/mod.rs`, `src/ws/mod.rs`, `src/types/mod.rs`

---

### Task 2: 定义核心类型和枚举

**What to do**:
- 定义所有枚举类型（Side, OrderType, TimeInForce, OrderStatus, MarginMode, Chain）
- 定义基础请求/响应结构体（带 serde 属性）
- 定义 Decimal 处理类型

**类型清单**:
- `enum Side { Buy, Sell }`
- `enum OrderType { Market, Limit, ... }`
- `enum TimeInForce { Gtc, Ioc, Fok, ... }`
- `enum OrderStatus { New, Open, Filled, ... }`
- `enum MarginMode { Cross, Isolated }`
- `enum Chain { Bsc, Solana }`
- `struct Symbol`, `struct Order`, `struct Position`, `struct Balance`, ...

**Must NOT do**:
- 不要添加业务逻辑方法（仅数据定义）
- 不要跳过任何 API 文档中的字段

**Recommended Agent Profile**:
- **Category**: `unspecified-high`
- **Reason**: 需要仔细阅读 API 文档，确保字段完整

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 1)
- **Parallel Group**: Wave 1 (with Task 1, 3)
- **Blocks**: Task 4, 5, 6, 7, 8
- **Blocked By**: None

**References**:
- https://docs.standx.com/standx-api/perps-reference - 枚举值完整列表

**Acceptance Criteria**:
- [x] 所有 v1 必需端点涉及的类型已定义
- [x] 类型可通过 `cargo check` 编译
- [x] 每个类型有基本的 serde 测试

**Commit**: YES
- Message: `feat(types): define core types and enums`
- Files: `src/types/*.rs`
- Pre-commit: `cargo test types`

---

### Task 3: 实现 Ed25519 签名器

**What to do**:
- 实现 `Ed25519Signer` 结构体
- 实现密钥对生成（随机）
- 实现签名方法
- 实现 base58 编码的公钥导出（用于 requestId）

**API**:
```rust
pub struct Ed25519Signer {
    keypair: SigningKey,
}

impl Ed25519Signer {
    pub fn generate() -> Self;
    pub fn from_secret_key(bytes: &[u8]) -> Result<Self>;
    pub fn sign(&self, message: &[u8]) -> Signature;
    pub fn public_key_base58(&self) -> String;
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool;
}
```

**Must NOT do**:
- 不要硬编码密钥
- 不要暴露私钥方法

**Recommended Agent Profile**:
- **Category**: `unspecified-high`
- **Skills**: 需要密码学知识

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 1)
- **Parallel Group**: Wave 1 (with Task 1, 2)
- **Blocks**: Task 8
- **Blocked By**: None

**Acceptance Criteria**:
- [x] 密钥生成可重复测试
- [x] 签名可被验证
- [x] base58 编码与 API 示例一致
- [x] 单元测试覆盖所有方法

**Commit**: YES
- Message: `feat(auth): implement Ed25519 signer`
- Files: `src/auth/signer.rs`
- Pre-commit: `cargo test signer`

---

### Task 4: HTTP 客户端基础结构

**What to do**:
- 创建 `StandxClient` 结构体
- 实现基础 HTTP 方法（GET/POST）
- 配置 reqwest 客户端（超时、连接池）
- 实现 base URL 管理（区分 auth/trading）

**结构**:
```rust
pub struct StandxClient {
    http_client: reqwest::Client,
    auth_base_url: Url,
    trading_base_url: Url,
    credentials: Option<Credentials>,
}

pub struct Credentials {
    jwt_token: String,
    wallet_address: String,
    chain: Chain,
}
```

**Must NOT do**:
- 不要实现具体端点
- 不要处理响应解析

**Recommended Agent Profile**:
- **Category**: `unspecified-high`
- **Skills**: `rust-best-practices`（HTTP 客户端模式）

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 2)
- **Parallel Group**: Wave 2 (with Task 5)
- **Blocks**: Task 6, 7, 8, 9, 10, 11
- **Blocked By**: Task 1, 2

**Acceptance Criteria**:
- [x] 客户端可构建
- [x] 可通过 wiremock 测试基础 HTTP 调用
- [x] 支持自定义超时配置

**Commit**: YES
- Message: `feat(http): implement base client structure`
- Files: `src/http/client.rs`, `src/http/config.rs`

---

### Task 5: 请求/响应中间件

**What to do**:
- 实现请求构建器
- 实现响应解析（处理 decimal 字符串）
- 实现错误处理转换
- 实现请求/响应日志（tracing）

**API**:
```rust
impl StandxClient {
    async fn request<T: Serialize>(&self, method: Method, endpoint: &str, body: Option<T>) -> Result<Response>;
    async fn parse_response<T: DeserializeOwned>(&self, response: Response) -> Result<T>;
}
```

**Must NOT do**:
- 不要添加重试逻辑
- 不要在中间件中处理认证

**Recommended Agent Profile**:
- **Category**: `unspecified-high`

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 2)
- **Parallel Group**: Wave 2 (with Task 4)
- **Blocks**: Task 8, 9, 10, 11
- **Blocked By**: Task 4

**Acceptance Criteria**:
- [x] 请求构建正确
- [x] Decimal 字符串解析正确
- [x] 错误响应可转换为 `StandxError`
- [x] 日志输出使用 tracing

**Commit**: YES
- Message: `feat(http): implement request/response middleware`
- Files: `src/http/middleware.rs`, `src/error.rs`

---

### Task 6: JWT 管理器

**What to do**:
- 实现 `JwtManager` 结构体
- 实现 token 存储和获取
- 实现 token 过期检查
- 实现自动刷新逻辑（预留接口）

**API**:
```rust
pub struct JwtManager {
    token: Arc<RwLock<Option<TokenData>>>,
}

struct TokenData {
    token: String,
    expires_at: DateTime<Utc>,
    address: String,
    chain: Chain,
}

impl JwtManager {
    pub fn set_token(&self, token: String, expires_seconds: u64) -> Result<()>;
    pub fn get_token(&self) -> Option<String>;
    pub fn is_expired(&self) -> bool;
    pub fn credentials(&self) -> Option<Credentials>;
}
```

**Must NOT do**:
- 不要实现自动刷新（v1 手动刷新）
- 不要存储私钥

**Recommended Agent Profile**:
- **Category**: `unspecified-high`

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 3)
- **Parallel Group**: Wave 3 (with Task 7, 8)
- **Blocks**: Task 8, 9, 10, 11, 12
- **Blocked By**: Task 3, 4

**Acceptance Criteria**:
- [x] Token 可存储和获取
- [x] 过期时间检查准确
- [x] 线程安全（Arc<RwLock>）
- [x] 单元测试覆盖

**Commit**: YES
- Message: `feat(auth): implement JWT manager`
- Files: `src/auth/jwt.rs`

---

### Task 7: 钱包签名 Trait

**What to do**:
- 定义 `WalletSigner` trait
- 为 EVM 钱包实现示例（使用 ethers-rs 或回调）
- 为 Solana 钱包实现示例
- 提供测试用的 mock 实现

**API**:
```rust
#[async_trait]
pub trait WalletSigner: Send + Sync {
    fn chain(&self) -> Chain;
    fn address(&self) -> &str;
    async fn sign_message(&self, message: &str) -> Result<String>;
}

// 示例实现 - 用户需自行实现或使用示例
pub struct EvmWalletSigner { ... }
pub struct SolanaWalletSigner { ... }
```

**Must NOT do**:
- 不要强制用户使用特定钱包库
- 不要在 trait 中暴露私钥

**Recommended Agent Profile**:
- **Category**: `unspecified-high`
- **Skills**: 需要理解 EVM/Solana 签名差异

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 3)
- **Parallel Group**: Wave 3 (with Task 6, 8)
- **Blocks**: Task 8
- **Blocked By**: Task 4

**Acceptance Criteria**:
- [x] Trait 定义清晰
- [x] 示例实现可编译
- [x] Mock 实现可用于测试
- [x] 文档说明如何使用自定义实现

**Commit**: YES
- Message: `feat(auth): define WalletSigner trait with examples`
- Files: `src/auth/wallet.rs`

---

### Task 8: 认证流程实现

**What to do**:
- 实现 `AuthManager` 结构体
- 实现 `prepare_signin` - 获取签名数据
- 实现 `login` - 使用签名获取 JWT
- 实现 `authenticate` - 完整认证流程（组合上述步骤）

**流程**:
```
1. 生成 Ed25519 密钥对
2. POST /v1/offchain/prepare-signin (chain, address, requestId)
3. 解析 signedData（JWT），提取 message
4. 用户钱包签名 message
5. POST /v1/offchain/login (signature, signedData)
6. 存储 JWT token
```

**API**:
```rust
pub struct AuthManager {
    client: StandxClient,
    signer: Ed25519Signer,
    jwt_manager: JwtManager,
}

impl AuthManager {
    pub async fn authenticate(&self, wallet: &dyn WalletSigner) -> Result<Credentials>;
    pub async fn prepare_signin(&self, chain: Chain, address: &str) -> Result<SigninData>;
    pub async fn login(&self, signature: &str, signed_data: &str, expires_seconds: u64) -> Result<LoginResponse>;
}
```

**Must NOT do**:
- 不要在后台自动刷新 token（v1 显式刷新）

**Recommended Agent Profile**:
- **Category**: `unspecified-high`
- **Skills**: `rust-best-practices`

**Parallelization**:
- **Can Run In Parallel**: NO (Sequential)
- **Blocked By**: Task 5, 6, 7
- **Blocks**: Task 9, 10, 11

**Acceptance Criteria**:
- [x] 完整认证流程可通过 wiremock 测试
- [x] 签名数据解析正确
- [x] 错误处理覆盖各失败点
- [x] 示例代码展示完整流程

**Commit**: YES
- Message: `feat(auth): implement complete authentication flow`
- Files: `src/auth/manager.rs`, `src/auth/types.rs`

---

### Task 9: 公共端点实现

**What to do**:
- 实现 `query_symbol_info` - 交易对信息
- 实现 `query_symbol_price` - 价格查询
- 实现 `query_depth_book` - 深度数据
- 实现 `get_kline_history` - K线历史

**特点**:
- 这些端点 **不需要认证**
- 用于测试客户端基础功能
- 验证类型定义正确

**API**:
```rust
impl StandxClient {
    pub async fn query_symbol_info(&self, symbol: &str) -> Result<Vec<SymbolInfo>>;
    pub async fn query_symbol_price(&self, symbol: &str) -> Result<SymbolPrice>;
    pub async fn query_depth_book(&self, symbol: &str) -> Result<DepthBook>;
    pub async fn get_kline_history(&self, symbol: &str, from: u64, to: u64, resolution: &str) -> Result<KlineData>;
}
```

**Recommended Agent Profile**:
- **Category**: `unspecified-high`

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 4)
- **Parallel Group**: Wave 4 (with Task 10, 11)
- **Blocks**: None (独立任务)
- **Blocked By**: Task 8

**Acceptance Criteria**:
- [x] 所有端点通过 wiremock 测试
- [x] 响应类型正确解析
- [x] 错误码正确处理

**Commit**: YES
- Message: `feat(api): implement public endpoints`
- Files: `src/http/public.rs`

---

### Task 10: 交易端点实现

**What to do**:
- 实现 `new_order` - 创建订单（需要 Body Signature）
- 实现 `cancel_order` - 取消订单
- 实现 `change_leverage` - 修改杠杆
- 实现请求体签名中间件

**特点**:
- 这些端点 **需要认证 + Body Signature**
- 实现 `RequestSigner` 处理 Ed25519 签名

**API**:
```rust
impl StandxClient {
    pub async fn new_order(&self, req: NewOrderRequest) -> Result<NewOrderResponse>;
    pub async fn cancel_order(&self, req: CancelOrderRequest) -> Result<CancelOrderResponse>;
    pub async fn change_leverage(&self, symbol: &str, leverage: u32) -> Result<ChangeLeverageResponse>;
}

pub struct RequestSigner {
    signer: Ed25519Signer,
}

impl RequestSigner {
    pub fn sign_request(&self, version: &str, request_id: &str, timestamp: u64, payload: &str) -> String;
}
```

**Recommended Agent Profile**:
- **Category**: `unspecified-high`

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 4)
- **Parallel Group**: Wave 4 (with Task 9, 11)
- **Blocks**: None
- **Blocked By**: Task 8

**Acceptance Criteria**:
- [x] 请求体签名正确（与 API 示例一致）
- [x] 所有请求头正确设置（x-request-sign-version, x-request-id, x-request-timestamp, x-request-signature）
- [x] 端点通过 wiremock 测试

**Commit**: YES
- Message: `feat(api): implement trading endpoints with body signature`
- Files: `src/http/trade.rs`, `src/http/signature.rs`

---

### Task 11: 用户端点实现

**What to do**:
- 实现 `query_orders` - 查询用户订单
- 实现 `query_open_orders` - 查询开放订单
- 实现 `query_positions` - 查询用户仓位
- 实现 `query_balance` - 查询用户余额

**特点**:
- 这些端点 **仅需 JWT 认证**
- 无 Body Signature 要求

**API**:
```rust
impl StandxClient {
    pub async fn query_orders(&self, symbol: Option<&str>, status: Option<OrderStatus>, limit: Option<u32>) -> Result<PaginatedOrders>;
    pub async fn query_open_orders(&self, symbol: Option<&str>) -> Result<PaginatedOrders>;
    pub async fn query_positions(&self, symbol: Option<&str>) -> Result<Vec<Position>>;
    pub async fn query_balance(&self) -> Result<Balance>;
}
```

**Recommended Agent Profile**:
- **Category**: `unspecified-high`

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 4)
- **Parallel Group**: Wave 4 (with Task 9, 10)
- **Blocks**: None
- **Blocked By**: Task 8

**Acceptance Criteria**:
- [x] 所有端点通过 wiremock 测试
- [x] 分页参数正确处理
- [x] 响应类型正确解析

**Commit**: YES
- Message: `feat(api): implement user endpoints`
- Files: `src/http/user.rs`

---

### Task 12: WebSocket 客户端实现

**What to do**:
- 实现 `StandxWebSocket` 结构体
- 实现连接管理（自动重连）
- 实现订阅/取消订阅方法
- 实现消息解析和分发
- 实现 Ping/Pong 处理

**API**:
```rust
pub struct StandxWebSocket {
    // ...
}

impl StandxWebSocket {
    pub async fn connect_market_stream(&self) -> Result<()>;
    pub async fn connect_order_stream(&self, token: &str) -> Result<()>;
    pub async fn subscribe_price(&self, symbol: &str) -> Result<()>;
    pub async fn subscribe_depth(&self, symbol: &str) -> Result<()>;
    pub async fn subscribe_orders(&self) -> Result<()>;
    pub fn on_message(&self) -> mpsc::Receiver<WebSocketMessage>;
}
```

**Must NOT do**:
- 不要实现状态跟踪（仅转发消息）
- 不要在 WebSocket 中处理业务逻辑

**Recommended Agent Profile**:
- **Category**: `unspecified-high`
- **Skills**: `rust-best-practices`（async 流处理）

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 4)
- **Parallel Group**: Wave 4 (with Task 9, 10, 11)
- **Blocks**: Task 13, 14, 15
- **Blocked By**: Task 8

**Acceptance Criteria**:
- [x] 可连接到 mock WebSocket server
- [x] 订阅消息格式正确
- [x] 接收消息可解析为正确类型
- [x] 自动重连机制工作（测试断开重连）
- [x] Ping/Pong 机制工作

**Commit**: YES
- Message: `feat(ws): implement WebSocket client`
- Files: `src/ws/client.rs`, `src/ws/message.rs`, `src/ws/channels.rs`

---

### Task 13: 错误处理完善

**What to do**:
- 定义完整的 `StandxError` 枚举
- 实现 `std::error::Error` trait
- 添加错误上下文（哪一步失败）
- 添加重试建议（某些错误可重试）

**API**:
```rust
#[derive(Debug, Error)]
pub enum StandxError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Authentication failed: {message}")]
    Authentication { code: u16, message: String },
    #[error("API error {code}: {message}")]
    Api { code: i32, message: String },
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Token expired")]
    TokenExpired,
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}
```

**Recommended Agent Profile**:
- **Category**: `unspecified-high`

**Parallelization**:
- **Can Run In Parallel**: NO (Sequential)
- **Blocked By**: Task 9, 10, 11, 12
- **Blocks**: Task 14, 15

**Acceptance Criteria**:
- [x] 所有错误变体有明确含义
- [x] 错误链正确（source() 返回底层错误）
- [x] 错误消息对开发者友好
- [x] 测试覆盖所有错误场景

**Commit**: YES
- Message: `feat(error): implement comprehensive error handling`
- Files: `src/error.rs`

---

### Task 14: 示例和文档

**What to do**:
- 创建 `examples/` 目录
- 编写完整认证示例
- 编写下单示例
- 编写 WebSocket 订阅示例
- 编写 README.md（快速开始指南）

**示例清单**:
- `examples/auth.rs` - 完整认证流程
- `examples/trade.rs` - 下单和撤单
- `examples/market_data.rs` - 查询市场数据
- `examples/websocket.rs` - WebSocket 订阅

**Recommended Agent Profile**:
- **Category**: `writing`
- **Skills**: `documentation`

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 5)
- **Parallel Group**: Wave 5 (with Task 13, 15)
- **Blocks**: None
- **Blocked By**: Task 12, 13

**Acceptance Criteria**:
- [x] 所有示例代码可编译（`cargo build --examples`）
- [x] README 包含快速开始指南
- [x] 所有公共 API 有 Rustdoc 文档
- [x] 文档中包含链接到 StandX API 文档

**Commit**: YES
- Message: `docs: add examples and README`
- Files: `examples/*.rs`, `README.md`

---

### Task 15: 集成测试

**What to do**:
- 编写完整的端到端测试（使用 wiremock）
- 测试完整认证流程
- 测试完整交易流程（下单 → 查询 → 撤单）
- 测试 WebSocket 连接流程
- 确保测试覆盖率 > 70%

**测试清单**:
- `tests/auth_integration.rs` - 认证流程
- `tests/trade_integration.rs` - 交易流程
- `tests/market_integration.rs` - 市场数据
- `tests/ws_integration.rs` - WebSocket

**Recommended Agent Profile**:
- **Category**: `unspecified-high`

**Parallelization**:
- **Can Run In Parallel**: YES (Wave 5)
- **Parallel Group**: Wave 5 (with Task 13, 14)
- **Blocks**: None
- **Blocked By**: Task 12, 13

**Acceptance Criteria**:
- [x] `cargo test` 全部通过
- [x] 测试覆盖率 > 70%（使用 `cargo tarpaulin` 或 `cargo llvm-cov`）
- [x] 所有主要代码路径有测试
- [x] Mock 测试不依赖外部 API

**Commit**: YES
- Message: `test: add comprehensive integration tests`
- Files: `tests/*.rs`

---

## Commit Strategy

| After Task | Message | Files | Verification |
|------------|---------|-------|--------------|
| 0 | `chore(adapter): setup test infrastructure` | `Cargo.toml`, `tests/` | `cargo test` passes |
| 1 | `chore(adapter): setup crate structure and dependencies` | `Cargo.toml`, `src/` | `cargo check` passes |
| 2 | `feat(types): define core types and enums` | `src/types/` | `cargo test types` passes |
| 3 | `feat(auth): implement Ed25519 signer` | `src/auth/signer.rs` | `cargo test signer` passes |
| 4 | `feat(http): implement base client structure` | `src/http/client.rs` | `cargo check` passes |
| 5 | `feat(http): implement request/response middleware` | `src/http/middleware.rs` | `cargo test http` passes |
| 6 | `feat(auth): implement JWT manager` | `src/auth/jwt.rs` | `cargo test jwt` passes |
| 7 | `feat(auth): define WalletSigner trait with examples` | `src/auth/wallet.rs` | `cargo check` passes |
| 8 | `feat(auth): implement complete authentication flow` | `src/auth/manager.rs` | `cargo test auth` passes |
| 9 | `feat(api): implement public endpoints` | `src/http/public.rs` | `cargo test public` passes |
| 10 | `feat(api): implement trading endpoints with body signature` | `src/http/trade.rs`, `src/http/signature.rs` | `cargo test trade` passes |
| 11 | `feat(api): implement user endpoints` | `src/http/user.rs` | `cargo test user` passes |
| 12 | `feat(ws): implement WebSocket client` | `src/ws/` | `cargo test ws` passes |
| 13 | `feat(error): implement comprehensive error handling` | `src/error.rs` | `cargo test error` passes |
| 14 | `docs: add examples and README` | `examples/`, `README.md` | `cargo build --examples` passes |
| 15 | `test: add comprehensive integration tests` | `tests/` | `cargo test` passes, coverage > 70% |

---

## Success Criteria

### Verification Commands
```bash
# 编译检查
cargo check

# 运行所有测试
cargo test

# 构建示例
cargo build --examples

# 检查文档
cargo doc --no-deps

# 检查格式化
cargo fmt --check

# 运行 clippy
cargo clippy -- -D warnings
```

### Final Checklist
- [x] 所有 "Must Have" 端点实现
- [x] 所有 "Must NOT Have" 未实现
- [x] 所有测试通过
- [x] 文档完整（Rustdoc + README + 示例）
- [x] 无编译警告（clippy clean）
- [x] 代码格式化通过
- [x] 测试覆盖率 > 70%

### External References
- [StandX API 文档](https://docs.standx.com/standx-api/standx-api)
- [StandX HTTP API](https://docs.standx.com/standx-api/perps-http)
- [StandX WebSocket API](https://docs.standx.com/standx-api/perps-ws)
- [StandX 认证指南](https://docs.standx.com/standx-api/perps-auth)
