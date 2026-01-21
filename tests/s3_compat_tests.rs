//! Integration tests for S3-compatible storage using MinIO
//!
//! These tests require Docker to be running. They spin up a MinIO container
//! and test the S3DataFetcher against it.
//!
//! Run with: cargo test --test s3_compat_tests -- --ignored

use s3tui::model::local_selected_item::LocalSelectedItem;
use s3tui::model::s3_selected_item::S3SelectedItem;
use s3tui::model::transfer_state::TransferState;
use s3tui::services::s3_data_fetcher::S3DataFetcher;
use s3tui::settings::file_credentials::FileCredential;
use std::io::Write;
use tempfile::NamedTempFile;
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::minio::MinIO;
use tokio::sync::mpsc;

/// Default MinIO minio
const MINIO_ACCESS_KEY: &str = "minioadmin";
const MINIO_SECRET_KEY: &str = "minioadmin";

/// Create a FileCredential configured for a MinIO container
fn create_minio_credential(port: u16) -> FileCredential {
    FileCredential {
        name: "minio-test".to_string(),
        access_key: MINIO_ACCESS_KEY.to_string(),
        secret_key: MINIO_SECRET_KEY.to_string(),
        default_region: "us-east-1".to_string(),
        selected: true,
        endpoint_url: Some(format!("http://127.0.0.1:{}", port)),
        force_path_style: true,
    }
}

/// Start a MinIO container and return it along with configured minio
async fn setup_minio() -> (ContainerAsync<MinIO>, FileCredential) {
    let container = MinIO::default()
        .start()
        .await
        .expect("Failed to start MinIO container");

    let port = container
        .get_host_port_ipv4(9000)
        .await
        .expect("Failed to get MinIO port");

    let creds = create_minio_credential(port);
    (container, creds)
}

