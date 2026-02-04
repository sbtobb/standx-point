/*
[INPUT]:  TaskStatus from state module, TaskAction enum
[OUTPUT]: Validated state transitions for tasks
[POS]:    Task domain logic - state machine for lifecycle management
[UPDATE]: When task status transitions or error handling needs refinement
*/

use crate::state::TaskStatus;
use thiserror::Error;

/// Actions that can trigger task state transitions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskAction {
    Save,
    Start,
    Pause,
    Resume,
    Stop,
    Edit,
    Error(String),
}

/// Errors occurring during state transitions
#[derive(Debug, Clone, Error)]
pub enum StateError {
    #[error("Invalid transition: {from:?} -> {action:?}")]
    InvalidTransition {
        from: TaskStatus,
        action: TaskAction,
    },
}

/// State machine managing task lifecycle transitions
pub struct TaskStateMachine {
    current_state: TaskStatus,
}

impl TaskStateMachine {
    /// Create a new task state machine with an initial state
    pub fn new(initial: TaskStatus) -> Self {
        Self {
            current_state: initial,
        }
    }

    /// Check if a transition to another state via the given action is valid
    pub fn can_transition(&self, action: &TaskAction) -> bool {
        let from = self.current_state;
        match (from, action) {
            (TaskStatus::Draft, TaskAction::Save) => true,
            (TaskStatus::Pending, TaskAction::Start) => true,
            (TaskStatus::Running, TaskAction::Pause) => true,
            (TaskStatus::Running, TaskAction::Stop) => true,
            (TaskStatus::Running, TaskAction::Error(_)) => true,
            (TaskStatus::Paused, TaskAction::Resume) => true,
            (TaskStatus::Paused, TaskAction::Stop) => true,
            (_, TaskAction::Edit) => true,
            _ => false,
        }
    }

    /// Perform a state transition
    pub fn transition(&mut self, action: TaskAction) -> Result<(), StateError> {
        if !self.can_transition(&action) {
            return Err(StateError::InvalidTransition {
                from: self.current_state,
                action,
            });
        }

        let next_state = match (self.current_state, action) {
            (TaskStatus::Draft, TaskAction::Save) => TaskStatus::Pending,
            (TaskStatus::Pending, TaskAction::Start) => TaskStatus::Running,
            (TaskStatus::Running, TaskAction::Pause) => TaskStatus::Paused,
            (TaskStatus::Running, TaskAction::Stop) => TaskStatus::Stopped,
            (TaskStatus::Running, TaskAction::Error(_)) => TaskStatus::Failed,
            (TaskStatus::Paused, TaskAction::Resume) => TaskStatus::Running,
            (TaskStatus::Paused, TaskAction::Stop) => TaskStatus::Stopped,
            (_, TaskAction::Edit) => TaskStatus::Draft,
            // All other valid transitions are covered above
            _ => unreachable!(),
        };

        self.current_state = next_state;
        Ok(())
    }

    /// Get the current state
    pub fn state(&self) -> TaskStatus {
        self.current_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let machine = TaskStateMachine::new(TaskStatus::Draft);
        assert_eq!(machine.state(), TaskStatus::Draft);
    }

    #[test]
    fn test_valid_transitions() {
        let mut machine = TaskStateMachine::new(TaskStatus::Draft);
        assert!(machine.transition(TaskAction::Save).is_ok());
        assert_eq!(machine.state(), TaskStatus::Pending);

        assert!(machine.transition(TaskAction::Start).is_ok());
        assert_eq!(machine.state(), TaskStatus::Running);

        assert!(machine.transition(TaskAction::Pause).is_ok());
        assert_eq!(machine.state(), TaskStatus::Paused);

        assert!(machine.transition(TaskAction::Resume).is_ok());
        assert_eq!(machine.state(), TaskStatus::Running);

        assert!(machine.transition(TaskAction::Stop).is_ok());
        assert_eq!(machine.state(), TaskStatus::Stopped);
    }

    #[test]
    fn test_edit_transition_from_any_state() {
        let states = [
            TaskStatus::Draft,
            TaskStatus::Pending,
            TaskStatus::Running,
            TaskStatus::Paused,
            TaskStatus::Stopped,
            TaskStatus::Failed,
        ];

        for &initial in &states {
            let mut machine = TaskStateMachine::new(initial);
            assert!(machine.transition(TaskAction::Edit).is_ok());
            assert_eq!(machine.state(), TaskStatus::Draft);
        }
    }

    #[test]
    fn test_error_transition() {
        let mut machine = TaskStateMachine::new(TaskStatus::Running);
        assert!(machine
            .transition(TaskAction::Error("Test error".to_string()))
            .is_ok());
        assert_eq!(machine.state(), TaskStatus::Failed);
    }

    #[test]
    fn test_invalid_transition() {
        let mut machine = TaskStateMachine::new(TaskStatus::Pending);
        let result = machine.transition(TaskAction::Pause);
        assert!(result.is_err());
        if let Err(StateError::InvalidTransition { from, action }) = result {
            assert_eq!(from, TaskStatus::Pending);
            assert_eq!(action, TaskAction::Pause);
        }
    }
}
