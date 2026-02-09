/*
[INPUT]:  AppState with storage, task manager, live client helpers, and task selections
[OUTPUT]: AppState refresh helpers for tasks, snapshots, and live data
[POS]:    TUI state refresh logic
[UPDATE]: 2026-02-09 Add placeholder module for TUI refactor
[UPDATE]: 2026-02-09 Move AppState refresh helpers from app.rs
[UPDATE]: 2026-02-10 Add price snapshot refresh for live task data
*/

use std::time::Instant;

use anyhow::{anyhow, Result};

use super::app::{AppState, LiveTaskData, PriceSnapshot, UiSnapshot};
use crate::tui::runtime::{build_live_client, query_open_orders_with_fallback, LIVE_REFRESH_INTERVAL};

impl AppState {
    pub(super) async fn refresh_accounts(&mut self) -> Result<()> {
        self.accounts = self.storage.list_accounts().await?;
        Ok(())
    }

    pub(super) async fn refresh_tasks(&mut self) -> Result<()> {
        let tasks = self.storage.list_tasks().await?;
        self.tasks = tasks;
        if self.tasks.is_empty() {
            self.list_state.select(None);
        } else if self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        } else if let Some(selected) = self.list_state.selected() {
            if selected >= self.tasks.len() {
                self.list_state.select(Some(self.tasks.len().saturating_sub(1)));
            }
        }
        self.last_refresh = Instant::now();
        Ok(())
    }

    pub(super) async fn build_snapshot(&self) -> Result<UiSnapshot> {
        let manager = self.task_manager.lock().await;
        let runtime_status = manager.runtime_status_snapshot();
        let metrics = manager.task_metrics_snapshot().await;
        drop(manager);

        Ok(UiSnapshot {
            runtime_status,
            metrics,
        })
    }

    pub(super) async fn refresh_live_data(&mut self) -> Result<()> {
        if self.last_live_refresh.elapsed() < LIVE_REFRESH_INTERVAL {
            return Ok(());
        }

        let Some(task) = self.selected_task().cloned() else {
            return Ok(());
        };

        let account = self
            .storage
            .get_account(&task.account_id)
            .await
            .ok_or_else(|| anyhow!("account not found: {}", task.account_id))?;

        let client = build_live_client(&account)?;
        let symbol = task.symbol.as_str();

        let mut data = self
            .live_data
            .remove(&task.id)
            .unwrap_or_else(LiveTaskData::empty);
        let mut errors = Vec::new();

        match client.query_symbol_price(symbol).await {
            Ok(response) => {
                let mark_price = response.mark_price;
                let last_price = response.last_price;
                let min_price = match last_price.as_ref() {
                    Some(last) => std::cmp::min(mark_price.clone(), last.clone()),
                    None => mark_price.clone(),
                };
                data.price_data = Some(PriceSnapshot {
                    mark_price,
                    last_price,
                    min_price,
                });
            }
            Err(err) => errors.push(format!("price: {err}")),
        }

        match client.query_balance().await {
            Ok(balance) => data.balance = Some(balance),
            Err(err) => errors.push(format!("balance: {err}")),
        }

        match client.query_positions(Some(symbol)).await {
            Ok(positions) => data.positions = positions,
            Err(err) => errors.push(format!("positions: {err}")),
        }

        match query_open_orders_with_fallback(&client, symbol).await {
            Ok(orders) => data.open_orders = orders.result,
            Err(err) => errors.push(format!("open_orders: {err}")),
        }

        data.last_update = Some(Instant::now());
        data.last_error = if errors.is_empty() {
            None
        } else {
            Some(errors.join(" | "))
        };

        self.live_data.insert(task.id.clone(), data);
        self.last_live_refresh = Instant::now();
        Ok(())
    }
}
