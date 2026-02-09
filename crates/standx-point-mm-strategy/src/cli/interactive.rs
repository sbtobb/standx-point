/*
[INPUT]:  Stored accounts/tasks and user input via CLI
[OUTPUT]: StrategyConfig for selected tasks or storage updates
[POS]:    CLI interactive flow
[UPDATE]: 2026-02-06 Add interactive CLI task/account management
[UPDATE]: 2026-02-08 Build config using wallet private key auth
*/

use anyhow::{Context, Result, anyhow};
use console::style;
use dialoguer::{Confirm, Input, MultiSelect, Select, theme::ColorfulTheme};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::state::storage::{Account, Storage, Task};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use standx_point_adapter::auth::{EvmWalletSigner, SolanaWalletSigner};
use standx_point_adapter::{AuthManager, Chain, StandxClient, WalletSigner};
use standx_point_mm_strategy::config::{AccountConfig, RiskConfig, StrategyConfig, TaskConfig};

pub async fn run_interactive() -> Result<Option<StrategyConfig>> {
    let theme = ColorfulTheme::default();
    println!("{}", style("StandX MM Strategy CLI").bold().cyan());

    let storage = Arc::new(Storage::new().await?);

    loop {
        let actions = vec![
            "Start tasks",
            "Add account",
            "Edit account",
            "Delete account",
            "Add task",
            "Edit task",
            "Delete task",
            "Exit",
        ];
        let selection = Select::with_theme(&theme)
            .with_prompt("Select action")
            .items(&actions)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                if let Some(config) = select_tasks_to_run(storage.clone(), &theme).await? {
                    return Ok(Some(config));
                }
            }
            1 => create_account(storage.clone(), &theme).await?,
            2 => edit_account(storage.clone(), &theme).await?,
            3 => delete_account(storage.clone(), &theme).await?,
            4 => create_task(storage.clone(), &theme).await?,
            5 => edit_task(storage.clone(), &theme).await?,
            6 => delete_task(storage.clone(), &theme).await?,
            _ => return Ok(None),
        }
    }
}

async fn select_tasks_to_run(
    storage: Arc<Storage>,
    theme: &ColorfulTheme,
) -> Result<Option<StrategyConfig>> {
    let tasks = storage.list_tasks().await?;
    if tasks.is_empty() {
        println!("{}", style("No tasks found.").yellow());
        return Ok(None);
    }

    let accounts = storage.list_accounts().await?;
    let account_names = accounts
        .iter()
        .map(|account| (account.id.clone(), account.name.clone()))
        .collect::<HashMap<_, _>>();

    let items: Vec<String> = tasks
        .iter()
        .map(|task| {
            let name = account_names
                .get(&task.account_id)
                .map(String::as_str)
                .unwrap_or("unknown-account");
            format!("{} | {} | {}", task.id, task.symbol, name)
        })
        .collect();

    let selections = MultiSelect::with_theme(theme)
        .with_prompt("Select tasks to start")
        .items(&items)
        .interact()?;

    if selections.is_empty() {
        println!("{}", style("No tasks selected.").yellow());
        return Ok(None);
    }

    let selected_tasks: Vec<Task> = selections
        .into_iter()
        .map(|idx| tasks[idx].clone())
        .collect();
    let config = build_strategy_config(&storage, &selected_tasks, false).await?;
    print_strategy_summary(&config);

    let confirmed = Confirm::with_theme(theme)
        .with_prompt("Start selected tasks now?")
        .default(true)
        .interact()?;

    if confirmed {
        Ok(Some(config))
    } else {
        Ok(None)
    }
}

async fn select_account(
    storage: &Storage,
    theme: &ColorfulTheme,
    prompt: &str,
) -> Result<Option<Account>> {
    let accounts = storage.list_accounts().await?;
    if accounts.is_empty() {
        println!("{}", style("No accounts found.").yellow());
        return Ok(None);
    }

    let items: Vec<String> = accounts
        .iter()
        .map(|account| format!("{} | {}", account.id, account.name))
        .collect();
    let selection = Select::with_theme(theme)
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact()?;

    Ok(Some(accounts[selection].clone()))
}

