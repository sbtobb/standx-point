/*
[INPUT]:  Individual task components
[OUTPUT]: Unified task module for GUI crate
[POS]:    Task domain layer - aggregates state machine and task-related types
[UPDATE]: When adding new task-related modules or functionality
*/

pub mod state_machine;

pub use state_machine::{StateError, TaskAction, TaskStateMachine};
