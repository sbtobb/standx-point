# 优化 StandX 认证流程与添加交互式 CLI 配置

## TL;DR

**目标**：
1. 简化 `standx-point-adapter` 认证流程：仅需钱包私钥 + 链类型，ed25519 临时密钥自动持久化存储到 `.standx-keys/` 目录
2. 为 `standx-point-mm-strategy` 添加交互式 CLI 配置生成器（`init` 命令）

**关键交付物**：
- `PersistentKeyManager`: 自动管理 ed25519 密钥的读取/生成/存储
- `EvmWalletSigner` / `SolanaWalletSigner`: 钱包签名实现
- 简化的 `AuthManager` API: `authenticate_with_wallet(private_key, chain)`
- 交互式 `init` CLI: 引导用户创建配置文件，支持多任务配置

**预估工作量**: Medium (~2-3 小时)
**并行执行**: 两部分可独立开发（认证优化 vs CLI），但最后需整合测试

---

## Context

### 当前状态分析

**1. standx-point-adapter 认证层**
- `AuthManager` 每次运行生成新的 Ed25519 keypair (`signer: Ed25519Signer::generate()`)
- 用户需要手动管理 JWT token 和 signing_key
- 有 `WalletSigner` trait 但无具体实现（只有 Mock）
- `AuthManager::authenticate` 流程已设计但未完全实现 HTTP 调用

**参考代码**: `crates/standx-point-adapter/src/auth/manager.rs:44-47`
```rust
pub fn new(client: StandxClient) -> Self {
    Self {
        client,
        signer: Ed25519Signer::generate(),  // ❌ 每次都生成新密钥
        jwt_manager: JwtManager::new(),
    }
}
```

**2. standx-point-mm-strategy CLI**
- 当前只有 `--config`, `--log-level`, `--dry-run` 三个参数
- 配置文件需要手动编写 YAML
- 无配置验证或引导功能

**参考代码**: `crates/standx-point-mm-strategy/src/main.rs:19-28`

### StandX Perps Auth 文档要点

