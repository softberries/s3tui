//! Benchmark tests for s3tui transfer operations
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use s3tui::model::local_selected_item::LocalSelectedItem;
use s3tui::model::s3_selected_item::S3SelectedItem;
use s3tui::model::state::State;
use s3tui::model::transfer_state::TransferState;
use s3tui::services::transfer_manager::JobId;
use s3tui::settings::file_credentials::FileCredential;

fn create_test_credential() -> FileCredential {
    FileCredential {
        name: "bench-test".to_string(),
        access_key: "bench_key".to_string(),
        secret_key: "bench_secret".to_string(),
        default_region: "us-east-1".to_string(),
        selected: true,
        endpoint_url: None,
        force_path_style: false,
    }
}

fn create_local_selected_items(count: usize) -> Vec<LocalSelectedItem> {
    (0..count)
        .map(|i| LocalSelectedItem {
            name: format!("file-{}.txt", i),
            path: format!("/path/to/file-{}.txt", i),
            is_directory: false,
            destination_bucket: "test-bucket".to_string(),
            destination_path: format!("file-{}.txt", i),
            s3_creds: create_test_credential(),
            children: None,
            transfer_state: TransferState::InProgress(0.0),
            job_id: Some(JobId::from(i as u64)),
        })
        .collect()
}

fn create_s3_selected_items(count: usize) -> Vec<S3SelectedItem> {
    (0..count)
        .map(|i| S3SelectedItem {
            bucket: Some("test-bucket".to_string()),
            name: format!("s3-file-{}.txt", i),
            path: Some(format!("s3-file-{}.txt", i)),
            is_directory: false,
            is_bucket: false,
            destination_dir: "/tmp/downloads".to_string(),
            s3_creds: create_test_credential(),
            children: None,
            transfer_state: TransferState::InProgress(0.0),
            job_id: Some(JobId::from(i as u64)),
        })
        .collect()
}

fn bench_progress_tracking_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("progress_tracking");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("overall_upload_progress", size),
            size,
            |b, &size| {
                let creds = vec![create_test_credential()];
                let mut state = State::new(creds);
                state.local_selected_items = create_local_selected_items(size);

                // Set varying progress for items
                for (i, item) in state.local_selected_items.iter_mut().enumerate() {
                    item.transfer_state = TransferState::InProgress((i % 100) as f64);
                }

                b.iter(|| {
                    let progress: f64 = state
                        .local_selected_items
                        .iter()
                        .map(|item| item.transfer_state.progress())
                        .sum::<f64>()
                        / state.local_selected_items.len() as f64;
                    black_box(progress)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("overall_download_progress", size),
            size,
            |b, &size| {
                let creds = vec![create_test_credential()];
                let mut state = State::new(creds);
                state.s3_selected_items = create_s3_selected_items(size);

                // Set varying progress for items
                for (i, item) in state.s3_selected_items.iter_mut().enumerate() {
                    item.transfer_state = TransferState::InProgress((i % 100) as f64);
                }

                b.iter(|| {
                    let progress: f64 = state
                        .s3_selected_items
                        .iter()
                        .map(|item| item.transfer_state.progress())
                        .sum::<f64>()
                        / state.s3_selected_items.len() as f64;
                    black_box(progress)
                });
            },
        );
    }

    group.finish();
}

fn bench_state_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_creation");

    for cred_count in [1, 5, 10].iter() {
        group.bench_with_input(
            BenchmarkId::new("new_state", cred_count),
            cred_count,
            |b, &count| {
                let creds: Vec<FileCredential> = (0..count)
                    .map(|i| FileCredential {
                        name: format!("cred-{}", i),
                        access_key: format!("key-{}", i),
                        secret_key: format!("secret-{}", i),
                        default_region: "us-east-1".to_string(),
                        selected: i == 0,
                        endpoint_url: None,
                        force_path_style: false,
                    })
                    .collect();

                b.iter(|| {
                    let state = State::new(black_box(creds.clone()));
                    black_box(state)
                });
            },
        );
    }

    group.finish();
}

fn bench_item_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("item_selection");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("add_local_selected_item", size),
            size,
            |b, &size| {
                let creds = vec![create_test_credential()];
                let mut state = State::new(creds);
                state.local_selected_items = create_local_selected_items(size);

                let new_item = LocalSelectedItem {
                    name: "new-file.txt".to_string(),
                    path: "/path/to/new-file.txt".to_string(),
                    is_directory: false,
                    destination_bucket: "test-bucket".to_string(),
                    destination_path: "new-file.txt".to_string(),
                    s3_creds: create_test_credential(),
                    children: None,
                    transfer_state: TransferState::Pending,
                    job_id: None,
                };

                b.iter(|| {
                    state.add_local_selected_item(black_box(new_item.clone()));
                    // Remove it so we can add again
                    state.local_selected_items.pop();
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("add_s3_selected_item", size),
            size,
            |b, &size| {
                let creds = vec![create_test_credential()];
                let mut state = State::new(creds);
                state.s3_selected_items = create_s3_selected_items(size);

                let new_item = S3SelectedItem {
                    bucket: Some("test-bucket".to_string()),
                    name: "new-s3-file.txt".to_string(),
                    path: Some("new-s3-file.txt".to_string()),
                    is_directory: false,
                    is_bucket: false,
                    destination_dir: "/tmp/downloads".to_string(),
                    s3_creds: create_test_credential(),
                    children: None,
                    transfer_state: TransferState::Pending,
                    job_id: None,
                };

                b.iter(|| {
                    state.add_s3_selected_item(black_box(new_item.clone()));
                    // Remove it so we can add again
                    state.s3_selected_items.pop();
                });
            },
        );
    }

    group.finish();
}

fn bench_transfer_state_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("transfer_state");

    group.bench_function("progress_extraction", |b| {
        let states = vec![
            TransferState::Pending,
            TransferState::InProgress(50.0),
            TransferState::Paused(25.0),
            TransferState::Completed,
            TransferState::Failed("error".to_string()),
            TransferState::Cancelled,
        ];

        b.iter(|| {
            for state in &states {
                black_box(state.progress());
            }
        });
    });

    group.bench_function("state_checks", |b| {
        let state = TransferState::InProgress(50.0);

        b.iter(|| {
            black_box(state.is_completed());
            black_box(state.is_paused());
            black_box(state.is_terminal());
            black_box(state.is_cancelled());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_progress_tracking_overhead,
    bench_state_creation,
    bench_item_selection,
    bench_transfer_state_operations,
);
criterion_main!(benches);
