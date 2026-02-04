/*
[INPUT]:  schema.sql, state Account/Task types, adapter enums, system-derived key seed.
[OUTPUT]: SQLite-backed Database pool with CRUD operations and encrypted credentials.
[POS]:    Persistence layer - GUI local storage for accounts, tasks, and history.
[UPDATE]: When schema.sql or state model shapes change.
*/

use crate::state::{Account, Order, OrderStatus, Side, Task, Trade};
use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use anyhow::{anyhow, bail, Context, Result};
use rand::rngs::OsRng;
use rand::RngCore;
use rusqlite::{params, OptionalExtension};
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::hash::Hasher;
use std::path::Path;
use std::str::FromStr;

pub struct Database {
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
}

#[derive(Debug, Clone)]
pub struct AccountCredentials {
    pub jwt: String,
    pub signing_key: String,
}

#[derive(Debug, Clone)]
pub struct AccountRecord {
    pub account: Account,
    pub jwt: Option<String>,
    pub signing_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OrderHistoryRecord {
    pub id: String,
    pub task_id: String,
    pub order_id: i64,
    pub symbol: String,
    pub side: Side,
    pub price: Option<Decimal>,
    pub qty: Decimal,
    pub status: OrderStatus,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct TradeHistoryRecord {
    pub id: String,
    pub task_id: String,
    pub trade_id: i64,
    pub order_id: i64,
    pub symbol: String,
    pub side: Side,
    pub price: Decimal,
    pub qty: Decimal,
    pub fee: Decimal,
    pub pnl: Option<Decimal>,
    pub created_at: String,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let manager = r2d2_sqlite::SqliteConnectionManager::file(path).with_init(|conn| {
            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn.pragma_update(None, "foreign_keys", "ON")?;
            Ok(())
        });

        let pool = r2d2::Pool::new(manager).context("create sqlite pool")?;
        let db = Self { pool };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn create_account(
        &self,
        account: &Account,
        credentials: Option<&AccountCredentials>,
    ) -> Result<String> {
        let conn = self.pool.get().context("get sqlite connection")?;
        let chain = encode_enum(&account.chain)?;
        let encrypted_jwt = encrypt_optional(credentials.map(|cred| cred.jwt.as_str()))?;
        let encrypted_signing_key =
            encrypt_optional(credentials.map(|cred| cred.signing_key.as_str()))?;

        conn.execute(
            "INSERT INTO accounts (address, alias, encrypted_jwt, encrypted_signing_key, chain) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                account.address,
                account.alias,
                encrypted_jwt,
                encrypted_signing_key,
                chain
            ],
        )
        .context("insert account")?;

        Ok(conn.last_insert_rowid().to_string())
    }

    pub fn get_account(&self, account_id: &str) -> Result<Option<AccountRecord>> {
        let id = parse_id(account_id)?;
        let conn = self.pool.get().context("get sqlite connection")?;

        let row = conn
            .query_row(
                "SELECT id, address, alias, encrypted_jwt, encrypted_signing_key, chain FROM accounts WHERE id = ?1",
                params![id],
                |row| AccountRow::from_row(row),
            )
            .optional()
            .context("fetch account")?;

        row.map(AccountRecord::try_from).transpose()
    }

    pub fn update_account(
        &self,
        account: &Account,
        credentials: Option<&AccountCredentials>,
    ) -> Result<()> {
        let id = parse_id(&account.id)?;
        let conn = self.pool.get().context("get sqlite connection")?;
        let chain = encode_enum(&account.chain)?;
        let encrypted_jwt = encrypt_optional(credentials.map(|cred| cred.jwt.as_str()))?;
        let encrypted_signing_key =
            encrypt_optional(credentials.map(|cred| cred.signing_key.as_str()))?;

        conn.execute(
            "UPDATE accounts SET address = ?1, alias = ?2, chain = ?3, encrypted_jwt = COALESCE(?4, encrypted_jwt), encrypted_signing_key = COALESCE(?5, encrypted_signing_key) WHERE id = ?6",
            params![
                account.address,
                account.alias,
                chain,
                encrypted_jwt,
                encrypted_signing_key,
                id
            ],
        )
        .context("update account")?;

        Ok(())
    }

    pub fn delete_account(&self, account_id: &str) -> Result<()> {
        let id = parse_id(account_id)?;
        let conn = self.pool.get().context("get sqlite connection")?;
        conn.execute("DELETE FROM accounts WHERE id = ?1", params![id])
            .context("delete account")?;
        Ok(())
    }

    pub fn list_accounts(&self) -> Result<Vec<AccountRecord>> {
        let conn = self.pool.get().context("get sqlite connection")?;
        let mut stmt = conn
            .prepare(
                "SELECT id, address, alias, encrypted_jwt, encrypted_signing_key, chain FROM accounts ORDER BY created_at DESC",
            )
            .context("prepare account list")?;

        let rows = stmt
            .query_map([], |row| AccountRow::from_row(row))
            .context("query account list")?;

        let mut records = Vec::new();
        for row in rows {
            let record = AccountRecord::try_from(row?)?;
            records.push(record);
        }

        Ok(records)
    }

    pub fn create_task(&self, task: &Task) -> Result<String> {
        let conn = self.pool.get().context("get sqlite connection")?;
        let account_id = parse_id(&task.account_id)?;
        let config_json = serde_json::to_string(&task.config).context("serialize task config")?;
        let status = encode_enum(&task.status)?;

        conn.execute(
            "INSERT INTO tasks (account_id, name, symbol, config_json, status) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![account_id, task.name, task.symbol, config_json, status],
        )
        .context("insert task")?;

        Ok(conn.last_insert_rowid().to_string())
    }

    pub fn get_task(&self, task_id: &str) -> Result<Option<Task>> {
        let id = parse_id(task_id)?;
        let conn = self.pool.get().context("get sqlite connection")?;

        let row = conn
            .query_row(
                "SELECT id, account_id, name, symbol, config_json, status FROM tasks WHERE id = ?1",
                params![id],
                |row| TaskRow::from_row(row),
            )
            .optional()
            .context("fetch task")?;

        row.map(Task::try_from).transpose()
    }

    pub fn update_task(&self, task: &Task) -> Result<()> {
        let id = parse_id(&task.id)?;
        let account_id = parse_id(&task.account_id)?;
        let conn = self.pool.get().context("get sqlite connection")?;
        let config_json = serde_json::to_string(&task.config).context("serialize task config")?;
        let status = encode_enum(&task.status)?;

        conn.execute(
            "UPDATE tasks SET account_id = ?1, name = ?2, symbol = ?3, config_json = ?4, status = ?5, updated_at = CURRENT_TIMESTAMP WHERE id = ?6",
            params![account_id, task.name, task.symbol, config_json, status, id],
        )
        .context("update task")?;

        Ok(())
    }

    pub fn delete_task(&self, task_id: &str) -> Result<()> {
        let id = parse_id(task_id)?;
        let conn = self.pool.get().context("get sqlite connection")?;
        conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])
            .context("delete task")?;
        Ok(())
    }

    pub fn list_tasks(&self) -> Result<Vec<Task>> {
        let conn = self.pool.get().context("get sqlite connection")?;
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, name, symbol, config_json, status FROM tasks ORDER BY created_at DESC",
            )
            .context("prepare task list")?;

        let rows = stmt
            .query_map([], |row| TaskRow::from_row(row))
            .context("query task list")?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(Task::try_from(row?)?);
        }

        Ok(tasks)
    }

    pub fn list_tasks_by_account(&self, account_id: &str) -> Result<Vec<Task>> {
        let account_id = parse_id(account_id)?;
        let conn = self.pool.get().context("get sqlite connection")?;
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, name, symbol, config_json, status FROM tasks WHERE account_id = ?1 ORDER BY created_at DESC",
            )
            .context("prepare task list")?;

        let rows = stmt
            .query_map(params![account_id], |row| TaskRow::from_row(row))
            .context("query task list")?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(Task::try_from(row?)?);
        }

        Ok(tasks)
    }

    pub fn save_order(&self, task_id: &str, order: &Order) -> Result<i64> {
        let task_id = parse_id(task_id)?;
        let conn = self.pool.get().context("get sqlite connection")?;
        let side = encode_enum(&order.side)?;
        let status = encode_enum(&order.status)?;
        let price = order.price.as_ref().map(Decimal::to_string);
        let qty = order.qty.to_string();

        conn.execute(
            "INSERT INTO order_history (task_id, order_id, symbol, side, price, qty, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![task_id, order.id, order.symbol, side, price, qty, status],
        )
        .context("insert order history")?;

        Ok(conn.last_insert_rowid())
    }

    pub fn get_orders_by_task(&self, task_id: &str) -> Result<Vec<OrderHistoryRecord>> {
        let task_id = parse_id(task_id)?;
        let conn = self.pool.get().context("get sqlite connection")?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, order_id, symbol, side, price, qty, status, created_at FROM order_history WHERE task_id = ?1 ORDER BY created_at DESC",
            )
            .context("prepare order history list")?;

        let rows = stmt
            .query_map(params![task_id], |row| OrderHistoryRow::from_row(row))
            .context("query order history")?;

        let mut records = Vec::new();
        for row in rows {
            records.push(OrderHistoryRecord::try_from(row?)?);
        }

        Ok(records)
    }

    pub fn save_trade(&self, task_id: &str, trade: &Trade) -> Result<i64> {
        let task_id = parse_id(task_id)?;
        let conn = self.pool.get().context("get sqlite connection")?;
        let side = encode_enum(&trade.side)?;
        let price = trade.price.to_string();
        let qty = trade.qty.to_string();
        let fee = trade.fee_qty.to_string();
        let pnl = Some(trade.pnl.to_string());

        conn.execute(
            "INSERT INTO trade_history (task_id, trade_id, order_id, symbol, side, price, qty, fee, pnl) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                task_id,
                trade.id,
                trade.order_id,
                trade.symbol,
                side,
                price,
                qty,
                fee,
                pnl
            ],
        )
        .context("insert trade history")?;

        Ok(conn.last_insert_rowid())
    }

    pub fn get_trades_by_task(&self, task_id: &str) -> Result<Vec<TradeHistoryRecord>> {
        let task_id = parse_id(task_id)?;
        let conn = self.pool.get().context("get sqlite connection")?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, trade_id, order_id, symbol, side, price, qty, fee, pnl, created_at FROM trade_history WHERE task_id = ?1 ORDER BY created_at DESC",
            )
            .context("prepare trade history list")?;

        let rows = stmt
            .query_map(params![task_id], |row| TradeHistoryRow::from_row(row))
            .context("query trade history")?;

        let mut records = Vec::new();
        for row in rows {
            records.push(TradeHistoryRecord::try_from(row?)?);
        }

        Ok(records)
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.pool.get().context("get sqlite connection")?;
        conn.execute_batch(include_str!("schema.sql"))
            .context("apply schema.sql")?;
        Ok(())
    }
}

