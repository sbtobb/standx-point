/*
[INPUT]:  Storage handle, task manager, log buffer, live task snapshots
[OUTPUT]: AppState helpers for TUI rendering and task control
[POS]:    TUI app state and snapshot management
[UPDATE]: 2026-02-09 Add placeholder module for TUI refactor
[UPDATE]: 2026-02-09 Move AppState, LiveTaskData, and UiSnapshot from tui/mod.rs
[UPDATE]: 2026-02-09 Add AppMode enum placeholder for TUI flows
[UPDATE]: 2026-02-09 Add tab state for TUI navigation
[UPDATE]: 2026-02-10 Add price snapshot data to live task data
[UPDATE]: 2026-02-10 Add active modal state to AppState
[UPDATE]: 2026-02-10 Allow dead_code on modal scaffolding
[UPDATE]: 2026-02-10 Implement modal submit flows for accounts and tasks
*/

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use ratatui::widgets::ListState;
use rust_decimal::Decimal;
use standx_point_adapter::auth::{EvmWalletSigner, SolanaWalletSigner};
use standx_point_adapter::{
    AuthManager, Balance, Chain, Order, Position, StandxClient, WalletSigner,
};
use standx_point_mm_strategy::metrics::TaskMetricsSnapshot;
use standx_point_mm_strategy::task::TaskRuntimeStatus;
use standx_point_mm_strategy::TaskManager;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

use crate::cli::interactive::build_strategy_config;
use crate::state::storage::{Account as StoredAccount, Storage, Task as StoredTask};
use crate::tui::ui::modal::{CreateAccountModal, CreateTaskModal};
use crate::tui::runtime::LIVE_REFRESH_INTERVAL;
use crate::tui::LogBufferHandle;

#[allow(dead_code)]
pub(super) enum AppMode {
    Dashboard,
    LogsTab,
    CreateAccount,
    CreateTask,
}

pub(super) enum ActiveModal {
    CreateAccount(CreateAccountModal),
    CreateTask(CreateTaskModal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tab {
    Dashboard,
    Logs,
    Create,
}

pub(super) struct UiSnapshot {
    pub(super) runtime_status: HashMap<String, TaskRuntimeStatus>,
    pub(super) metrics: HashMap<String, TaskMetricsSnapshot>,
}

#[derive(Debug)]
pub(super) struct PriceSnapshot {
    pub(super) mark_price: Decimal,
    pub(super) last_price: Option<Decimal>,
    pub(super) min_price: Decimal,
}

#[derive(Debug)]
pub(super) struct LiveTaskData {
    pub(super) balance: Option<Balance>,
    pub(super) positions: Vec<Position>,
    pub(super) open_orders: Vec<Order>,
    pub(super) price_data: Option<PriceSnapshot>,
    pub(super) last_update: Option<Instant>,
    pub(super) last_error: Option<String>,
}

impl LiveTaskData {
    pub(super) fn empty() -> Self {
        Self {
            balance: None,
            positions: Vec::new(),
            open_orders: Vec::new(),
            price_data: None,
            last_update: None,
            last_error: None,
        }
    }
}

pub(super) struct AppState {
    pub(super) storage: Arc<Storage>,
    pub(super) task_manager: Arc<TokioMutex<TaskManager>>,
    pub(super) log_buffer: LogBufferHandle,
    pub(super) accounts: Vec<StoredAccount>,
    pub(super) tasks: Vec<StoredTask>,
    pub(super) list_state: ListState,
    pub(super) current_tab: Tab,
    pub(super) status_message: String,
    pub(super) last_refresh: Instant,
    pub(super) last_live_refresh: Instant,
    pub(super) live_data: HashMap<String, LiveTaskData>,
    pub(super) active_modal: Option<ActiveModal>,
}

impl AppState {
    pub(super) fn new(
        storage: Arc<Storage>,
        task_manager: Arc<TokioMutex<TaskManager>>,
        log_buffer: LogBufferHandle,
    ) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            storage,
            task_manager,
            log_buffer,
            accounts: Vec::new(),
            tasks: Vec::new(),
            list_state,
            current_tab: Tab::Dashboard,
            status_message: "Ready".to_string(),
            last_refresh: Instant::now() - Duration::from_secs(10),
            last_live_refresh: Instant::now() - LIVE_REFRESH_INTERVAL,
            live_data: HashMap::new(),
            active_modal: None,
        }
    }

    pub(super) fn open_create_account(&mut self) {
        self.active_modal = Some(ActiveModal::CreateAccount(CreateAccountModal::new()));
    }

    pub(super) async fn open_create_task(&mut self) -> Result<()> {
        if self.accounts.is_empty() {
            self.refresh_accounts().await?;
        }

        if self.accounts.is_empty() {
            self.status_message = "no accounts found; create one first".to_string();
            return Ok(());
        }

        let account_options = self
            .accounts
            .iter()
            .map(|account| (account.id.clone(), format!("{} | {}", account.id, account.name)))
            .collect();
        let symbols = default_task_symbols();
        let id = Uuid::new_v4().to_string();

        self.active_modal = Some(ActiveModal::CreateTask(CreateTaskModal::new(
            id,
            symbols,
            account_options,
        )));
        Ok(())
    }