async fn select_task(
    storage: &Storage,
    theme: &ColorfulTheme,
    prompt: &str,
) -> Result<Option<Task>> {
    let tasks = storage.list_tasks().await?;
    if tasks.is_empty() {
        println!("{}", style("No tasks found.").yellow());
        return Ok(None);
    }

    let items: Vec<String> = tasks
        .iter()
        .map(|task| format!("{} | {} | {}", task.id, task.symbol, task.account_id))
        .collect();
    let selection = Select::with_theme(theme)
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact()?;

    Ok(Some(tasks[selection].clone()))
}

async fn create_account(storage: Arc<Storage>, theme: &ColorfulTheme) -> Result<()> {
    println!("{}", style("Create new account").bold());
    let name: String = Input::with_theme(theme)
        .with_prompt("Account name")
        .interact_text()?;

    let chains = vec!["bsc", "solana"];
    let chain_index = Select::with_theme(theme)
        .with_prompt("Chain")
        .items(&chains)
        .default(0)
        .interact()?;
    let chain = if chain_index == 0 {
        Chain::Bsc
    } else {
        Chain::Solana
    };

    let private_key: String = Input::with_theme(theme)
        .with_prompt("Wallet private key")
        .interact_text()?;

    let (wallet_address, jwt_token, signing_key) =
        authenticate_account(&private_key, chain).await?;

    let account = Account::new(
        wallet_address,
        name,
        private_key,
        jwt_token,
        signing_key,
        Some(chain),
    );
    storage
        .create_account(account)
        .await
        .context("create account")?;
    println!("{}", style("Account created.").green());
    Ok(())
}

async fn edit_account(storage: Arc<Storage>, theme: &ColorfulTheme) -> Result<()> {
    let Some(account) = select_account(&storage, theme, "Select account to edit").await? else {
        return Ok(());
    };

    println!("{}", style("Edit account").bold());

    let name: String = Input::with_theme(theme)
        .with_prompt("Account name")
        .default(account.name.clone())
        .interact_text()?;

    let mut chain = account.chain.unwrap_or(Chain::Bsc);
    let change_chain = Confirm::with_theme(theme)
        .with_prompt("Change chain?")
        .default(false)
        .interact()?;
    if change_chain {
        let chains = vec!["bsc", "solana"];
        let chain_index = Select::with_theme(theme)
            .with_prompt("Chain")
            .items(&chains)
            .default(if matches!(chain, Chain::Solana) { 1 } else { 0 })
            .interact()?;
        chain = if chain_index == 0 {
            Chain::Bsc
        } else {
            Chain::Solana
        };
    }

    let mut private_key = account.private_key.clone();
    let change_key = Confirm::with_theme(theme)
        .with_prompt("Update wallet private key?")
        .default(false)
        .interact()?;
    if change_key {
        private_key = Input::with_theme(theme)
            .with_prompt("Wallet private key")
            .interact_text()?;
    }

    let mut refresh_credentials = change_chain || change_key;
    if !refresh_credentials {
        refresh_credentials = Confirm::with_theme(theme)
            .with_prompt("Refresh credentials now?")
            .default(false)
            .interact()?;
    }

    if refresh_credentials {
        let (wallet_address, jwt_token, signing_key) =
            authenticate_account(&private_key, chain).await?;
        if wallet_address != account.id {
            return Err(anyhow!(
                "wallet address mismatch: stored={} derived={}",
                account.id,
                wallet_address
            ));
        }
        storage
            .update_account(&account.id, |stored| {
                stored.name = name.clone();
                stored.private_key = private_key.clone();
                stored.chain = Some(chain);
                stored.jwt_token = jwt_token.clone();
                stored.signing_key = signing_key.clone();
            })
            .await?;
    } else {
        storage
            .update_account(&account.id, |stored| {
                stored.name = name.clone();
                stored.private_key = private_key.clone();
                stored.chain = Some(chain);
            })
            .await?;
    }

    println!("{}", style("Account updated.").green());
    Ok(())
}

async fn delete_account(storage: Arc<Storage>, theme: &ColorfulTheme) -> Result<()> {
    let Some(account) = select_account(&storage, theme, "Select account to delete").await? else {
        return Ok(());
    };

    let confirmed = Confirm::with_theme(theme)
        .with_prompt(format!("Delete account '{}' ?", account.name))
        .default(false)
        .interact()?;
    if !confirmed {
        return Ok(());
    }

    storage.delete_account(&account.id).await?;
    println!("{}", style("Account deleted.").green());
    Ok(())
}