struct AccountRow {
    id: i64,
    address: String,
    alias: Option<String>,
    encrypted_jwt: Option<String>,
    encrypted_signing_key: Option<String>,
    chain: String,
}

impl AccountRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            address: row.get(1)?,
            alias: row.get(2)?,
            encrypted_jwt: row.get(3)?,
            encrypted_signing_key: row.get(4)?,
            chain: row.get(5)?,
        })
    }
}

impl TryFrom<AccountRow> for AccountRecord {
    type Error = anyhow::Error;

    fn try_from(row: AccountRow) -> Result<Self> {
        let chain = decode_enum(&row.chain)?;
        let jwt = decrypt_optional(row.encrypted_jwt)?;
        let signing_key = decrypt_optional(row.encrypted_signing_key)?;

        Ok(Self {
            account: Account {
                id: row.id.to_string(),
                address: row.address,
                alias: row.alias.unwrap_or_default(),
                chain,
            },
            jwt,
            signing_key,
        })
    }
}

struct TaskRow {
    id: i64,
    account_id: i64,
    name: String,
    symbol: String,
    config_json: String,
    status: String,
}

impl TaskRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            account_id: row.get(1)?,
            name: row.get(2)?,
            symbol: row.get(3)?,
            config_json: row.get(4)?,
            status: row.get(5)?,
        })
    }
}