    pub(super) fn close_modal(&mut self) {
        self.active_modal = None;
    }

    pub(super) fn active_modal_mut(&mut self) -> Option<&mut ActiveModal> {
        self.active_modal.as_mut()
    }

    pub(super) fn selected_task(&self) -> Option<&StoredTask> {
        let idx = self.list_state.selected().unwrap_or(0);
        self.tasks.get(idx)
    }

    pub(super) fn selected_live_data(&self) -> Option<&LiveTaskData> {
        let task = self.selected_task()?;
        self.live_data.get(&task.id)
    }

    pub(super) fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Dashboard => Tab::Logs,
            Tab::Logs => Tab::Create,
            Tab::Create => Tab::Dashboard,
        };
    }

    pub(super) fn set_tab(&mut self, tab: Tab) {
        self.current_tab = tab;
    }

    pub(super) async fn submit_create_account(
        &mut self,
        name: String,
        private_key: String,
        chain: Chain,
    ) -> Result<()> {
        let (wallet_address, jwt_token, signing_key) =
            authenticate_account(&private_key, chain).await?;
        let account = StoredAccount::new(
            wallet_address.clone(),
            name,
            private_key,
            jwt_token,
            signing_key,
            Some(chain),
        );
        self.storage
            .create_account(account)
            .await
            .context("create account")?;
        self.refresh_accounts().await?;
        self.status_message = format!("account created: {}", wallet_address);
        Ok(())
    }

    pub(super) async fn submit_create_task(
        &mut self,
        id: String,
        symbol: String,
        account_id: String,
        risk_level: String,
        budget_usd: String,
    ) -> Result<()> {
        let task = StoredTask::new(id.clone(), symbol, account_id, risk_level, budget_usd);
        self.storage
            .create_task(task)
            .await
            .context("create task")?;
        self.refresh_tasks().await?;
        self.status_message = format!("task created: {}", id);
        Ok(())
    }

    pub(super) async fn start_selected_task(&mut self) -> Result<()> {
        let task = self
            .selected_task()
            .cloned()
            .ok_or_else(|| anyhow!("no task selected"))?;

        let config = build_strategy_config(&self.storage, std::slice::from_ref(&task), true).await?;

        let mut manager = self.task_manager.lock().await;
        if manager.runtime_status(&task.id).is_some() {
            self.status_message = format!("task already running: {}", task.id);
            return Ok(());
        }
        manager.spawn_from_config(config).await?;
        self.status_message = format!("task started: {}", task.id);
        Ok(())
    }

    pub(super) async fn stop_selected_task(&mut self) -> Result<()> {
        let task = self
            .selected_task()
            .cloned()
            .ok_or_else(|| anyhow!("no task selected"))?;

        let mut manager = self.task_manager.lock().await;
        manager.stop_task(&task.id).await?;
        self.status_message = format!("task stopped: {}", task.id);
        Ok(())
    }

    pub(super) fn move_selection(&mut self, delta: isize) {
        if self.tasks.is_empty() {
            self.list_state.select(None);
            return;
        }
        let current = self.list_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, (self.tasks.len() - 1) as isize) as usize;
        self.list_state.select(Some(next));
        self.last_live_refresh = Instant::now() - LIVE_REFRESH_INTERVAL;
    }
}

fn default_task_symbols() -> Vec<String> {
    vec![
        String::from("BTC-USD"),
        String::from("ETH-USD"),
        String::from("XAG-USD"),
        String::from("XAU-USD"),
    ]
}

async fn authenticate_account(private_key: &str, chain: Chain) -> Result<(String, String, String)> {
    let client = StandxClient::new().map_err(|err| anyhow!("create StandxClient failed: {err}"))?;
    let auth = AuthManager::new(client);
    let (wallet_address, login_response): (String, _) = match chain {
        Chain::Bsc => {
            let wallet = EvmWalletSigner::new(private_key)
                .map_err(|err| anyhow!("invalid EVM private key: {err}"))?;
            let address = wallet.address().to_string();
            let login = auth
                .authenticate(&wallet, 7 * 24 * 60 * 60)
                .await
                .map_err(|err| anyhow!("authenticate failed: {err}"))?;
            (address, login)
        }
        Chain::Solana => {
            let wallet = SolanaWalletSigner::new(private_key)
                .map_err(|err| anyhow!("invalid Solana private key: {err}"))?;
            let address = wallet.address().to_string();
            let login = auth
                .authenticate(&wallet, 7 * 24 * 60 * 60)
                .await
                .map_err(|err| anyhow!("authenticate failed: {err}"))?;
            (address, login)
        }
    };

    let signer = auth
        .key_manager()
        .get_or_create_signer(&wallet_address)
        .map_err(|err| anyhow!("load ed25519 signer failed: {err}"))?;
    let signing_key = STANDARD.encode(signer.secret_key_bytes());

    Ok((wallet_address, login_response.token, signing_key))
}
