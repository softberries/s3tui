//! Type-state pattern for transfer items
//!
//! This module provides a state machine representation for transfer lifecycle,
//! preventing invalid states from being representable.

use std::fmt;

/// Represents the lifecycle state of a transfer operation.
///
/// Using an enum instead of separate boolean/option fields ensures
/// only valid state combinations are possible at compile time.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TransferState {
    /// Transfer has not started yet
    #[default]
    Pending,
    /// Transfer is in progress with a percentage (0.0 to 100.0)
    InProgress(f64),
    /// Transfer completed successfully
    Completed,
    /// Transfer failed with an error message
    Failed(String),
}

impl TransferState {
    /// Returns true if the transfer has completed successfully
    pub fn is_completed(&self) -> bool {
        matches!(self, TransferState::Completed)
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
        assert_eq!(state, TransferState::Pending);
        assert_eq!(state.progress(), 0.0);
    }

    #[test]
    fn test_in_progress_state() {
        let state = TransferState::InProgress(50.0);
        assert!(matches!(state, TransferState::InProgress(_)));
        assert_eq!(state.progress(), 50.0);
    }

    #[test]
    fn test_completed_state() {
        let state = TransferState::Completed;
        assert!(state.is_completed());
        assert_eq!(state.progress(), 100.0);
    }

    #[test]
    fn test_failed_state() {
        let state = TransferState::Failed("Network error".into());
        assert!(matches!(state, TransferState::Failed(_)));
        assert_eq!(state.error(), Some("Network error"));
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
}