impl TryFrom<TaskRow> for Task {
    type Error = anyhow::Error;

    fn try_from(row: TaskRow) -> Result<Self> {
        let status = decode_enum(&row.status)?;
        let config = serde_json::from_str(&row.config_json).context("deserialize task config")?;

        Ok(Self {
            id: row.id.to_string(),
            account_id: row.account_id.to_string(),
            name: row.name,
            symbol: row.symbol,
            config,
            status,
        })
    }
}

struct OrderHistoryRow {
    id: i64,
    task_id: i64,
    order_id: i64,
    symbol: String,
    side: String,
    price: Option<String>,
    qty: String,
    status: String,
    created_at: String,
}

impl OrderHistoryRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            task_id: row.get(1)?,
            order_id: row.get(2)?,
            symbol: row.get(3)?,
            side: row.get(4)?,
            price: row.get(5)?,
            qty: row.get(6)?,
            status: row.get(7)?,
            created_at: row.get(8)?,
        })
    }
}

impl TryFrom<OrderHistoryRow> for OrderHistoryRecord {
    type Error = anyhow::Error;

    fn try_from(row: OrderHistoryRow) -> Result<Self> {
        let side = decode_enum(&row.side)?;
        let status = decode_enum(&row.status)?;
        let price = match row.price {
            Some(value) => Some(Decimal::from_str(&value).context("parse order price")?),
            None => None,
        };
        let qty = Decimal::from_str(&row.qty).context("parse order qty")?;

        Ok(Self {
            id: row.id.to_string(),
            task_id: row.task_id.to_string(),
            order_id: row.order_id,
            symbol: row.symbol,
            side,
            price,
            qty,
            status,
            created_at: row.created_at,
        })
    }
}