async fn create_task(storage: Arc<Storage>, theme: &ColorfulTheme) -> Result<()> {
    let accounts = storage.list_accounts().await?;
    if accounts.is_empty() {
        println!(
            "{}",
            style("No accounts found. Add an account first.").yellow()
        );
        return Ok(());
    }

    println!("{}", style("Create new task").bold());

    let id = Uuid::new_v4().to_string();
    println!(
        "{} {}",
        style("Generated Task ID:").dim(),
        style(&id).cyan()
    );

    let symbols = vec!["BTC-USD", "ETH-USD", "XAG-USD", "XAU-USD"];
    let symbol_index = Select::with_theme(theme)
        .with_prompt("Trading symbol")
        .items(&symbols)
        .default(0)
        .interact()?;
    let symbol = symbols[symbol_index].to_string();

    let account_items: Vec<String> = accounts
        .iter()
        .map(|account| format!("{} | {}", account.id, account.name))
        .collect();
    let account_index = Select::with_theme(theme)
        .with_prompt("Select account")
        .items(&account_items)
        .default(0)
        .interact()?;
    let account_id = accounts[account_index].id.clone();

    let risk_levels = vec![
        "low (5 tiers; widest bands)",
        "medium (3 tiers; balanced bands)",
        "high (2 tiers; tighter bands)",
        "xhigh (1 tier; tightest band)",
    ];
    let risk_index = Select::with_theme(theme)
        .with_prompt("Risk level")
        .items(&risk_levels)
        .default(0)
        .interact()?;
    let risk_level = match risk_index {
        0 => "low",
        1 => "medium",
        2 => "high",
        _ => "xhigh",
    }
    .to_string();

    let budget_usd: String = Input::with_theme(theme)
        .with_prompt("Budget (USD)")
        .default("50000".to_string())
        .interact_text()?;

    let task = Task::new(id, symbol, account_id, risk_level, budget_usd);
    storage.create_task(task).await.context("create task")?;
    println!("{}", style("Task created.").green());
    Ok(())
}

