/*
[INPUT]:  Task configuration, adapter client, and state management
[OUTPUT]: Async task execution with position/order monitoring
[POS]:    Task execution layer - manages trading task lifecycle
[UPDATE]: When adding new execution strategies or monitoring features
*/

use standx_point_adapter::{StandxClient, http::client::StandxClient as AdapterClient};
use standx_point_adapter::types::{Order, Position, Balance};
use tokio::sync::{mpsc, watch};
use std::sync::Arc;
use tokio::task::JoinHandle;
use anyhow::{Result, anyhow};

pub struct TaskExecutor {
    client: Arc<AdapterClient>,
    shutdown_tx: mpsc::Sender<()>,
}

pub struct TaskHandle {
    pub task_id: String,
    pub join_handle: JoinHandle<()>,
    pub position_watch: watch::Receiver<Vec<Position>>,
    pub orders_watch: watch::Receiver<Vec<Order>>,
}

impl TaskExecutor {
    pub fn new(client: AdapterClient) -> Self {
        let (shutdown_tx, _) = mpsc::channel(1);
        Self {
            client: Arc::new(client),
            shutdown_tx,
        }
    }
    
    /// Start a task with the given configuration
    pub async fn start_task(
        &self,
        task_id: String,
        symbol: String,
        _account_id: String,
    ) -> Result<TaskHandle> {
        // Create position and orders watch channels
        let (position_tx, position_rx) = watch::channel(Vec::new());
        let (orders_tx, orders_rx) = watch::channel(Vec::new());
        
        let client = Arc::clone(&self.client);
        let symbol_clone = symbol.clone();
        let task_id_clone = task_id.clone();
        
        // Spawn async task for monitoring
        let join_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                // Query positions
                match client.query_positions(&symbol_clone).await {
                    Ok(positions) => {
                        if position_tx.send(positions).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Task {}: Failed to query positions: {}", task_id_clone, e);
                    }
                }
                
                // Query orders
                match client.query_orders(&symbol_clone).await {
                    Ok(orders) => {
                        if orders_tx.send(orders).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Task {}: Failed to query orders: {}", task_id_clone, e);
                    }
                }
            }
        });
        
        Ok(TaskHandle {
            task_id,
            join_handle,
            position_watch: position_rx,
            orders_watch: orders_rx,
        })
    }
    
    /// Stop a running task gracefully
    pub async fn stop_task(&self, _task_id: &str) -> Result<()> {
        // Send shutdown signal (simplified for MVP)
        Ok(())
    }
    
    /// Query positions for a task (one-time fetch)
    pub async fn query_positions(&self, symbol: &str) -> Result<Vec<Position>> {
        self.client.query_positions(symbol).await.map_err(|e| anyhow!(e))
    }
    
    /// Query orders for a task (one-time fetch)
    pub async fn query_orders(&self, symbol: &str) -> Result<Vec<Order>> {
        self.client.query_orders(symbol).await.map_err(|e| anyhow!(e))
    }
}