/// Create a test file with specified content
fn create_test_file(content: &[u8]) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content).expect("Failed to write to temp file");
    file.flush().expect("Failed to flush temp file");
    file
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_minio_list_buckets() {
    let (_container, creds) = setup_minio().await;
    let fetcher = S3DataFetcher::new(creds);

    // List buckets - should be empty initially
    let buckets = fetcher
        .list_current_location(None, None)
        .await
        .expect("Failed to list buckets");
    assert!(buckets.is_empty(), "Expected no buckets initially");
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_minio_create_and_list_bucket() {
    let (_container, creds) = setup_minio().await;
    let fetcher = S3DataFetcher::new(creds);

    // Create a bucket
    let bucket_name = "test-bucket";
    let result = fetcher
        .create_bucket(bucket_name.to_string(), "us-east-1".to_string())
        .await
        .expect("Failed to create bucket");
    assert!(result.is_none(), "Expected no error creating bucket");

    // List buckets - should have one bucket now
    let buckets = fetcher
        .list_current_location(None, None)
        .await
        .expect("Failed to list buckets");
    assert_eq!(buckets.len(), 1, "Expected one bucket");
    assert_eq!(buckets[0].name, bucket_name);
    assert!(buckets[0].is_bucket, "Expected item to be a bucket");
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_minio_upload_and_list_objects() {
    let (_container, creds) = setup_minio().await;
    let fetcher = S3DataFetcher::new(creds.clone());

    // Create a bucket
    let bucket_name = "upload-test-bucket";
    fetcher
        .create_bucket(bucket_name.to_string(), "us-east-1".to_string())
        .await
        .expect("Failed to create bucket");

    // Create a test file
    let test_content = b"Hello, MinIO!";
    let test_file = create_test_file(test_content);
    let file_path = test_file.path().to_str().unwrap().to_string();

    // Create upload progress channel
    let (tx, mut rx) = mpsc::channel(100);

    // Create LocalSelectedItem for upload
    let upload_item = LocalSelectedItem {
        name: "test-file.txt".to_string(),
        path: file_path,
        is_directory: false,
        destination_bucket: bucket_name.to_string(),
        destination_path: "test-file.txt".to_string(),
        s3_creds: creds.clone(),
        children: None,
        transfer_state: TransferState::Pending,
        job_id: None,
    };

    // Upload the file
    let result = fetcher.upload_item(upload_item, tx, None).await;
    assert!(result.is_ok(), "Upload should succeed");

    // Drain the progress channel
    rx.close();
    while rx.recv().await.is_some() {}

    // List objects in bucket
    let objects = fetcher
        .list_current_location(Some(bucket_name.to_string()), None)
        .await
        .expect("Failed to list objects");

    assert_eq!(objects.len(), 1, "Expected one object");
    assert_eq!(objects[0].name, "test-file.txt");
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_minio_upload_and_download() {
    let (_container, creds) = setup_minio().await;
    let fetcher = S3DataFetcher::new(creds.clone());

    // Create a bucket
    let bucket_name = "download-test-bucket";
    fetcher
        .create_bucket(bucket_name.to_string(), "us-east-1".to_string())
        .await
        .expect("Failed to create bucket");

    // Create a test file with known content
    let test_content = b"Test content for download verification";
    let test_file = create_test_file(test_content);
    let file_path = test_file.path().to_str().unwrap().to_string();

    // Upload progress channel
    let (upload_tx, mut upload_rx) = mpsc::channel(100);

    // Create LocalSelectedItem for upload
    let upload_item = LocalSelectedItem {
        name: "download-test.txt".to_string(),
        path: file_path,
        is_directory: false,
        destination_bucket: bucket_name.to_string(),
        destination_path: "download-test.txt".to_string(),
        s3_creds: creds.clone(),
        children: None,
        transfer_state: TransferState::Pending,
        job_id: None,
    };

    // Upload
    fetcher
        .upload_item(upload_item, upload_tx, None)
        .await
        .expect("Upload failed");

    // Drain upload progress channel
    upload_rx.close();
    while upload_rx.recv().await.is_some() {}

    // Create temp directory for download
    let download_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let download_path = download_dir.path().to_str().unwrap().to_string();

    // Download progress channel
    let (download_tx, mut download_rx) = mpsc::channel(100);

    // Create S3SelectedItem for download
    let download_item = S3SelectedItem {
        bucket: Some(bucket_name.to_string()),
        name: "download-test.txt".to_string(),
        path: Some("download-test.txt".to_string()),
        is_directory: false,
        is_bucket: false,
        destination_dir: download_path.clone(),
        s3_creds: creds,
        children: None,
        transfer_state: TransferState::Pending,
        job_id: None,
    };

    // Download
    fetcher
        .download_item(download_item, download_tx, None)
        .await
        .expect("Download failed");

    // Drain download progress channel
    download_rx.close();
    while download_rx.recv().await.is_some() {}

    // Verify downloaded content
    let downloaded_content =
        std::fs::read(format!("{}/download-test.txt", download_path)).expect("Failed to read downloaded file");
    assert_eq!(
        downloaded_content, test_content,
        "Downloaded content should match original"
    );
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_minio_delete_object() {
    let (_container, creds) = setup_minio().await;
    let fetcher = S3DataFetcher::new(creds.clone());

    // Create a bucket
    let bucket_name = "delete-test-bucket";
    fetcher
        .create_bucket(bucket_name.to_string(), "us-east-1".to_string())
        .await
        .expect("Failed to create bucket");

    // Create and upload a test file
    let test_file = create_test_file(b"Delete me");
    let file_path = test_file.path().to_str().unwrap().to_string();

    let (tx, mut rx) = mpsc::channel(100);

    let upload_item = LocalSelectedItem {
        name: "to-delete.txt".to_string(),
        path: file_path,
        is_directory: false,
        destination_bucket: bucket_name.to_string(),
        destination_path: "to-delete.txt".to_string(),
        s3_creds: creds.clone(),
        children: None,
        transfer_state: TransferState::Pending,
        job_id: None,
    };

    fetcher.upload_item(upload_item, tx, None).await.expect("Upload failed");
    rx.close();
    while rx.recv().await.is_some() {}

    // Verify object exists
    let objects_before = fetcher
        .list_current_location(Some(bucket_name.to_string()), None)
        .await
        .expect("Failed to list objects");
    assert_eq!(objects_before.len(), 1);

    // Delete the object
    let delete_result = fetcher
        .delete_data(false, Some(bucket_name.to_string()), "to-delete.txt".to_string(), false)
        .await
        .expect("Delete failed");
    assert!(delete_result.is_none(), "Expected no error on delete");

    // Verify object is gone
    let objects_after = fetcher
        .list_current_location(Some(bucket_name.to_string()), None)
        .await
        .expect("Failed to list objects");
    assert!(objects_after.is_empty(), "Object should be deleted");
}

#[tokio::test]
#[ignore] // Requires Docker
async fn test_minio_list_objects_with_prefix() {
    let (_container, creds) = setup_minio().await;
    let fetcher = S3DataFetcher::new(creds.clone());

    // Create a bucket
    let bucket_name = "prefix-test-bucket";
    fetcher
        .create_bucket(bucket_name.to_string(), "us-east-1".to_string())
        .await
        .expect("Failed to create bucket");

    // Upload files with different prefixes
    let files = vec![
        ("folder1/file1.txt", b"content1" as &[u8]),
        ("folder1/file2.txt", b"content2"),
        ("folder2/file3.txt", b"content3"),
        ("root-file.txt", b"root content"),
    ];

    for (key, content) in files {
        let test_file = create_test_file(content);
        let file_path = test_file.path().to_str().unwrap().to_string();
        let (tx, mut rx) = mpsc::channel(100);

        let upload_item = LocalSelectedItem {
            name: key.split('/').last().unwrap().to_string(),
            path: file_path,
            is_directory: false,
            destination_bucket: bucket_name.to_string(),
            destination_path: key.to_string(),
            s3_creds: creds.clone(),
            children: None,
            transfer_state: TransferState::Pending,
            job_id: None,
        };

        fetcher.upload_item(upload_item, tx, None).await.expect("Upload failed");
        rx.close();
        while rx.recv().await.is_some() {}
    }

    // List all objects at root level
    let all_objects = fetcher
        .list_current_location(Some(bucket_name.to_string()), None)
        .await
        .expect("Failed to list objects");

    // Should have 2 "folders" (prefixes) and 1 root file at root level
    // MinIO returns common prefixes as directories
    assert!(
        all_objects.len() >= 2,
        "Expected at least 2 items at root level, got {}",
        all_objects.len()
    );

    // List objects in folder1/
    let folder1_objects = fetcher
        .list_current_location(Some(bucket_name.to_string()), Some("folder1/".to_string()))
        .await
        .expect("Failed to list folder1 objects");
    assert_eq!(folder1_objects.len(), 2, "Expected 2 files in folder1/");
}
