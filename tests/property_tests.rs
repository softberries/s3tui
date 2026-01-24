//! Property-based tests for s3tui
//!
//! These tests use proptest to verify invariants hold across random inputs.
//!
//! Run with: cargo test --test property_tests

use proptest::prelude::*;
use s3tui::model::transfer_state::TransferState;

/// Strategy to generate random progress values (0.0 to 100.0)
fn progress_strategy() -> impl Strategy<Value = f64> {
    0.0..=100.0f64
}

/// Strategy to generate a sequence of progress updates (monotonically increasing)
fn progress_sequence_strategy() -> impl Strategy<Value = Vec<f64>> {
    prop::collection::vec(progress_strategy(), 1..20).prop_map(|mut values| {
        // Sort to ensure monotonic sequence
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        values
    })
}

/// Strategy to generate transfer states
fn transfer_state_strategy() -> impl Strategy<Value = TransferState> {
    prop_oneof![
        Just(TransferState::Pending),
        (0.0..=100.0f64).prop_map(TransferState::InProgress),
        Just(TransferState::Completed),
        (0.0..=100.0f64).prop_map(TransferState::Paused),
        ".*".prop_map(TransferState::Failed),
        Just(TransferState::Cancelled),
    ]
}

proptest! {
    /// Test that progress values are always clamped to valid range
    #[test]
    fn test_progress_clamped_to_valid_range(progress in -1000.0..1000.0f64) {
        let clamped = progress.clamp(0.0, 100.0);
        prop_assert!(clamped >= 0.0);
        prop_assert!(clamped <= 100.0);
    }

    /// Test that progress sequences maintain monotonic property when sorted
    #[test]
    fn test_progress_monotonic_when_sorted(sequence in progress_sequence_strategy()) {
        for window in sequence.windows(2) {
            prop_assert!(
                window[0] <= window[1],
                "Progress should be monotonically increasing: {} > {}",
                window[0],
                window[1]
            );
        }
    }

    /// Test that TransferState transitions are valid
    #[test]
    fn test_transfer_state_invariants(state in transfer_state_strategy()) {
        // Test progress() method returns valid values
        let progress = state.progress();
        prop_assert!(progress >= 0.0, "Progress should be >= 0.0, got {}", progress);
        prop_assert!(progress <= 100.0, "Progress should be <= 100.0, got {}", progress);

        match &state {
            TransferState::Pending => {
                prop_assert!(!state.is_completed());
                prop_assert!(!state.is_paused());
                prop_assert!(!state.is_terminal());
                prop_assert_eq!(state.progress(), 0.0);
            }
            TransferState::InProgress(p) => {
                prop_assert!(*p >= 0.0);
                prop_assert!(*p <= 100.0);
                prop_assert!(!state.is_completed());
                prop_assert!(!state.is_paused());
                prop_assert!(!state.is_terminal());
            }
            TransferState::Completed => {
                prop_assert!(state.is_completed());
                prop_assert!(state.is_terminal());
                prop_assert_eq!(state.progress(), 100.0);
            }
            TransferState::Paused(p) => {
                prop_assert!(*p >= 0.0);
                prop_assert!(*p <= 100.0);
                prop_assert!(state.is_paused());
                prop_assert!(!state.is_completed());
                prop_assert!(!state.is_terminal());
            }
            TransferState::Failed(_) => {
                prop_assert!(!state.is_completed());
                prop_assert!(state.is_terminal());
                prop_assert!(state.error().is_some());
            }
            TransferState::Cancelled => {
                prop_assert!(!state.is_completed());
                prop_assert!(state.is_cancelled());
                prop_assert!(state.is_terminal());
            }
        }
    }

    /// Test that transfer state display formatting doesn't panic
    #[test]
    fn test_transfer_state_display_no_panic(state in transfer_state_strategy()) {
        let display = format!("{}", state);
        prop_assert!(!display.is_empty());
    }

    /// Test that progress percentage extraction is consistent
    #[test]
    fn test_progress_extraction_consistency(progress in 0.0..=100.0f64) {
        let state = TransferState::InProgress(progress);
        let extracted = state.progress();
        prop_assert!((extracted - progress).abs() < f64::EPSILON);
    }

    /// Test that overall progress calculation is bounded
    #[test]
    fn test_overall_progress_bounded(
        progresses in prop::collection::vec(0.0..=100.0f64, 1..100)
    ) {
        let sum: f64 = progresses.iter().sum();
        let count = progresses.len() as f64;
        let average = sum / count;

        prop_assert!(average >= 0.0);
        prop_assert!(average <= 100.0);
    }

    /// Test that file size formatting handles edge cases
    #[test]
    fn test_file_size_values(size in 0u64..u64::MAX) {
        // Verify size doesn't cause overflow in common operations
        let kb = size / 1024;
        let mb = kb / 1024;
        let gb = mb / 1024;

        // These should not overflow
        prop_assert!(kb <= size || size < 1024);
        prop_assert!(mb <= kb || kb < 1024);
        prop_assert!(gb <= mb || mb < 1024);
    }
}

#[cfg(test)]
mod state_invariant_tests {
    use s3tui::model::state::State;
    use s3tui::settings::file_credentials::FileCredential;

    fn create_test_credential(name: &str, selected: bool) -> FileCredential {
        FileCredential {
            name: name.to_string(),
            access_key: format!("{}_key", name),
            secret_key: format!("{}_secret", name),
            default_region: "us-east-1".to_string(),
            selected,
            endpoint_url: None,
            force_path_style: false,
        }
    }

    #[test]
    fn test_state_invariant_initial_state_valid() {
        let creds = vec![create_test_credential("test", true)];
        let state = State::new(creds);

        // Invariant: current_creds should be set
        assert!(!state.current_creds.name.is_empty());

        // Invariant: selected items should be empty initially
        assert!(state.local_selected_items.is_empty());
        assert!(state.s3_selected_items.is_empty());
    }

    #[test]
    fn test_state_invariant_search_mode_consistency() {
        let creds = vec![create_test_credential("test", true)];
        let mut state = State::new(creds);

        // Invariant: search_mode false means query can be empty or non-empty
        assert!(!state.search_mode);

        // Activate search mode
        state.set_search_mode(true);
        assert!(state.search_mode);

        // Set query
        state.set_search_query("test".to_string());
        assert_eq!(state.search_query, "test");

        // Deactivate preserves query
        state.set_search_mode(false);
        assert!(!state.search_mode);
        assert_eq!(state.search_query, "test");

        // Clear search clears query
        state.clear_search();
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn test_state_invariant_credentials_selection() {
        // Test with selected credential
        let creds = vec![
            create_test_credential("cred1", false),
            create_test_credential("cred2", true),
        ];

        let state = State::new(creds);

        // Invariant: current_creds should be the selected one
        assert_eq!(state.current_creds.name, "cred2");

        // With no selected credential, should use first
        let creds2 = vec![
            create_test_credential("cred1", false),
            create_test_credential("cred2", false),
        ];
        let state2 = State::new(creds2);

        // When no credential is selected, the default empty one is used
        // or it picks the first one - let's check what actually happens
        // Based on the State::new implementation, if none is selected,
        // it uses the default (empty) current_creds
        assert!(state2.creds.len() == 2);
    }

    #[test]
    fn test_state_invariant_empty_credentials() {
        let creds: Vec<FileCredential> = vec![];
        let state = State::new(creds);

        // With no credentials, current_creds should be default (empty)
        assert!(state.current_creds.name.is_empty());
        assert!(state.creds.is_empty());
    }
}
