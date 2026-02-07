use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use standx_point_adapter::Chain;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::Mutex;

/// Account data structure for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub private_key: String,
    pub jwt_token: String,
    pub signing_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<Chain>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Account {
    pub fn new(
        id: String,
        name: String,
        private_key: String,
        jwt_token: String,
        signing_key: String,
        chain: Option<Chain>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            private_key,
            jwt_token,
            signing_key,
            chain,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(anyhow!("Account name cannot be empty"));
        }
        if self.private_key.is_empty() {
            return Err(anyhow!("Account private key cannot be empty"));
        }
        if self.jwt_token.is_empty() {
            return Err(anyhow!("JWT token cannot be empty"));
        }
        if self.signing_key.is_empty() {
            return Err(anyhow!("Signing key cannot be empty"));
        }
        Ok(())
    }
}

/// Task data structure for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub symbol: String,
    pub account_id: String,
    pub risk_level: String,
    #[serde(alias = "max_position_usd")]
    pub budget_usd: String,
    pub state: TaskState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskState {
    Stopped,
    Running,
    Failed(String),
}

impl Task {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        symbol: String,
        account_id: String,
        risk_level: String,
        budget_usd: String,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            symbol,
            account_id,
            risk_level,
            budget_usd,
            state: TaskState::Stopped,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(anyhow!("Task ID cannot be empty"));
        }
        if self.symbol.is_empty() {
            return Err(anyhow!("Symbol cannot be empty"));
        }
        if self.account_id.is_empty() {
            return Err(anyhow!("Account ID cannot be empty"));
        }
        if self.budget_usd.is_empty() {
            return Err(anyhow!("Budget USD cannot be empty"));
        }
        Ok(())
    }
}

/// Storage manager for accounts and tasks
#[derive(Debug)]
pub struct Storage {
    accounts_path: PathBuf,
    tasks_path: PathBuf,
    accounts: Mutex<HashMap<String, Account>>,
    tasks: Mutex<HashMap<String, Task>>,
}

impl Storage {
    pub async fn new() -> Result<Self> {
        let data_dir = Self::default_data_dir()?;
        fs::create_dir_all(&data_dir).await?;

        if let Some(legacy_dir) = Self::legacy_data_dir() {
            Self::migrate_legacy_files(&data_dir, &legacy_dir).await?;
        }

        let accounts_path = data_dir.join("accounts.json");
        let tasks_path = data_dir.join("tasks.json");

        let accounts = Self::load_accounts(&accounts_path).await?;
        let tasks = Self::load_tasks(&tasks_path).await?;

        Ok(Self {
            accounts_path,
            tasks_path,
            accounts: Mutex::new(accounts),
            tasks: Mutex::new(tasks),
        })
    }

    /// Test-only constructor that creates storage in a specific directory.
    /// Creates the directory if it doesn't exist.
    #[cfg(test)]
    pub async fn new_in_dir(data_dir: &Path) -> Result<Self> {
        fs::create_dir_all(data_dir).await?;

        let accounts_path = data_dir.join("accounts.json");
        let tasks_path = data_dir.join("tasks.json");

        let accounts = Self::load_accounts(&accounts_path).await?;
        let tasks = Self::load_tasks(&tasks_path).await?;

        Ok(Self {
            accounts_path,
            tasks_path,
            accounts: Mutex::new(accounts),
            tasks: Mutex::new(tasks),
        })
    }

    async fn load_accounts(path: &Path) -> Result<HashMap<String, Account>> {
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let content = fs::read_to_string(path).await?;
        let accounts: Vec<Account> = serde_json::from_str(&content)?;
        Ok(accounts.into_iter().map(|a| (a.id.clone(), a)).collect())
    }

    async fn load_tasks(path: &Path) -> Result<HashMap<String, Task>> {
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let content = fs::read_to_string(path).await?;
        let tasks: Vec<Task> = serde_json::from_str(&content)?;
        Ok(tasks.into_iter().map(|t| (t.id.clone(), t)).collect())
    }

    fn default_data_dir() -> Result<PathBuf> {
        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Ok(base_dir.join(".standx-config"))
    }

    fn legacy_data_dir() -> Option<PathBuf> {
        dirs::data_dir().map(|dir| dir.join("standx-mm"))
    }

    async fn migrate_legacy_files(new_dir: &Path, legacy_dir: &Path) -> Result<()> {
        if !legacy_dir.exists() {
            return Ok(());
        }

        let legacy_accounts = legacy_dir.join("accounts.json");
        let legacy_tasks = legacy_dir.join("tasks.json");
        let new_accounts = new_dir.join("accounts.json");
        let new_tasks = new_dir.join("tasks.json");

        Self::migrate_file_if_missing(&legacy_accounts, &new_accounts).await?;
        Self::migrate_file_if_missing(&legacy_tasks, &new_tasks).await?;

        Ok(())
    }