struct TradeHistoryRow {
    id: i64,
    task_id: i64,
    trade_id: i64,
    order_id: i64,
    symbol: String,
    side: String,
    price: String,
    qty: String,
    fee: String,
    pnl: Option<String>,
    created_at: String,
}

impl TradeHistoryRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            task_id: row.get(1)?,
            trade_id: row.get(2)?,
            order_id: row.get(3)?,
            symbol: row.get(4)?,
            side: row.get(5)?,
            price: row.get(6)?,
            qty: row.get(7)?,
            fee: row.get(8)?,
            pnl: row.get(9)?,
            created_at: row.get(10)?,
        })
    }
}

impl TryFrom<TradeHistoryRow> for TradeHistoryRecord {
    type Error = anyhow::Error;

    fn try_from(row: TradeHistoryRow) -> Result<Self> {
        let side = decode_enum(&row.side)?;
        let price = Decimal::from_str(&row.price).context("parse trade price")?;
        let qty = Decimal::from_str(&row.qty).context("parse trade qty")?;
        let fee = Decimal::from_str(&row.fee).context("parse trade fee")?;
        let pnl = match row.pnl {
            Some(value) => Some(Decimal::from_str(&value).context("parse trade pnl")?),
            None => None,
        };

        Ok(Self {
            id: row.id.to_string(),
            task_id: row.task_id.to_string(),
            trade_id: row.trade_id,
            order_id: row.order_id,
            symbol: row.symbol,
            side,
            price,
            qty,
            fee,
            pnl,
            created_at: row.created_at,
        })
    }
}

fn parse_id(value: &str) -> Result<i64> {
    value
        .parse::<i64>()
        .with_context(|| format!("invalid id: {value}"))
}

fn encode_enum<T: Serialize>(value: &T) -> Result<String> {
    let serialized = serde_json::to_value(value).context("serialize enum")?;
    serialized
        .as_str()
        .map(str::to_string)
        .context("enum serialized as non-string")
}

fn decode_enum<T: DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(value.to_string())).context("deserialize enum")
}

fn encrypt_optional(value: Option<&str>) -> Result<Option<String>> {
    value.map(encrypt_string).transpose()
}

fn decrypt_optional(value: Option<String>) -> Result<Option<String>> {
    value.as_deref().map(decrypt_string).transpose()
}

fn encrypt_string(value: &str) -> Result<String> {
    let key = derive_key_bytes();
    let cipher = Aes256Gcm::new_from_slice(&key).context("init aes-gcm")?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, value.as_bytes())
        .map_err(|_| anyhow!("encrypt credentials"))?;

    let mut combined = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    Ok(encode_hex(&combined))
}

fn decrypt_string(value: &str) -> Result<String> {
    let data = decode_hex(value)?;
    if data.len() < 12 {
        bail!("encrypted payload too short");
    }

    let (nonce_bytes, ciphertext) = data.split_at(12);
    let key = derive_key_bytes();
    let cipher = Aes256Gcm::new_from_slice(&key).context("init aes-gcm")?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("decrypt credentials"))?;
    String::from_utf8(plaintext).context("decode credentials as utf-8")
}

fn derive_key_bytes() -> [u8; 32] {
    let seed = key_seed();
    let mut key = [0u8; 32];

    for (index, chunk) in key.chunks_mut(8).enumerate() {
        let mut hasher = DefaultHasher::new();
        hasher.write(seed.as_bytes());
        hasher.write_u64(index as u64);
        let digest = hasher.finish().to_le_bytes();
        chunk.copy_from_slice(&digest);
    }

    key
}

fn key_seed() -> String {
    if let Ok(passphrase) = env::var("STANDX_DB_PASSPHRASE") {
        if !passphrase.is_empty() {
            return passphrase;
        }
    }

    let mut parts = Vec::new();
    for key in ["USER", "USERNAME", "HOSTNAME", "COMPUTERNAME", "HOME"] {
        if let Ok(value) = env::var(key) {
            if !value.is_empty() {
                parts.push(value);
            }
        }
    }

    if parts.is_empty() {
        "standx-point-default".to_string()
    } else {
        parts.join("|")
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &value in bytes {
        output.push(HEX[(value >> 4) as usize] as char);
        output.push(HEX[(value & 0x0f) as usize] as char);
    }
    output
}

fn decode_hex(value: &str) -> Result<Vec<u8>> {
    let bytes = value.as_bytes();
    if bytes.len() % 2 != 0 {
        bail!("hex payload length must be even");
    }

    let mut output = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks(2) {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        output.push((high << 4) | low);
    }

    Ok(output)
}

fn hex_value(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => bail!("invalid hex character"),
    }
}
