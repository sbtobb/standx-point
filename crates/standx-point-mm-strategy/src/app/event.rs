use ratatui::crossterm::event::KeyEvent;
use rust_decimal::Decimal;

/// All possible events that can occur in the application
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppEvent {
    /// Periodic tick for UI updates (every 250ms)
    Tick,

    /// Keyboard input from user
    Key(KeyEvent),

    /// Price update from market data hub (symbol, price)
    PriceUpdate(String, Decimal),

    /// Task status changed (task_id, new_state)
    TaskStatusChange(String, TaskState),

    /// Account was created
    AccountCreated(String), // account_id

    /// Account was updated
    AccountUpdated(String), // account_id

    /// Account was deleted
    AccountDeleted(String), // account_id

    /// Task was created
    TaskCreated(String), // task_id

    /// Task was updated
    TaskUpdated(String), // task_id

    /// Task was deleted
    TaskDeleted(String), // task_id

    /// Modal/dialog should be closed
    ModalClose,

    /// Application should shut down
    Shutdown,

    /// Terminal resize event (width, height)
    Resize(u16, u16),
}

/// Task states for status updates
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TaskState {
    Init,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed(String), // Error message
}

/// Trait for types that can handle application events
#[allow(dead_code)]
pub trait EventHandler {
    /// Handle an application event
    fn handle_event(&mut self, event: AppEvent) -> anyhow::Result<()>;
}