    async fn migrate_file_if_missing(legacy_path: &Path, new_path: &Path) -> Result<()> {
        if new_path.exists() || !legacy_path.exists() {
            return Ok(());
        }

        if let Err(err) = fs::rename(legacy_path, new_path).await {
            fs::copy(legacy_path, new_path).await.map_err(|copy_err| {
                anyhow!(
                    "migrate {} -> {} failed: rename={err}; copy={copy_err}",
                    legacy_path.display(),
                    new_path.display()
                )
            })?;
        }

        Ok(())
    }

    pub async fn create_account(&self, account: Account) -> Result<()> {
        account.validate()?;
        let mut accounts = self.accounts.lock().await;
        if accounts.contains_key(&account.id) {
            return Err(anyhow!("Account with ID '{}' already exists", account.id));
        }
        accounts.insert(account.id.clone(), account);
        self.save_accounts(&accounts).await?;
        Ok(())
    }

    pub async fn update_account(&self, id: &str, f: impl FnOnce(&mut Account)) -> Result<()> {
        let mut accounts = self.accounts.lock().await;
        let account = accounts
            .get_mut(id)
            .ok_or_else(|| anyhow!("Account '{}' not found", id))?;
        f(account);
        account.updated_at = chrono::Utc::now();
        self.save_accounts(&accounts).await?;
        Ok(())
    }

    pub async fn delete_account(&self, id: &str) -> Result<()> {
        let tasks = self.tasks.lock().await;
        let has_tasks = tasks.values().any(|t| t.account_id == id);
        if has_tasks {
            return Err(anyhow!(
                "Cannot delete account '{}' because it has associated tasks",
                id
            ));
        }
        drop(tasks);

        let mut accounts = self.accounts.lock().await;
        if accounts.remove(id).is_none() {
            return Err(anyhow!("Account '{}' not found", id));
        }
        self.save_accounts(&accounts).await?;
        Ok(())
    }

    pub async fn list_accounts(&self) -> Result<Vec<Account>> {
        let accounts = self.accounts.lock().await;
        let mut list: Vec<_> = accounts.values().cloned().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(list)
    }

    pub async fn get_account(&self, id: &str) -> Option<Account> {
        self.accounts.lock().await.get(id).cloned()
    }

    // Task operations (similar pattern)
    pub async fn create_task(&self, task: Task) -> Result<()> {
        task.validate()?;
        let mut tasks = self.tasks.lock().await;
        if tasks.contains_key(&task.id) {
            return Err(anyhow!("Task with ID '{}' already exists", task.id));
        }
        tasks.insert(task.id.clone(), task);
        self.save_tasks(&tasks).await?;
        Ok(())
    }

    pub async fn update_task(&self, id: &str, f: impl FnOnce(&mut Task)) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        let task = tasks
            .get_mut(id)
            .ok_or_else(|| anyhow!("Task '{}' not found", id))?;
        f(task);
        task.updated_at = chrono::Utc::now();
        self.save_tasks(&tasks).await?;
        Ok(())
    }

    pub async fn delete_task(&self, id: &str) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        if tasks.remove(id).is_none() {
            return Err(anyhow!("Task '{}' not found", id));
        }
        self.save_tasks(&tasks).await?;
        Ok(())
    }

    pub async fn list_tasks(&self) -> Result<Vec<Task>> {
        let tasks = self.tasks.lock().await;
        let mut list: Vec<_> = tasks.values().cloned().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(list)
    }

    #[allow(dead_code)]
    pub async fn list_tasks_for_account(&self, account_id: &str) -> Result<Vec<Task>> {
        let tasks = self.tasks.lock().await;
        let mut list: Vec<_> = tasks
            .values()
            .filter(|t| t.account_id == account_id)
            .cloned()
            .collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(list)
    }

    #[allow(dead_code)]
    pub async fn get_task(&self, id: &str) -> Option<Task> {
        self.tasks.lock().await.get(id).cloned()
    }

    // Private helper methods
    async fn save_accounts(&self, accounts: &HashMap<String, Account>) -> Result<()> {
        let list: Vec<_> = accounts.values().cloned().collect();
        let content = serde_json::to_string_pretty(&list)?;

        // Atomic write: write to temp file then rename
        let temp_path = self.accounts_path.with_extension("tmp");
        fs::write(&temp_path, content).await?;
        fs::rename(&temp_path, &self.accounts_path).await?;
        Ok(())
    }

    async fn save_tasks(&self, tasks: &HashMap<String, Task>) -> Result<()> {
        let list: Vec<_> = tasks.values().cloned().collect();
        let content = serde_json::to_string_pretty(&list)?;

        let temp_path = self.tasks_path.with_extension("tmp");
        fs::write(&temp_path, content).await?;
        fs::rename(&temp_path, &self.tasks_path).await?;
        Ok(())
    }
}
