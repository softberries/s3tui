//! Type-state pattern for transfer items
//!
//! This module provides a state machine representation for transfer lifecycle,
//! preventing invalid states from being representable.

use std::fmt;

/// Represents the lifecycle state of a transfer operation.
///
/// Using an enum instead of separate boolean/option fields ensures
/// only valid state combinations are possible at compile time.
#[derive(Debug, Clone, PartialEq)]
pub enum TransferState {
    /// Transfer has not started yet
    Pending,
    /// Transfer is in progress with a percentage (0.0 to 100.0)
    InProgress(f64),
    /// Transfer completed successfully
    Completed,
    /// Transfer failed with an error message
    Failed(String),
}

impl Default for TransferState {
    fn default() -> Self {
        TransferState::Pending
    }
}

impl TransferState {
    /// Returns true if the transfer has completed successfully
    pub fn is_completed(&self) -> bool {
        matches!(self, TransferState::Completed)
    }

    /// Returns true if the transfer has failed
    pub fn is_failed(&self) -> bool {
        matches!(self, TransferState::Failed(_))
    }

    /// Returns true if the transfer is still pending
    pub fn is_pending(&self) -> bool {
        matches!(self, TransferState::Pending)
    }

    /// Returns true if the transfer is currently in progress
    pub fn is_in_progress(&self) -> bool {
        matches!(self, TransferState::InProgress(_))
    }

    /// Returns true if the transfer is finished (completed or failed)
    pub fn is_finished(&self) -> bool {
        self.is_completed() || self.is_failed()
    }

    /// Returns the progress percentage (0.0 for pending, 100.0 for completed, actual value for in-progress)
    pub fn progress(&self) -> f64 {
        match self {
            TransferState::Pending => 0.0,
            TransferState::InProgress(p) => *p,
            TransferState::Completed => 100.0,
            TransferState::Failed(_) => 0.0,
        }
    }

    /// Returns the error message if the transfer failed
    pub fn error(&self) -> Option<&str> {
        match self {
            TransferState::Failed(msg) => Some(msg),
            _ => None,
        }
    }

    /// Transition to in-progress state with given progress
    pub fn set_progress(progress: f64) -> Self {
        TransferState::InProgress(progress.clamp(0.0, 100.0))
    }

    /// Transition to completed state
    pub fn complete() -> Self {
        TransferState::Completed
    }

    /// Transition to failed state with given error
    pub fn fail(error: impl Into<String>) -> Self {
        TransferState::Failed(error.into())
    }
}

impl fmt::Display for TransferState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransferState::Pending => write!(f, "Pending"),
            TransferState::InProgress(p) => write!(f, "In Progress ({:.1}%)", p),
            TransferState::Completed => write!(f, "Completed"),
            TransferState::Failed(msg) => write!(f, "Failed: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_pending() {
        let state = TransferState::default();
        assert!(state.is_pending());
        assert_eq!(state.progress(), 0.0);
    }

    #[test]
    fn test_in_progress_state() {
        let state = TransferState::InProgress(50.0);
        assert!(state.is_in_progress());
        assert!(!state.is_finished());
        assert_eq!(state.progress(), 50.0);
    }

    #[test]
    fn test_completed_state() {
        let state = TransferState::Completed;
        assert!(state.is_completed());
        assert!(state.is_finished());
        assert_eq!(state.progress(), 100.0);
    }

    #[test]
    fn test_failed_state() {
        let state = TransferState::Failed("Network error".into());
        assert!(state.is_failed());
        assert!(state.is_finished());
        assert_eq!(state.error(), Some("Network error"));
        assert_eq!(state.progress(), 0.0);
    }

    #[test]
    fn test_set_progress_clamps_values() {
        let state = TransferState::set_progress(150.0);
        assert_eq!(state.progress(), 100.0);

        let state = TransferState::set_progress(-10.0);
        assert_eq!(state.progress(), 0.0);
    }

    #[test]
    fn test_display_formatting() {
        assert_eq!(format!("{}", TransferState::Pending), "Pending");
        assert_eq!(
            format!("{}", TransferState::InProgress(50.5)),
            "In Progress (50.5%)"
        );
        assert_eq!(format!("{}", TransferState::Completed), "Completed");
        assert_eq!(
            format!("{}", TransferState::Failed("Error".into())),
            "Failed: Error"
        );
    }

    #[test]
    fn test_transfer_state_transitions_are_valid() {
        // Start pending
        let state = TransferState::default();
        assert!(state.is_pending());

        // Transition to in progress
        let state = TransferState::set_progress(25.0);
        assert!(state.is_in_progress());
        assert_eq!(state.progress(), 25.0);

        // Update progress
        let state = TransferState::set_progress(75.0);
        assert_eq!(state.progress(), 75.0);

        // Complete successfully
        let state = TransferState::complete();
        assert!(state.is_completed());
        assert!(state.is_finished());

        // Or fail
        let state = TransferState::fail("Connection lost");
        assert!(state.is_failed());
        assert!(state.is_finished());
        assert_eq!(state.error(), Some("Connection lost"));
    }
}
