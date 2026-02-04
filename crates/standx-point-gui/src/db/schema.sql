-- StandX Point GUI - Database Schema
-- [INPUT]:  Application data models (Accounts, Tasks, Orders, Trades)
-- [OUTPUT]: SQLite schema with tables, indexes, and constraints
-- [POS]:    Data layer - persistence schema
-- [UPDATE]: When data models change or new tracking requirements emerge

-- Account credentials and metadata
CREATE TABLE IF NOT EXISTS accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    address TEXT NOT NULL UNIQUE,          -- Wallet address (e.g., 0x...)
    alias TEXT,                           -- User-defined name for the account
    encrypted_jwt TEXT,                   -- Base64 encoded encrypted JWT
    encrypted_signing_key TEXT,           -- Base64 encoded encrypted Ed25519 private key
    chain TEXT NOT NULL,                  -- Chain identifier (e.g., Bsc, Solana)
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Trading tasks (market making instances)
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL,
    name TEXT NOT NULL,                   -- Task display name
    symbol TEXT NOT NULL,                 -- Trading symbol (e.g., BTC-USD)
    config_json TEXT NOT NULL,            -- JSON serialized TaskConfig (risk, sizing, etc.)
    status TEXT NOT NULL,                 -- TaskStatus: Draft, Pending, Running, Paused, Stopped, Failed
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

-- Historical orders placed by tasks
CREATE TABLE IF NOT EXISTS order_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    order_id INTEGER NOT NULL,            -- Remote order ID from exchange
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,                   -- Buy, Sell
    price TEXT,                           -- Decimal stored as string
    qty TEXT NOT NULL,                    -- Decimal stored as string
    status TEXT NOT NULL,                 -- OrderStatus: Created, Open, Filled, Canceled, etc.
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Historical trades (fills) associated with orders
CREATE TABLE IF NOT EXISTS trade_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    trade_id INTEGER NOT NULL,            -- Remote trade ID from exchange
    order_id INTEGER NOT NULL,            -- Associated exchange order ID
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,
    price TEXT NOT NULL,                  -- Decimal stored as string
    qty TEXT NOT NULL,                    -- Decimal stored as string
    fee TEXT NOT NULL,                    -- Fee amount (Decimal string)
    pnl TEXT,                             -- Realized PnL (Decimal string)
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Logs for task operations (startup, shutdown, errors, risk triggers)
CREATE TABLE IF NOT EXISTS operation_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    action TEXT NOT NULL,                 -- e.g., START, STOP, RISK_HALT, ERROR
    details TEXT,                         -- Detailed message or JSON payload
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_accounts_address ON accounts(address);
CREATE INDEX IF NOT EXISTS idx_tasks_account_id ON tasks(account_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_order_history_task_id ON order_history(task_id);
CREATE INDEX IF NOT EXISTS idx_order_history_created_at ON order_history(created_at);
CREATE INDEX IF NOT EXISTS idx_trade_history_task_id ON trade_history(task_id);
CREATE INDEX IF NOT EXISTS idx_trade_history_created_at ON trade_history(created_at);
CREATE INDEX IF NOT EXISTS idx_operation_logs_task_id ON operation_logs(task_id);
CREATE INDEX IF NOT EXISTS idx_operation_logs_created_at ON operation_logs(created_at);