根据 [perps-auth](https://docs.standx.com/standx-api/perps-auth) 文档：
1. 准备钱包和临时 ed25519 key pair
2. 获取 signature data (`prepare-signin`)
3. 用钱包私钥签名 message
4. 登录获取 JWT token (`login`)
5. 使用 access token + ed25519 签名请求体

**安全提示**：文档强调 "Keep secure; use environment variables"

---

## Work Objectives

### Core Objective
创建用户友好的认证和配置体验：
- 认证：用户只需提供钱包私钥，库自动处理 ed25519 密钥和 JWT
- 配置：交互式 CLI 引导用户创建配置文件

### Concrete Deliverables
- [x] `PersistentKeyManager` 模块（文件存储/读取/生成 ed25519 密钥）
- [x] `EvmWalletSigner` 实现（ethers-rs 或 alloy 钱包签名）
- [x] `SolanaWalletSigner` 实现（solana-sdk 钱包签名）
- [x] 简化的 `AuthManager::authenticate_with_wallet()` API
- [x] CLI `init` 子命令（交互式配置生成）
- [x] 配置验证和示例配置文件更新

### Definition of Done
- [x] 运行示例代码：仅需钱包私钥即可完成认证并获取 JWT
- [x] 检查 `.standx-keys/` 目录：ed25519 密钥文件自动创建并复用
- [x] 运行 `init` 命令：完整交互流程生成有效配置文件
- [x] 配置文件能通过 `--dry-run` 验证

### Must Have
- 密钥文件存储在运行时目录下的 `.standx-keys/`（默认位置）
- 密钥文件命名：`{wallet_address}_ed25519.key`（支持多账户）
- 支持 BSC (EVM) 和 Solana 两种链类型
- 密钥文件格式：Base64 编码的 32 字节私钥（兼容现有解码逻辑）
- 文件权限：设置为 0o600（仅所有者可读写）
- 交互式 CLI 必须收集：任务 ID、symbol、钱包私钥、链类型
- 生成符合现有 Schema 的 YAML 配置文件

### Must NOT Have (Guardrails)
- ❌ 不要支持明文私钥直接存储在配置文件（避免误提交到版本库）
- ❌ 不要添加不必要的依赖（保持轻量）
- ❌ 不要修改现有的 StrategyConfig 结构（向后兼容）
- ❌ 不要在 AuthManager 中硬编码 HTTP 客户端逻辑（保持分层）

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES - 已有测试框架 (`cargo test`)
- **Automated tests**: Tests-after - 实现完成后添加测试
- **Framework**: 内置 `cargo test`

### Agent-Executed QA Scenarios

#### Scenario 1: Persistent Key Storage (Bash)
```yaml
Tool: Bash
Preconditions: standx-point-adapter 已实现 PersistentKeyManager
Steps:
  1. 删除现有密钥目录: rm -rf ./.standx-keys/
  2. 运行测试示例: cargo run --example auth_with_persistent_key -- --chain bsc --key <test-key>
  3. 检查密钥文件存在: ls -la ./.standx-keys/
  4. 验证文件内容格式: cat ./.standx-keys/ed25519_key | base64 -d | wc -c  # 应输出 32
  5. 再次运行示例: cargo run --example auth_with_persistent_key -- --chain bsc --key <test-key>
  6. 验证使用相同密钥（检查日志中的 request_id 是否相同）
Expected Result:
  - 首次运行创建密钥文件
  - 密钥文件为 32 字节 base64 编码
  - 后续运行复用相同密钥（request_id 一致）
Evidence: 终端输出日志
```

#### Scenario 2: EVM Wallet Signer (Bash)
```yaml
Tool: Bash
Preconditions: EvmWalletSigner 已实现
Steps:
  1. 使用测试私钥创建签名器: 
     cargo test evm_wallet_signer -- --nocapture
  2. 验证签名格式: 
     - 检查签名以 0x 开头
     - 检查签名长度为 132 字符 (0x + 65 bytes hex)
Expected Result:
  - 成功创建 EvmWalletSigner
  - 签名格式符合 EVM 标准 (0x + r + s + v)
Evidence: 测试输出
```

#### Scenario 3: Solana Wallet Signer (Bash)
```yaml
Tool: Bash
Preconditions: SolanaWalletSigner 已实现
Steps:
  1. 使用测试私钥创建签名器:
     cargo test solana_wallet_signer -- --nocapture
  2. 验证公钥地址与预期一致
  3. 验证签名格式为 base64
Expected Result:
  - 成功创建 SolanaWalletSigner
  - 公钥地址从私钥正确派生
  - 签名格式符合 Solana 标准
Evidence: 测试输出
```

#### Scenario 4: Interactive Config Generator (Bash + Expect)
```yaml
Tool: Bash
Preconditions: standx-point-mm-strategy 已添加 init 命令
Steps:
  1. 运行交互式配置生成: cargo run -- init --output ./test-config.yaml
  2. 模拟用户输入:
     - 任务数量: 1
     - 任务 ID: test-btc-mm
     - Symbol: BTC-USD
     - 链类型: bsc
     - 私钥提示: 确认收到安全警告
     - JWT Token: test-jwt-token
     - Signing Key: test-signing-key-base64
     - Risk Level: conservative
     - Max Position: 50000
  3. 验证输出文件存在: ls -la ./test-config.yaml
  4. 验证文件格式: cat ./test-config.yaml | grep -E "(id:|symbol:|jwt_token|signing_key)"
  5. 验证配置文件: cargo run -- --config ./test-config.yaml --dry-run
Expected Result:
  - 配置文件成功创建
  - 格式符合 StrategyConfig schema
  - --dry-run 验证通过
Evidence: 配置文件内容 + dry-run 输出
```

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (独立开发 - 可并行):
├── Task 1: PersistentKeyManager 实现 (standx-point-adapter)
└── Task 2: 交互式 CLI init 命令 (standx-point-mm-strategy)

Wave 2 (独立开发 - 可并行):
├── Task 3: EvmWalletSigner 实现 (standx-point-adapter)
└── Task 4: SolanaWalletSigner 实现 (standx-point-adapter)

Wave 3 (依赖 Wave 1-2):
└── Task 5: 简化 AuthManager API 并集成持久化密钥

Wave 4 (依赖 Wave 1-3):
└── Task 6: 整合测试与示例代码

Wave 5 (文档与完成):
└── Task 7: 更新文档和配置示例
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 1 | None | 5 | 2, 3, 4 |
| 2 | None | None | 1, 3, 4 |
| 3 | None | 5 | 1, 2, 4 |
| 4 | None | 5 | 1, 2, 3 |
| 5 | 1, 3, 4 | 6 | None |
| 6 | 5 | 7 | None |
| 7 | 6 | None | None |

### Critical Path
Task 1/3/4 → Task 5 → Task 6 → Task 7 (约 70% 的总时间)

---

## TODOs

### Task 1: PersistentKeyManager 实现

**What to do**:
- [x] 在 `crates/standx-point-adapter/src/auth/` 创建 `persistent_key.rs`
- [x] 实现 `PersistentKeyManager` struct，包含：
  - `key_dir: PathBuf` - 密钥存储目录
- [x] 实现核心方法：
  - `new(key_dir: impl AsRef<Path>) -> Self` - 构造函数
  - `get_or_create_signer(wallet_address: &str) -> Result<Ed25519Signer>` - 读取或生成指定账户的密钥
  - `load_signer(wallet_address: &str) -> Option<Ed25519Signer>` - 尝试加载指定账户的现有密钥
  - `save_signer(wallet_address: &str, &Ed25519Signer) -> Result<()>` - 保存指定账户的密钥到文件
  - `list_stored_accounts() -> Vec<String>` - 列出所有已存储的账户地址
  - `key_file_path(wallet_address: &str) -> PathBuf` - 获取指定账户的密钥文件路径
- [x] 密钥文件命名格式：`{wallet_address}_ed25519.key`（例如：`0x1234..._ed25519.key`）
- [x] 密钥文件格式：使用 base64 编码的 32 字节私钥
- [x] 文件权限：设置为 0o600（仅所有者可读写）

**Must NOT do**:
- ❌ 不要添加加密（用户负责目录安全）
- ❌ 不要修改 Ed25519Signer 结构（保持向后兼容）

**Recommended Agent Profile**:
- **Category**: `quick` - 这是一个模块实现任务，逻辑清晰
- **Skills**: [`rust-best-practices`]
  - `rust-best-practices`: 需要正确的文件权限处理和错误处理模式

**Parallelization**:
- **Can Run In Parallel**: YES
- **Parallel Group**: Wave 1
- **Blocks**: Task 5
- **Blocked By**: None

**References**:
- `crates/standx-point-adapter/src/auth/signer.rs:20-29` - Ed25519Signer API 参考
- `crates/standx-point-adapter/src/auth/manager.rs:44-47` - 当前密钥生成方式（需替换）
- 解码逻辑参考: `crates/standx-point-mm-strategy/src/task.rs:495-516` - Base64 解码逻辑（32/64 字节支持）

**Acceptance Criteria**:
- [x] `PersistentKeyManager::get_or_create_signer("0x1234...")` 首次调用创建新密钥
- [x] 密钥文件保存到 `./.standx-keys/0x1234..._ed25519.key`（使用账户地址命名）
- [x] 文件权限为 0o600
- [x] 第二次调用复用相同密钥（验证公钥相同）
- [x] 不同账户地址生成独立的密钥文件
- [x] `list_stored_accounts()` 返回所有已存储的账户地址列表
- [x] 错误处理：目录创建失败、文件写入失败等给出清晰错误信息

**Agent-Executed QA Scenarios**:

**Scenario 1: 多账户密钥创建**
```yaml
Tool: Bash
Steps:
  1. rm -rf ./.standx-keys/
  2. cargo test persistent_key_multi_account -- --nocapture
  3. ls -la ./.standx-keys/
Expected: 
  - 两个不同的密钥文件: `0xaaa..._ed25519.key` 和 `0xbbb..._ed25519.key`
  - 两个文件权限均为 600
  - 不同账户的公钥不同
```

**Scenario 2: 复用现有密钥**
```yaml
Tool: Bash
Steps:
  1. cargo test persistent_key_reuse -- --nocapture 2>&1 | tee /tmp/test1.log
  2. cargo test persistent_key_reuse -- --nocapture 2>&1 | tee /tmp/test2.log
  3. diff /tmp/test1.log /tmp/test2.log  # 应显示相同公钥
Expected: 两次运行公钥相同
```

**Scenario 3: 列出存储的账户**
```yaml
Tool: Bash
Steps:
  1. cargo test persistent_key_list_accounts -- --nocapture
Expected: 返回所有已存储的账户地址列表（不含 `_ed25519.key` 后缀）
```

**Commit**: YES
- Message: `feat(adapter): add PersistentKeyManager for ed25519 key storage`
- Files: `crates/standx-point-adapter/src/auth/persistent_key.rs`, `mod.rs`

---

### Task 2: 交互式 CLI init 命令

**What to do**:
- [x] 添加依赖到 `crates/standx-point-mm-strategy/Cargo.toml`:
  - `dialoguer = "0.11"` (交互式提示)
  - `console = "0.15"` (终端样式)
- [x] 在 `src/` 创建 `cli/` 目录结构：
  - `src/cli/mod.rs` - 模块入口
  - `src/cli/init.rs` - init 命令实现
- [x] 实现交互式配置收集：
  - 任务数量（1-5 个）
  - 对每个任务收集：
    - 任务 ID（验证唯一性）
    - Symbol（BTC-USD, ETH-USD 等）
    - 链类型（BSC / Solana）
    - JWT Token（输入时显示警告）
    - Signing Key（Base64 编码的 ed25519 私钥）
    - 风险级别（conservative/moderate/aggressive）
    - 最大仓位（USD）
    - 价格跳变阈值（bps）
    - 基础订单量
    - 订单层数（1-3）
- [x] 实时配置验证：
  - 验证 symbol 格式（XXX-YYY）
  - 验证 signing_key 是有效 base64
  - 验证 JWT token 格式（可选）
- [x] 生成 YAML 配置文件（使用 serde_yaml）
- [x] 在 main.rs 添加 `init` 子命令路由

**Must NOT do**:
- ❌ 不要存储钱包私钥到配置文件（只存 JWT 和 signing_key）
- ❌ 不要添加过于复杂的验证（保持流程顺畅）

**Recommended Agent Profile**:
- **Category**: `visual-engineering` - CLI 交互体验很重要
- **Skills**: [`ui-ux-pro-max`, `git-commit`]
  - `ui-ux-pro-max`: 设计清晰的交互流程和错误提示
  - `git-commit`: 可能需要添加新文件和修改 Cargo.toml

**Parallelization**:
- **Can Run In Parallel**: YES
- **Parallel Group**: Wave 1
- **Blocks**: None
- **Blocked By**: None

**References**:
- 现有 CLI 结构: `crates/standx-point-mm-strategy/src/main.rs:19-28`
- 配置结构: `crates/standx-point-mm-strategy/src/config.rs:17-30`
- 示例配置: `crates/standx-point-mm-strategy/examples/config.yaml`
- `dialoguer` 文档: https://docs.rs/dialoguer/latest/dialoguer/

**Acceptance Criteria**:
- [x] 运行 `cargo run -- init --output test.yaml` 启动交互流程
- [x] 所有配置项都有默认值或提示
- [x] 错误输入给出清晰反馈（如 symbol 格式错误）
- [x] 生成的 YAML 能通过 `StrategyConfig::from_file()` 解析
- [x] 配置文件包含 Fractal Context header 注释

**Agent-Executed QA Scenarios**:

**Scenario 1: 完整交互流程**
```yaml
Tool: Bash
Steps:
  1. rm -f ./test-config.yaml
  2. cargo run -- init --output ./test-config.yaml << 'EOF'
1
test-task
BTC-USD
bsc
test-jwt-token-12345
dGVzdC1zaWduaW5nLWtleS0xMjM0NTY3OA==
conservative
50000
5
0.1
2
EOF
  3. ls -la ./test-config.yaml
  4. cargo run -- --config ./test-config.yaml --dry-run
Expected: 配置文件创建成功，dry-run 验证通过
```

**Scenario 2: 输入验证**
```yaml
Tool: Bash
Steps:
  1. cargo test cli_input_validation -- --nocapture
Expected: 无效输入给出清晰错误提示
```

**Commit**: YES
- Message: `feat(mm-strategy): add interactive init command for config generation`
- Files: `src/cli/`, `Cargo.toml`, `main.rs`

---

### Task 3: EvmWalletSigner 实现

**What to do**:
- [x] 在 `crates/standx-point-adapter/src/auth/` 添加 `evm_wallet.rs`
- [x] 添加依赖到 Cargo.toml: `alloy = { version = "0.3", features = ["signers"] }`
- [x] 实现 `EvmWalletSigner` struct：
  - 字段: `signing_key: alloy::signers::local::LocalSigner<alloy::primitives::Signature>`
  - 字段: `chain: Chain`
- [x] 实现 `WalletSigner` trait：
  - `chain() -> Chain` - 返回 Chain::Bsc
  - `address() -> &str` - 返回 hex 编码地址（0x...）
  - `sign_message(&self, message: &str) -> Result<String>` - 签名并返回 0x... 格式
- [x] 构造函数：`new(private_key_hex: &str) -> Result<Self>`
  - 支持带或不带 0x 前缀的 hex 字符串
  - 支持 64 字符（32 字节）私钥

**Must NOT do**:
- ❌ 不要添加额外的网络依赖（只用本地签名）
- ❌ 不要暴露私钥调试信息

**Recommended Agent Profile**:
- **Category**: `quick`
- **Skills**: []
- 理由: 标准钱包实现，alloy 文档清晰

**Parallelization**:
- **Can Run In Parallel**: YES
- **Parallel Group**: Wave 2
- **Blocks**: Task 5
- **Blocked By**: None

**References**:
- 文档: https://docs.standx.com/standx-api/perps-auth-evm-example
- WalletSigner trait: `crates/standx-point-adapter/src/auth/wallet.rs:17-30`
- Alloy signers: https://docs.rs/alloy-signer/latest/alloy_signer/

**Acceptance Criteria**:
- [x] `EvmWalletSigner::new("0x...")` 成功创建
- [x] `address()` 返回正确的 EVM 地址
- [x] `sign_message()` 返回 0x 开头的 132 字符签名
- [x] 签名可通过 ethers verify 验证（单元测试）

**Agent-Executed QA Scenarios**:

**Scenario 1: 钱包创建与签名**
```yaml
Tool: Bash
Steps:
  1. cargo test evm_wallet_sign_flow -- --nocapture
Expected: 签名成功，格式正确
```

**Commit**: YES
- Message: `feat(adapter): add EvmWalletSigner for BSC authentication`
- Files: `src/auth/evm_wallet.rs`, `Cargo.toml`, `auth/mod.rs`

---

### Task 4: SolanaWalletSigner 实现

**What to do**:
- [x] 在 `crates/standx-point-adapter/src/auth/` 添加 `solana_wallet.rs`
- [x] 添加依赖到 Cargo.toml: `solana-sdk = "2.0"`, `bs58 = "0.5"`
- [x] 实现 `SolanaWalletSigner` struct：
  - 字段: `keypair: solana_sdk::signature::Keypair`
  - 字段: `chain: Chain`
- [x] 实现 `WalletSigner` trait：
  - `chain() -> Chain` - 返回 Chain::Solana
  - `address() -> &str` - 返回 base58 编码公钥
  - `sign_message()` - 使用 ed25519 签名，返回特定 JSON 格式（见文档）
- [x] 构造函数：`new(private_key_base58: &str) -> Result<Self>`
  - 从 base58 解码 64 字节密钥对
  - 或支持 32 字节私钥种子

**重点**：Solana 签名格式特殊（见 [perps-auth-svm-example](https://docs.standx.com/standx-api/perps-auth-svm-example)）
```rust
// 需要返回 base64 编码的 JSON:
{
  "input": payload,
  "output": {
    "signedMessage": [...],
    "signature": [...],
    "account": {
      "publicKey": [...]
    }
  }
}
```

**Must NOT do**:
- ❌ 不要简化签名格式（必须遵循文档要求）
- ❌ 不要混淆签名密钥和 ed25519 请求签名密钥

**Recommended Agent Profile**:
- **Category**: `quick`
- **Skills**: []
- 理由: Solana SDK 有标准做法

**Parallelization**:
- **Can Run In Parallel**: YES
- **Parallel Group**: Wave 2
- **Blocks**: Task 5
- **Blocked By**: None

**References**:
- 文档: https://docs.standx.com/standx-api/perps-auth-svm-example
- Solana SDK: https://docs.rs/solana-sdk/latest/solana_sdk/

**Acceptance Criteria**:
- [x] `SolanaWalletSigner::new("base58...")` 成功创建
- [x] `address()` 返回正确的 base58 公钥
- [x] `sign_message()` 返回文档要求的 JSON 格式
- [x] 公钥可从私钥正确派生（验证一致性）

**Agent-Executed QA Scenarios**:

**Scenario 1: 钱包创建与签名格式**
```yaml
Tool: Bash
Steps:
  1. cargo test solana_wallet_sign_format -- --nocapture
Expected: 签名格式符合文档要求
```

**Commit**: YES
- Message: `feat(adapter): add SolanaWalletSigner for SVM authentication`
- Files: `src/auth/solana_wallet.rs`, `Cargo.toml`, `auth/mod.rs`

---

### Task 5: 简化 AuthManager API 并集成持久化密钥

**What to do**:
- [x] 修改 `AuthManager` 构造函数：
  - 移除 `Ed25519Signer::generate()`
  - 添加 `PersistentKeyManager` 字段，用于多账户密钥管理
- [x] 添加新方法 `authenticate_with_wallet(wallet_address: &str, private_key: &str, chain: Chain) -> Result<LoginResponse>`
  - 根据 chain 类型自动创建对应 WalletSigner（从 private_key 推导 wallet_address 并验证匹配）
  - 使用 `PersistentKeyManager::get_or_create_signer(wallet_address)` 获取/创建该账户的 ed25519 密钥
  - 执行完整认证流程：prepare-signin → parse → sign → login
  - 密钥自动按 `wallet_address` 命名存储（`{wallet_address}_ed25519.key`）
- [x] 添加 `list_stored_accounts() -> Vec<String>` 方法，列出所有已存储密钥的账户地址
- [x] 实现 HTTP 调用（当前是 todo!）：
  - `prepare_signin`: POST /v1/offchain/prepare-signin?chain={chain}
  - `login`: POST /v1/offchain/login?chain={chain}
- [x] 添加 JWT 解析：从 `signed_data` JWT 提取 `message` 字段
- [x] 添加错误处理：网络错误、认证失败、JWT 过期、地址不匹配等

**API 设计目标**：
```rust
// 简化后的使用方式（支持多账户）
let mut auth = AuthManager::new(client, "./.standx-keys").await?;

// 账户 A 认证
let login_a = auth.authenticate_with_wallet(
    "0xAb5801a7C39835123f73D5c4B7b1a...",  // wallet_address
    "0xprivate_key_hex_here",               // private_key
    Chain::Bsc
).await?;

// 账户 B 认证（使用独立的 ed25519 密钥）
let login_b = auth.authenticate_with_wallet(
    "0xdD870fA1b7C4700F2BD7f44238821C26f7392148",
    "0xanother_private_key_here",
    Chain::Bsc
).await?;

// 查看已存储的账户
let accounts = auth.list_stored_accounts();
println!("Stored accounts: {:?}", accounts);
```

**Must NOT do**:
- ❌ 不要破坏现有 API（保持 `new()` 兼容或标记 deprecated）
- ❌ 不要在 AuthManager 里硬编码文件路径（通过参数传入）

**Recommended Agent Profile**:
- **Category**: `ultrabrain` - 涉及 HTTP 实现和流程整合
- **Skills**: [`rust-best-practices`]
  - 需要正确处理 async HTTP 和错误传播

**Parallelization**:
- **Can Run In Parallel**: NO
- **Parallel Group**: Wave 3
- **Blocks**: Task 6
- **Blocked By**: Task 1, 3, 4

**References**:
- 当前 AuthManager: `crates/standx-point-adapter/src/auth/manager.rs:35-54`
- HTTP 客户端: `crates/standx-point-adapter/src/http/client.rs`
- 登录流程文档: https://docs.standx.com/standx-api/perps-auth#5-get-access-token
- JWT 解析: https://docs.rs/jsonwebtoken/latest/jsonwebtoken/

**Acceptance Criteria**:
- [x] `AuthManager::authenticate_with_wallet(wallet_address, private_key, chain)` 完成完整认证流程
- [x] ed25519 密钥按账户地址命名存储（`{wallet_address}_ed25519.key`）
- [x] 不同账户使用独立的 ed25519 密钥
- [x] 从 private_key 推导的地址与传入的 wallet_address 匹配（验证一致性）
- [x] JWT 正确存储到 JwtManager（支持多账户 JWT）
- [x] `list_stored_accounts()` 返回所有已认证账户列表
- [x] 错误场景给出清晰错误信息（地址不匹配、网络错误等）

**Agent-Executed QA Scenarios**:

**Scenario 1: 多账户认证流程（使用 Mock HTTP）**
```yaml
Tool: Bash
Steps:
  1. rm -rf ./.standx-keys/
  2. cargo test auth_manager_multi_account -- --nocapture
  3. ls -la ./.standx-keys/
Expected: 
  - 两个账户各自的密钥文件创建
  - 两个账户的 JWT 都正确存储
  - list_stored_accounts() 返回两个地址
```

**Scenario 2: 地址验证**
```yaml
Tool: Bash
Steps:
  1. cargo test auth_manager_address_mismatch -- --nocapture
Expected: 
  - 传入错误的 wallet_address 时返回错误
  - 错误信息包含 "address mismatch" 或类似提示
```

**Commit**: YES
- Message: `feat(adapter): simplify AuthManager with persistent keys and wallet auth`
- Files: `src/auth/manager.rs`, `src/auth/mod.rs`

---

### Task 6: 整合测试与示例代码

**What to do**:
- [x] 创建示例 `auth_with_persistent_key.rs`：
  - 演示完整认证流程
  - 显示密钥存储和复用
- [x] 更新 `auth_example.rs`：
  - 使用新的简化 API
  - 添加 EVM 和 Solana 两种示例
- [x] 添加集成测试：
  - 测试 PersistentKeyManager 文件操作
  - 测试 WalletSigner 签名一致性
  - 测试完整认证流程（使用 wiremock 模拟 HTTP）
- [x] 验证 CLI init 生成的配置能通过认证

**Must NOT do**:
- ❌ 不要使用真实私钥（用测试密钥或环境变量）
- ❌ 不要修改现有的交易/市场数据示例（避免破坏）

**Recommended Agent Profile**:
- **Category**: `unspecified-high` - 需要编写测试和示例
- **Skills**: []

**Parallelization**:
- **Can Run In Parallel**: NO
- **Parallel Group**: Wave 4
- **Blocks**: Task 7
- **Blocked By**: Task 5

**Acceptance Criteria**:
- [x] 运行 `cargo test --package standx-point-adapter` 全部通过
- [x] 运行示例代码展示完整流程
- [x] 测试覆盖率 > 70%

**Agent-Executed QA Scenarios**:

**Scenario 1: 示例代码运行**
```yaml
Tool: Bash
Steps:
  1. cargo run --example auth_with_persistent_key 2>&1 | tee /tmp/example.log
  2. grep -E "(JWT|key|request)" /tmp/example.log
Expected: 输出显示成功获取 JWT 和密钥存储
```

**Commit**: YES
- Message: `test(adapter): add integration tests and updated examples`
- Files: `examples/`, `tests/`

---

### Task 7: 更新文档和配置示例

**What to do**:
- [x] 更新 `crates/standx-point-adapter/README.md`：
  - 添加新认证流程说明
  - 添加 EvmWalletSigner 和 SolanaWalletSigner 使用示例
  - 添加 PersistentKeyManager 说明
- [x] 更新 `crates/standx-point-mm-strategy/README.md`：
  - 添加 `init` 命令使用说明
  - 更新配置示例
- [x] 更新配置文件示例：
  - `examples/config.yaml` - 添加新格式说明
  - `examples/single_task.yaml` - 更新注释
- [x] 更新 `AGENTS.md`：
  - 添加新模块说明
  - 更新架构图（如需要）

**Must NOT do**:
- ❌ 不要删除旧文档（标记为 deprecated）
- ❌ 不要遗漏安全提示

**Recommended Agent Profile**:
- **Category**: `writing` - 文档编写
- **Skills**: []

**Parallelization**:
- **Can Run In Parallel**: NO
- **Parallel Group**: Wave 5
- **Blocks**: None
- **Blocked By**: Task 6

**Acceptance Criteria**:
- [x] README 包含简化认证流程说明
- [x] 配置示例包含完整注释
- [x] AGENTS.md 反映新架构

**Agent-Executed QA Scenarios**:

**Scenario 1: 文档验证**
```yaml
Tool: Bash
Steps:
  1. cargo doc --package standx-point-adapter --no-deps
  2. cargo doc --package standx-point-mm-strategy --no-deps
  3. 检查是否有警告
Expected: 无文档警告
```

**Commit**: YES
- Message: `docs: update README and examples for new auth flow and CLI`
- Files: `README.md`, `examples/`, `AGENTS.md`

---

## Commit Strategy

| After Task | Message | Files | Verification |
|------------|---------|-------|--------------|
| 1 | `feat(adapter): add PersistentKeyManager` | `src/auth/persistent_key.rs`, `mod.rs`, `Cargo.toml` | `cargo test persistent_key` |
| 2 | `feat(mm-strategy): add interactive init command` | `src/cli/`, `main.rs`, `Cargo.toml` | `cargo run -- init --help` |
| 3 | `feat(adapter): add EvmWalletSigner` | `src/auth/evm_wallet.rs`, `mod.rs`, `Cargo.toml` | `cargo test evm_wallet` |
| 4 | `feat(adapter): add SolanaWalletSigner` | `src/auth/solana_wallet.rs`, `mod.rs`, `Cargo.toml` | `cargo test solana_wallet` |
| 5 | `feat(adapter): simplify AuthManager API` | `src/auth/manager.rs`, `mod.rs` | `cargo test auth_manager` |
| 6 | `test(adapter): add integration tests` | `examples/`, `tests/` | `cargo test --package standx-point-adapter` |
| 7 | `docs: update README and examples` | `README.md`, `examples/`, `AGENTS.md` | `cargo doc --no-deps` |

---

## Success Criteria

### Verification Commands
```bash
# 1. 密钥持久化测试
rm -rf ./.standx-keys/
cargo test --package standx-point-adapter persistent_key -- --nocapture
ls -la ./.standx-keys/ed25519_key

# 2. 钱包签名测试
cargo test --package standx-point-adapter evm_wallet -- --nocapture
cargo test --package standx-point-adapter solana_wallet -- --nocapture

# 3. 完整认证流程测试（使用 Mock）
cargo test --package standx-point-adapter auth_manager -- --nocapture

# 4. CLI init 测试
cargo run --package standx-point-mm-strategy -- init --output /tmp/test.yaml
# 输入测试数据...
cargo run --package standx-point-mm-strategy -- --config /tmp/test.yaml --dry-run

# 5. 示例代码
cargo run --example auth_with_persistent_key

# 6. 文档检查
cargo doc --no-deps 2>&1 | grep -i warning || echo "No warnings"
```

### Final Checklist
- [x] 所有单元测试通过 (`cargo test`)
- [x] 示例代码可运行
- [x] 配置文件格式向后兼容
- [x] 文档更新完整
- [x] 安全提示已添加（不要在配置中存储私钥）
- [x] Fractal Context headers 已添加到新文件