async fn edit_task(storage: Arc<Storage>, theme: &ColorfulTheme) -> Result<()> {
    let Some(task) = select_task(&storage, theme, "Select task to edit").await? else {
        return Ok(());
    };

    let accounts = storage.list_accounts().await?;
    if accounts.is_empty() {
        println!(
            "{}",
            style("No accounts found. Add an account first.").yellow()
        );
        return Ok(());
    }

    println!("{}", style("Edit task").bold());

    let mut symbols = vec!["BTC-USD", "ETH-USD", "XAG-USD", "XAU-USD"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !symbols.iter().any(|symbol| symbol == &task.symbol) {
        symbols.push(task.symbol.clone());
    }
    let symbol_default = symbols
        .iter()
        .position(|symbol| symbol == &task.symbol)
        .unwrap_or(0);
    let symbol_index = Select::with_theme(theme)
        .with_prompt("Trading symbol")
        .items(&symbols)
        .default(symbol_default)
        .interact()?;
    let symbol = symbols[symbol_index].clone();

    let account_items: Vec<String> = accounts
        .iter()
        .map(|account| format!("{} | {}", account.id, account.name))
        .collect();
    let account_default = accounts
        .iter()
        .position(|account| account.id == task.account_id)
        .unwrap_or(0);
    let account_index = Select::with_theme(theme)
        .with_prompt("Select account")
        .items(&account_items)
        .default(account_default)
        .interact()?;
    let account_id = accounts[account_index].id.clone();

    let risk_levels = vec![
        "low (5 tiers; widest bands)",
        "medium (3 tiers; balanced bands)",
        "high (2 tiers; tighter bands)",
        "xhigh (1 tier; tightest band)",
    ];
    let risk_default = match task.risk_level.as_str() {
        "medium" => 1,
        "high" => 2,
        "xhigh" => 3,
        _ => 0,
    };
    let risk_index = Select::with_theme(theme)
        .with_prompt("Risk level")
        .items(&risk_levels)
        .default(risk_default)
        .interact()?;
    let risk_level = match risk_index {
        0 => "low",
        1 => "medium",
        2 => "high",
        _ => "xhigh",
    }
    .to_string();

    let budget_usd: String = Input::with_theme(theme)
        .with_prompt("Budget (USD)")
        .default(task.budget_usd.clone())
        .interact_text()?;

    storage
        .update_task(&task.id, |stored| {
            stored.symbol = symbol;
            stored.account_id = account_id;
            stored.risk_level = risk_level;
            stored.budget_usd = budget_usd;
        })
        .await?;

    println!("{}", style("Task updated.").green());
    Ok(())
}

async fn delete_task(storage: Arc<Storage>, theme: &ColorfulTheme) -> Result<()> {
    let Some(task) = select_task(&storage, theme, "Select task to delete").await? else {
        return Ok(());
    };

    let confirmed = Confirm::with_theme(theme)
        .with_prompt(format!("Delete task '{}' ?", task.id))
        .default(false)
        .interact()?;
    if !confirmed {
        return Ok(());
    }

    storage.delete_task(&task.id).await?;
    println!("{}", style("Task deleted.").green());
    Ok(())
}

pub(crate) async fn build_strategy_config(
    storage: &Storage,
    tasks: &[Task],
    quiet: bool,
) -> Result<StrategyConfig> {
    let mut configs = Vec::with_capacity(tasks.len());
    let mut refreshed_accounts: HashMap<String, Account> = HashMap::new();
    for task in tasks {
        let account = if let Some(account) = refreshed_accounts.get(&task.account_id) {
            account.clone()
        } else {
            let account = storage.get_account(&task.account_id).await.ok_or_else(|| {
                anyhow!(
                    "account '{}' not found for task '{}'",
                    task.account_id,
                    task.id
                )
            })?;
            let refreshed = refresh_account(storage, &account, quiet).await?;
            refreshed_accounts.insert(task.account_id.clone(), refreshed.clone());
            refreshed
        };
        let task_config = TaskConfig {
            id: task.id.clone(),
            symbol: task.symbol.clone(),
            account_id: account.id.clone(),
            risk: RiskConfig {
                level: task.risk_level.clone(),
                budget_usd: task.budget_usd.clone(),
            },
        };
        configs.push(task_config);
    }
    let accounts = refreshed_accounts
        .values()
        .map(|account| AccountConfig {
            id: account.id.clone(),
            private_key: non_empty(&account.private_key),
            jwt_token: non_empty(&account.jwt_token),
            signing_key: non_empty(&account.signing_key),
            chain: account.chain.unwrap_or(Chain::Bsc),
        })
        .collect();
    Ok(StrategyConfig {
        accounts,
        tasks: configs,
    })
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

async fn refresh_account(storage: &Storage, account: &Account, quiet: bool) -> Result<Account> {
    let chain = account
        .chain
        .ok_or_else(|| anyhow!("account chain not set"))?;
    if account.private_key.trim().is_empty() {
        return Err(anyhow!("account private key is missing"));
    }

    if !quiet {
        println!(
            "{} {}",
            style("Refreshing credentials for account:").dim(),
            style(&account.id).cyan()
        );
    }

    let (wallet_address, jwt_token, signing_key) =
        authenticate_account(&account.private_key, chain).await?;

    if wallet_address != account.id {
        return Err(anyhow!(
            "wallet address mismatch: stored={} derived={}",
            account.id,
            wallet_address
        ));
    }

    storage
        .update_account(&account.id, |stored| {
            stored.jwt_token = jwt_token.clone();
            stored.signing_key = signing_key.clone();
        })
        .await
        .context("update account credentials")?;

    if !quiet {
        println!(
            "{} {}",
            style("Credentials refreshed for account:").dim(),
            style(&account.id).green()
        );
    }

    let mut refreshed = account.clone();
    refreshed.jwt_token = jwt_token;
    refreshed.signing_key = signing_key;
    Ok(refreshed)
}

fn print_strategy_summary(config: &StrategyConfig) {
    println!("{}", style("Selected task parameters").bold());
    for task in &config.tasks {
        println!(
            "- {} | {} | account={} | risk={} | budget_usd={}",
            task.id, task.symbol, task.account_id, task.risk.level, task.risk.budget_usd
        );
    }
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
