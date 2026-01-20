use crate::model::local_selected_item::LocalSelectedItem;
use crate::model::s3_data_item::{BucketInfo, FileInfo, S3DataItem};
use crate::model::s3_selected_item::S3SelectedItem;
use crate::services::transfer_manager::PauseSignal;
use crate::services::transfer_state::TransferStateStore;
use crate::settings::file_credentials::FileCredential;
use aws_sdk_s3::config::{Credentials, Region};
use aws_smithy_runtime_api::http::Request;
use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::{
    convert::Infallible,
    fs,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;

use crate::model::download_progress_item::DownloadProgressItem;
use crate::model::upload_progress_item::UploadProgressItem;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::types::{BucketLocationConstraint, CreateBucketConfiguration};
use aws_sdk_s3::{
    primitives::{ByteStream, SdkBody},
    Client,
};
use aws_smithy_types::error::metadata::ProvideErrorMetadata;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use bytes::Bytes;
use color_eyre::{eyre, Report};
use http_body::{Body, SizeHint};
use tokio::io::AsyncReadExt;

/// Threshold above which multipart upload is used (100 MB)
const MULTIPART_THRESHOLD: u64 = 100 * 1024 * 1024;

/// Minimum part size for multipart upload (5 MB - AWS minimum)
const MIN_PART_SIZE: u64 = 5 * 1024 * 1024;

/// Default part size for multipart upload (8 MB - good balance)
const DEFAULT_PART_SIZE: u64 = 8 * 1024 * 1024;

/// Maximum number of parts allowed by S3
const MAX_PARTS: u64 = 10_000;

/// Maximum retry attempts for failed part uploads
const MAX_PART_RETRIES: u32 = 3;

/// Delay between retry attempts (in milliseconds)
const RETRY_DELAY_MS: u64 = 1000;

/// How often to persist download progress (in bytes)
const DOWNLOAD_PROGRESS_PERSIST_INTERVAL: u64 = 10 * 1024 * 1024; // 10 MB

/// Cache key for S3 clients - combines credentials identity and region
#[derive(Clone, Debug)]
struct ClientCacheKey {
    access_key_id: String,
    region: String,
}

impl PartialEq for ClientCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.access_key_id == other.access_key_id && self.region == other.region
    }
}

impl Eq for ClientCacheKey {}

impl Hash for ClientCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.access_key_id.hash(state);
        self.region.hash(state);
    }
}

/// Pool of S3 clients for reuse across operations.
///
/// AWS SDK clients are designed to be reused. This pool caches clients
/// by credentials and region to avoid recreating them for each operation.
#[derive(Clone)]
pub struct S3ClientPool {
    clients: Arc<RwLock<HashMap<ClientCacheKey, Client>>>,
}

impl S3ClientPool {
    /// Create a new empty client pool
    pub fn new() -> Self {
        S3ClientPool {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get an existing client or create a new one for the given credentials and region
    pub async fn get_or_create(&self, credentials: &Credentials, region: &str) -> Client {
        let key = ClientCacheKey {
            access_key_id: credentials.access_key_id().to_string(),
            region: region.to_string(),
        };

        // Try to get existing client (read lock)
        {
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(&key) {
                tracing::debug!(
                    "Reusing cached S3 client for region: {}, access_key: {}...",
                    region,
                    &key.access_key_id[..8.min(key.access_key_id.len())]
                );
                return client.clone();
            }
        }

        // Create new client (write lock)
        let mut clients = self.clients.write().await;

        // Double-check in case another task created it while we waited for write lock
        if let Some(client) = clients.get(&key) {
            return client.clone();
        }

        tracing::debug!(
            "Creating new S3 client for region: {}, access_key: {}...",
            region,
            &key.access_key_id[..8.min(key.access_key_id.len())]
        );

        // Create new client
        let region_provider = RegionProviderChain::first_try(Region::new(region.to_string()))
            .or_default_provider()
            .or_else(Region::new("eu-north-1"));
        let shared_config = aws_config::from_env()
            .credentials_provider(credentials.clone())
            .region(region_provider)
            .load()
            .await;
        let client = Client::new(&shared_config);

        clients.insert(key, client.clone());
        client
    }

    /// Clear all cached clients (useful when credentials are refreshed)
    #[allow(dead_code)] // Will be used for credential refresh feature
    pub async fn clear(&self) {
        let mut clients = self.clients.write().await;
        clients.clear();
    }

    /// Get the number of cached clients
    #[allow(dead_code)] // Used in tests
    pub async fn cached_count(&self) -> usize {
        self.clients.read().await.len()
    }
}

impl Default for S3ClientPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Handles interactions with the s3 services through AWS sdk
#[derive(Clone)]
pub struct S3DataFetcher {
    pub default_region: String,
    credentials: Credentials,
    client_pool: S3ClientPool,
    /// Optional transfer state store for resumable uploads/downloads
    transfer_state_store: Option<Arc<TransferStateStore>>,
    /// Original FileCredential for tracking resumable transfers
    file_credential: FileCredential,
}

struct ProgressTracker {
    bytes_written: u64,
    content_length: u64,
    progress_sender: Sender<UploadProgressItem>,
    uri: String,
}

impl ProgressTracker {
    fn track(&mut self, len: u64) {
        self.bytes_written += len;
        let progress = self.bytes_written as f64 / self.content_length as f64;
        let progress_item = UploadProgressItem {
            progress: progress * 100.0,
            uri: self.uri.clone(),
        };
        // Use try_send to avoid blocking transfers when channel is full
        let _ = self.progress_sender.try_send(progress_item);
    }
}

/// Handles the progress updates (copy of aws sdk s3 example)
#[pin_project::pin_project]
pub struct ProgressBody<InnerBody> {
    #[pin]
    inner: InnerBody,
    // progress_tracker is a separate field, so it can be accessed as &mut.
    progress_tracker: ProgressTracker,
}

// For an SdkBody specifically, the ProgressTracker swap itself in-place while customizing the SDK operation.
impl ProgressBody<SdkBody> {
    // Wrap a Requests SdkBody with a new ProgressBody, and replace it on the fly.
    // This is specialized for SdkBody specifically, as SdkBody provides ::taken() to
    // swap out the current body for a fresh, empty body and then provides ::from_dyn()
    // to get an SdkBody back from the ProgressBody it created. http::Body does not have
    // this "change the wheels on the fly" utility.
    pub fn replace(
        value: Request<SdkBody>,
        tx: Sender<UploadProgressItem>,
    ) -> Result<Request<SdkBody>, Infallible> {
        let uri = value.uri().to_string();
        let value = value.map(|body| {
            let len = body.content_length().expect("upload body sized");
            let cloned_uri = uri.clone();
            let body = ProgressBody::new(body, len, cloned_uri, tx.clone());
            SdkBody::from_body_0_4(body)
        });
        Ok(value)
    }
}

impl<InnerBody> ProgressBody<InnerBody>
    where
        InnerBody: Body<Data=Bytes, Error=aws_smithy_types::body::Error>,
{
    pub fn new(
        body: InnerBody,
        content_length: u64,
        uri: String,
        tx: Sender<UploadProgressItem>,
    ) -> Self {
        Self {
            inner: body,
            progress_tracker: ProgressTracker {
                bytes_written: 0,
                content_length,
                progress_sender: tx,
                uri: uri.to_string(),
            },
        }
    }
}

impl<InnerBody> Body for ProgressBody<InnerBody>
    where
        InnerBody: Body<Data=Bytes, Error=aws_smithy_types::body::Error>,
{
    type Data = Bytes;

    type Error = aws_smithy_types::body::Error;

    // Our poll_data delegates to the inner poll_data, but needs a project() to
    // get there. When the poll has data, it updates the progress_tracker.
    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        let this = self.project();
        match this.inner.poll_data(cx) {
            Poll::Ready(Some(Ok(data))) => {
                this.progress_tracker.track(data.len() as u64);
                Poll::Ready(Some(Ok(data)))
            }
            Poll::Ready(None) => {
                tracing::debug!("done");
                Poll::Ready(None)
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }

    // Delegate utilities to inner and progress_tracker.
    fn poll_trailers(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<http::HeaderMap>, Self::Error>> {
        self.project().inner.poll_trailers(cx)
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.progress_tracker.content_length)
    }
}
/*
- Handle buckets from different regions
- fix upload/download functions to handled dirs/buckets
- add create/delete buckets
- add copy files within a bucket
 */

impl S3DataFetcher {
    pub fn new(creds: FileCredential) -> Self {
        let access_key = creds.access_key.clone();
        let secret_access_key = creds.secret_key.clone();
        let default_region = creds.default_region.clone();
        let credentials = Credentials::new(
            access_key,
            secret_access_key,
            None,     // Token, if using temporary credentials (like STS)
            None,     // Expiry time, if applicable
            "manual", // Source, just a label for debugging
        );
        S3DataFetcher {
            default_region,
            credentials,
            client_pool: S3ClientPool::new(),
            transfer_state_store: None,
            file_credential: creds,
        }
    }

    /// Set the transfer state store for tracking resumable uploads/downloads
    pub fn with_transfer_state_store(mut self, store: Arc<TransferStateStore>) -> Self {
        self.transfer_state_store = Some(store);
        self
    }

    /// Check if a file should use multipart upload based on its size
    fn should_use_multipart(file_size: u64) -> bool {
        file_size >= MULTIPART_THRESHOLD
    }

    /// Calculate optimal part size for multipart upload
    ///
    /// Returns a part size that:
    /// - Is at least MIN_PART_SIZE (5MB)
    /// - Defaults to DEFAULT_PART_SIZE (8MB) for most files
    /// - Increases if needed to stay under MAX_PARTS (10,000)
    fn calculate_part_size(file_size: u64) -> u64 {
        // Start with default part size
        let mut part_size = DEFAULT_PART_SIZE;

        // Calculate how many parts we'd need
        let parts_needed = file_size.div_ceil(part_size);

        // If we'd exceed max parts, increase part size
        if parts_needed > MAX_PARTS {
            // Calculate minimum part size to stay under MAX_PARTS
            part_size = file_size.div_ceil(MAX_PARTS);
            // Round up to next MB for cleaner sizes
            part_size = part_size.div_ceil(1024 * 1024) * (1024 * 1024);
        }

        // Ensure we're at least at minimum
        part_size.max(MIN_PART_SIZE)
    }

    /// Upload a file, automatically using multipart upload for large files
    pub async fn upload_item(
        &self,
        item: LocalSelectedItem,
        upload_tx: Sender<UploadProgressItem>,
        pause_signal: Option<PauseSignal>,
    ) -> eyre::Result<bool> {
        // Get file size to determine upload method
        let file_metadata = tokio::fs::metadata(&item.path).await?;
        let file_size = file_metadata.len();

        if Self::should_use_multipart(file_size) {
            self.upload_multipart(item, upload_tx, file_size, pause_signal).await
        } else {
            self.upload_simple(item, upload_tx).await
        }
    }

    /// Simple single-request upload for small files
    async fn upload_simple(
        &self,
        item: LocalSelectedItem,
        upload_tx: Sender<UploadProgressItem>,
    ) -> eyre::Result<bool> {
        let client = self.get_s3_client(Some(item.s3_creds)).await;
        let body = ByteStream::read_from()
            .path(item.path)
            .build()
            .await?;
        let key = if item.destination_path == "/" {
            item.name
        } else {
            item.destination_path
        };

        let request = client
            .put_object()
            .bucket(item.destination_bucket)
            .key(key)
            .body(body);

        let customized = request
            .customize()
            .map_request(move |req| ProgressBody::<SdkBody>::replace(req, upload_tx.clone()));

        match customized.send().await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::error!("Upload SdkError: {:?}", e);
                Err(Report::msg(e.into_service_error().to_string()))
            }
        }
    }

    /// Multipart upload for large files
    ///
    /// This method:
    /// 1. Initiates a multipart upload
    /// 2. Uploads parts in sequence, reporting progress
    /// 3. Completes the upload with all part ETags
    /// 4. Aborts the upload on error
    ///
    /// If a TransferStateStore is configured, the upload state is persisted
    /// to allow resumption after failures or restarts.
    ///
    /// The pause_signal is checked between parts. If set to true, the upload
    /// will stop and return an error indicating it was paused.
    async fn upload_multipart(
        &self,
        item: LocalSelectedItem,
        upload_tx: Sender<UploadProgressItem>,
        file_size: u64,
        pause_signal: Option<PauseSignal>,
    ) -> eyre::Result<bool> {
        let client = self.get_s3_client(Some(item.s3_creds.clone())).await;
        let key = if item.destination_path == "/" {
            item.name.clone()
        } else {
            item.destination_path.clone()
        };
        let bucket = item.destination_bucket.clone();
        let part_size = Self::calculate_part_size(file_size);

        // Step 1: Initiate multipart upload
        let create_response = client
            .create_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to initiate multipart upload: {:?}", e);
                Report::msg(format!("Failed to initiate multipart upload: {}", e))
            })?;

        let upload_id = create_response
            .upload_id()
            .ok_or_else(|| Report::msg("No upload ID returned from create_multipart_upload"))?
            .to_string();

        // Register with transfer state store for resumability
        let transfer_id = if let Some(ref store) = self.transfer_state_store {
            match store
                .add_upload(
                    upload_id.clone(),
                    item.path.clone(),
                    bucket.clone(),
                    key.clone(),
                    file_size,
                    part_size,
                    &self.file_credential,
                )
                .await
            {
                Ok(id) => Some(id),
                Err(e) => {
                    tracing::warn!("Failed to register resumable upload: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Step 2: Upload parts
        let mut completed_parts: Vec<CompletedPart> = Vec::new();
        let mut file = tokio::fs::File::open(&item.path).await?;
        let mut part_number: i32 = 1;
        let mut bytes_uploaded: u64 = 0;
        let total_parts = file_size.div_ceil(part_size);

        loop {
            // Check if we should pause before starting the next part
            if let Some(ref signal) = pause_signal {
                if signal.load(Ordering::SeqCst) {
                    return Err(Report::msg("Upload paused by user"));
                }
            }

            // Read a full part's worth of data (or remaining bytes for last part)
            // Using a loop to fill the buffer because async read() can return partial data
            let mut buffer = vec![0u8; part_size as usize];
            let mut total_bytes_read = 0;

            while total_bytes_read < part_size as usize {
                match file.read(&mut buffer[total_bytes_read..]).await? {
                    0 => break, // EOF
                    n => total_bytes_read += n,
                }
            }

            if total_bytes_read == 0 {
                break; // End of file
            }

            // Trim buffer to actual bytes read
            buffer.truncate(total_bytes_read);
            let bytes_read = total_bytes_read;

            tracing::info!(
                "Uploading part {}/{}: {} bytes",
                part_number,
                total_parts,
                bytes_read
            );

            // Upload this part with retry logic
            let mut last_error: Option<String> = None;

            for attempt in 1..=MAX_PART_RETRIES {
                // Clone buffer for each attempt (needed because ByteStream consumes it)
                let part_body = ByteStream::from(buffer.clone());

                match client
                    .upload_part()
                    .bucket(&bucket)
                    .key(&key)
                    .upload_id(&upload_id)
                    .part_number(part_number)
                    .body(part_body)
                    .send()
                    .await
                {
                    Ok(response) => {
                        let e_tag = response.e_tag().unwrap_or("").to_string();

                        tracing::info!(
                            "Part {}/{} uploaded successfully ({}MB, attempt {})",
                            part_number,
                            total_parts,
                            bytes_read / (1024 * 1024),
                            attempt
                        );

                        completed_parts.push(
                            CompletedPart::builder()
                                .part_number(part_number)
                                .e_tag(e_tag.clone())
                                .build(),
                        );

                        // Persist completed part to transfer state store
                        if let (Some(ref store), Some(ref tid)) =
                            (&self.transfer_state_store, &transfer_id)
                        {
                            if let Err(e) =
                                store.update_upload_part(tid, part_number, e_tag).await
                            {
                                tracing::warn!("Failed to persist part {} progress: {}", part_number, e);
                            }
                        }

                        // Update progress
                        bytes_uploaded += bytes_read as u64;
                        let progress = (bytes_uploaded as f64 / file_size as f64) * 100.0;
                        let progress_item = UploadProgressItem {
                            progress,
                            uri: key.clone(),
                        };
                        let _ = upload_tx.try_send(progress_item);

                        last_error = None;
                        break;
                    }
                    Err(e) => {
                        let error_msg = format!("{}", e);
                        last_error = Some(error_msg.clone());

                        if attempt < MAX_PART_RETRIES {
                            tracing::warn!(
                                "Failed to upload part {} (attempt {}/{}): {}. Retrying in {}ms...",
                                part_number,
                                attempt,
                                MAX_PART_RETRIES,
                                error_msg,
                                RETRY_DELAY_MS
                            );
                            tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS))
                                .await;
                        } else {
                            tracing::error!(
                                "Failed to upload part {} after {} attempts: {}",
                                part_number,
                                MAX_PART_RETRIES,
                                error_msg
                            );
                        }
                    }
                }
            }

            // If all retries failed, keep state for resumption (don't abort)
            if let Some(error) = last_error {
                tracing::error!(
                    "Multipart upload failed at part {} - state preserved for resumption",
                    part_number
                );

                // Note: We intentionally do NOT abort the upload here,
                // allowing it to be resumed later. The upload will expire
                // after S3's default timeout (typically 7 days) if not resumed.

                return Err(Report::msg(format!(
                    "Failed to upload part {} after {} retries: {}. Upload can be resumed.",
                    part_number, MAX_PART_RETRIES, error
                )));
            }

            part_number += 1;
        }

        // Step 3: Complete multipart upload
        tracing::info!(
            "All {} parts uploaded, completing multipart upload...",
            part_number - 1
        );

        let completed_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        match client
            .complete_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .upload_id(&upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "Multipart upload completed successfully: {} ({} parts, {} bytes)",
                    key,
                    part_number - 1,
                    file_size
                );

                // Remove from transfer state store on success
                if let (Some(ref store), Some(ref tid)) =
                    (&self.transfer_state_store, &transfer_id)
                {
                    if let Err(e) = store.complete_upload(tid).await {
                        tracing::warn!("Failed to clear completed upload from state: {}", e);
                    }
                }

                Ok(true)
            }
            Err(e) => {
                tracing::error!("Failed to complete multipart upload: {:?}", e);

                // Try to abort on failure (completion failed, not worth keeping state)
                let _ = client
                    .abort_multipart_upload()
                    .bucket(&bucket)
                    .key(&key)
                    .upload_id(&upload_id)
                    .send()
                    .await;

                // Remove from transfer state store since we aborted
                if let (Some(ref store), Some(ref tid)) =
                    (&self.transfer_state_store, &transfer_id)
                {
                    let _ = store.remove_upload(tid).await;
                }

                Err(Report::msg(format!(
                    "Failed to complete multipart upload: {}",
                    e
                )))
            }
        }
    }

    fn create_directory_structure(&self, full_path: &Path) -> eyre::Result<()> {
        // Extract the directory path
        if let Some(parent_dir) = full_path.parent() {
            // Create the directory structure
            fs::create_dir_all(parent_dir)?;
        }

        Ok(())
    }

    /// Download a file from S3
    ///
    /// This method supports resumable downloads:
    /// - If a partial file exists, it resumes from where it left off using Range headers
    /// - Progress is periodically saved to TransferStateStore for crash recovery
    /// - On completion, the download is removed from the state store
    ///
    /// The pause_signal is checked during download. If set to true, the download
    /// will stop and return an error indicating it was paused.
    pub async fn download_item(
        &self,
        item: S3SelectedItem,
        download_tx: Sender<DownloadProgressItem>,
        pause_signal: Option<PauseSignal>,
    ) -> eyre::Result<bool> {
        let client = self.get_s3_client(Some(item.s3_creds.clone())).await;
        let mut path = PathBuf::from(&item.destination_dir);
        path.push(item.path.clone().unwrap_or(item.name.clone()));
        self.create_directory_structure(&path)?;

        let bucket = item.bucket.clone().expect("bucket must be defined");
        let key = item.path.clone().unwrap_or(item.name.clone());

        // Get total file size
        let head_obj = client
            .head_object()
            .bucket(&bucket)
            .key(&key)
            .send()
            .await?;
        let total_size = head_obj.content_length.unwrap_or(0i64) as u64;

        // Check if there's an existing partial download
        let bytes_already_downloaded = if path.exists() {
            let metadata = tokio::fs::metadata(&path).await?;
            metadata.len()
        } else {
            0
        };

        // Register download with state store for resumability
        let transfer_id = if let Some(ref store) = self.transfer_state_store {
            match store
                .add_download(
                    bucket.clone(),
                    key.clone(),
                    path.to_string_lossy().to_string(),
                    total_size,
                    &self.file_credential,
                )
                .await
            {
                Ok(id) => {
                    tracing::debug!("Registered resumable download with ID: {}", id);
                    Some(id)
                }
                Err(e) => {
                    tracing::warn!("Failed to register resumable download: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Open file for writing (append if resuming)
        let mut file = if bytes_already_downloaded > 0 && bytes_already_downloaded < total_size {
            tracing::info!(
                "Resuming download from {} bytes ({}% complete)",
                bytes_already_downloaded,
                (bytes_already_downloaded as f64 / total_size as f64 * 100.0) as u32
            );

            // Open in append mode for resuming
            std::fs::OpenOptions::new()
                .append(true)
                .open(&path)?
        } else {
            // Start fresh
            File::create(&path)?
        };

        // Build request with Range header if resuming
        let start_byte = if bytes_already_downloaded > 0 && bytes_already_downloaded < total_size {
            bytes_already_downloaded
        } else {
            0
        };

        let get_request = if start_byte > 0 {
            client
                .get_object()
                .bucket(&bucket)
                .key(&key)
                .range(format!("bytes={}-", start_byte))
        } else {
            client.get_object().bucket(&bucket).key(&key)
        };

        match get_request.send().await {
            Ok(mut object) => {
                let mut byte_count = start_byte as usize;
                let mut last_persist_byte = byte_count as u64;

                tracing::debug!(
                    "Starting download: name={}, bucket={}, start_byte={}, total_bytes={}",
                    key,
                    bucket,
                    start_byte,
                    total_size
                );

                while let Some(bytes) = object.body.try_next().await? {
                    // Check if we should pause
                    if let Some(ref signal) = pause_signal {
                        if signal.load(Ordering::SeqCst) {
                            tracing::info!(
                                "Download paused at {} bytes - state preserved for resumption",
                                byte_count
                            );
                            return Err(Report::msg("Download paused by user"));
                        }
                    }

                    let bytes_len = bytes.len();
                    file.write_all(&bytes)?;
                    byte_count += bytes_len;

                    // Periodically persist progress
                    if let (Some(ref store), Some(ref tid)) =
                        (&self.transfer_state_store, &transfer_id)
                    {
                        if byte_count as u64 - last_persist_byte >= DOWNLOAD_PROGRESS_PERSIST_INTERVAL
                        {
                            if let Err(e) = store
                                .update_download_progress(tid, byte_count as u64)
                                .await
                            {
                                tracing::warn!("Failed to persist download progress: {}", e);
                            }
                            last_persist_byte = byte_count as u64;
                        }
                    }

                    let progress =
                        Self::calculate_download_percentage(total_size as i64, byte_count);
                    tracing::debug!(
                        "Download chunk: bytes={}, total_bytes={}, progress={}",
                        bytes_len,
                        byte_count,
                        progress
                    );

                    let download_progress_item = DownloadProgressItem {
                        name: key.clone(),
                        bucket: bucket.clone(),
                        progress,
                    };
                    // Use try_send to avoid blocking downloads when channel is full
                    let _ = download_tx.try_send(download_progress_item);
                }

                // Remove from state store on success
                if let (Some(ref store), Some(ref tid)) =
                    (&self.transfer_state_store, &transfer_id)
                {
                    if let Err(e) = store.complete_download(tid).await {
                        tracing::warn!("Failed to clear completed download from state: {}", e);
                    }
                }

                tracing::info!("Download completed: {} ({} bytes)", key, byte_count);
                Ok(true)
            }
            Err(e) => {
                tracing::error!("Download SdkError: {:?}", e);
                // Keep state for resumption on error (don't remove from store)
                Err(Report::msg(e.into_service_error().to_string()))
            }
        }
    }

    fn calculate_download_percentage(total: i64, byte_count: usize) -> f64 {
        if total == 0 {
            0.0 // Return 0% if total is 0 to avoid division by zero
        } else {
            (byte_count as f64 / total as f64) * 100.0 // Cast to f64 to ensure floating-point division
        }
    }

    pub async fn list_current_location(
        &self,
        bucket: Option<String>,
        prefix: Option<String>,
    ) -> eyre::Result<Vec<S3DataItem>> {
        match (bucket, prefix) {
            (None, None) => self.list_buckets().await,
            (Some(bucket), None) => self.list_objects(bucket.as_str(), None).await,
            (Some(bucket), Some(prefix)) => self.list_objects(bucket.as_str(), Some(prefix)).await,
            _ => self.list_buckets().await,
        }
    }

    async fn get_bucket_location(&self, bucket: &str) -> eyre::Result<String> {
        let default_region = self.default_region.clone();
        let client = self.get_s3_client(None).await;
        let head_obj = client.get_bucket_location().bucket(bucket).send().await?;
        let location = head_obj
            .location_constraint()
            .map(|lc| lc.to_string())
            .unwrap_or_else(|| default_region.to_string());
        Ok(location)
    }

    // Example async method to fetch data from an external service
    async fn list_buckets(&self) -> eyre::Result<Vec<S3DataItem>> {
        let client = self.get_s3_client(None).await;
        let mut fetched_data: Vec<S3DataItem> = vec![];
        if let Ok(res) = client.list_buckets().send().await {
            fetched_data = res.buckets.as_ref().map_or_else(
                Vec::new, // In case there is no buckets field (it's None), return an empty Vec
                |buckets| {
                    buckets
                        .iter()
                        .filter_map(|bucket| {
                            // Filter out buckets where name is None, and map those with a name to a Vec<String>
                            bucket.name.as_ref().map(|name| {
                                let file_info = FileInfo {
                                    file_name: name.clone(),
                                    size: "".to_string(),
                                    file_type: "Bucket".to_string(),
                                    path: name.clone(),
                                    is_directory: false,
                                };
                                let bucket_info = BucketInfo {
                                    bucket: None,
                                    region: None,
                                    is_bucket: true,
                                };
                                S3DataItem::init(bucket_info, file_info)
                            })
                        })
                        .collect()
                },
            )
        }
        Ok(fetched_data)
    }

    pub async fn create_bucket(
        &self,
        name: String,
        region: String,
    ) -> eyre::Result<Option<String>> {
        let client = self.get_s3_client(None).await;
        let constraint = BucketLocationConstraint::from(region.as_str());
        let cfg = CreateBucketConfiguration::builder()
            .location_constraint(constraint)
            .build();
        match client
            .create_bucket()
            .create_bucket_configuration(cfg)
            .bucket(name.clone())
            .send()
            .await
        {
            Ok(_) => {
                tracing::info!("Bucket created");
                Ok(None)
            }
            Err(e) => {
                tracing::error!("Cannot create bucket");
                Ok(Some(
                    e.into_service_error()
                        .message()
                        .unwrap_or("Cannot create bucket")
                        .to_string(),
                ))
            }
        }
    }

    pub async fn delete_data(
        &self,
        is_bucket: bool,
        bucket: Option<String>,
        name: String,
        _is_directory: bool,
    ) -> eyre::Result<Option<String>> {
        if is_bucket {
            let location = self.get_bucket_location(&name).await?;
            let creds = self.credentials.clone();
            let temp_file_creds = FileCredential {
                name: "temp".to_string(),
                access_key: creds.access_key_id().to_string(),
                secret_key: creds.secret_access_key().to_string(),
                default_region: location.clone(),
                selected: false,
            };
            let client_with_location = self.get_s3_client(Some(temp_file_creds)).await;
            let response = client_with_location
                .delete_bucket()
                .bucket(name.clone())
                .send()
                .await;
            match response {
                Ok(_) => {
                    tracing::info!("bucket deleted: {}", name);
                    Ok(None)
                }
                Err(e) => {
                    tracing::error!("error deleting bucket: {}, {:?}", name, e);
                    Ok(Some(
                        e.into_service_error()
                            .message()
                            .unwrap_or("Error deleting bucket")
                            .to_string(),
                    ))
                }
            }
        } else {
            tracing::info!("Deleting object: {:?}, {:?}", name, bucket);
            match bucket {
                Some(b) => self.delete_single_item(&b, &name).await,
                None => Ok(Some("No bucket specified!".into())),
            }
        }
    }

    async fn delete_single_item(&self, bucket: &str, name: &str) -> eyre::Result<Option<String>> {
        let location = self.get_bucket_location(bucket).await?;
        let creds = self.credentials.clone();
        let temp_file_creds = FileCredential {
            name: "temp".to_string(),
            access_key: creds.access_key_id().to_string(),
            secret_key: creds.secret_access_key().to_string(),
            default_region: location.clone(),
            selected: false,
        };
        let client_with_location = self.get_s3_client(Some(temp_file_creds)).await;
        let response = client_with_location
            .delete_object()
            .key(name)
            .bucket(bucket)
            .send()
            .await;
        match response {
            Ok(_) => {
                tracing::info!("S3 Object deleted, bucket: {:?}, name: {:?}", bucket, name);
                Ok(None)
            }
            Err(e) => {
                tracing::error!(
                    "Cannot delete object, bucket: {:?}, name: {:?}, error: {:?}",
                    bucket,
                    name,
                    e
                );
                Ok(Some(format!(
                    "Cannot delete object, {:?}",
                    e.into_service_error().message().unwrap_or("")
                )))
            }
        }
    }

    /// Lists all object in the given bucket (or filtered by prefix) and constructs the items
    /// representing directories
    /// This method is used for displaying bucket/prefix content while browsing s3 and
    /// it's not fetching all the contents behind prefixes together
    async fn list_objects(
        &self,
        bucket: &str,
        prefix: Option<String>,
    ) -> eyre::Result<Vec<S3DataItem>> {
        let mut all_objects = Vec::new();
        let location = self.get_bucket_location(bucket).await?;
        let creds = self.credentials.clone();
        let temp_file_creds = FileCredential {
            name: "temp".to_string(),
            access_key: creds.access_key_id().to_string(),
            secret_key: creds.secret_access_key().to_string(),
            default_region: location.clone(),
            selected: false,
        };
        let client_with_location = self.get_s3_client(Some(temp_file_creds)).await;
        let mut response = client_with_location
            .list_objects_v2()
            .delimiter("/")
            .set_prefix(prefix)
            .bucket(bucket.to_owned())
            .into_paginator()
            .send();

        while let Some(result) = response.next().await {
            match result {
                Ok(output) => {
                    for object in output.contents() {
                        let key = object.key().unwrap_or_default();
                        //todo: get size of the file
                        let size = object
                            .size()
                            .map_or(String::new(), |value| value.to_string());
                        let path = Path::new(key);
                        let file_extension = path
                            .extension()
                            .and_then(|ext| ext.to_str()) // Convert the OsStr to a &str
                            .unwrap_or("");
                        let file_info = FileInfo {
                            file_name: Self::get_filename(key).unwrap_or_default(),
                            size,
                            file_type: file_extension.to_string(),
                            path: key.to_string(),
                            is_directory: false,
                        };
                        let bucket_info = BucketInfo {
                            bucket: Some(bucket.to_string()),
                            region: Some(location.clone()),
                            is_bucket: false,
                        };
                        all_objects.push(S3DataItem::init(bucket_info, file_info));
                    }
                    for object in output.common_prefixes() {
                        let key = object.prefix().unwrap_or_default();
                        if key != "/" {
                            let file_info = FileInfo {
                                file_name: Self::get_last_directory(key).unwrap_or_default(),
                                size: "".to_string(),
                                file_type: "Dir".to_string(),
                                path: key.to_string(),
                                is_directory: true,
                            };
                            let bucket_info = BucketInfo {
                                bucket: Some(bucket.to_string()),
                                region: Some(location.clone()),
                                is_bucket: false,
                            };
                            all_objects.push(S3DataItem::init(bucket_info, file_info));
                        }
                    }
                }
                Err(err) => {
                    tracing::error!("Err: {:?}", err) // Return the error immediately if encountered
                }
            }
        }

        Ok(all_objects)
    }

    fn get_last_directory(path: &str) -> Option<String> {
        let parts: Vec<&str> = path.split('/').collect();
        let parts: Vec<&str> = parts.into_iter().filter(|&part| !part.is_empty()).collect();
        parts.last().map(|&last| format!("{}/", last))
    }
    fn get_filename(path: &str) -> Option<String> {
        let parts: Vec<&str> = path.split('/').collect();
        let parts: Vec<&str> = parts.into_iter().filter(|&part| !part.is_empty()).collect();
        parts.last().and_then(|&last| {
            if path.ends_with('/') {
                None
            } else {
                Some(last.to_string())
            }
        })
    }

    /// This method is similar to `list_objects` but it fetches all the data recursively
    /// including data behind the prefixes.
    /// Designed to be used mainly when selecting whole bucket/prefix for download or delete.
    pub async fn list_all_objects(
        &self,
        bucket: &str,
        prefix: Option<String>,
    ) -> eyre::Result<Vec<S3DataItem>> {
        let mut all_objects = Vec::new();
        let location = self.get_bucket_location(bucket).await?;
        self.recursive_list_objects(bucket, prefix, &location, &mut all_objects)
            .await?;
        Ok(all_objects)
    }
    fn recursive_list_objects<'a>(
        &'a self,
        bucket: &'a str,
        prefix: Option<String>,
        location: &'a str,
        all_objects: &'a mut Vec<S3DataItem>,
    ) -> Pin<Box<dyn std::future::Future<Output=Result<(), Report>> + Send + 'a>> {
        Box::pin(async move {
            let creds = self.credentials.clone();
            let temp_file_creds = FileCredential {
                name: "temp".to_string(),
                access_key: creds.access_key_id().to_string(),
                secret_key: creds.secret_access_key().to_string(),
                default_region: location.to_string(),
                selected: false,
            };

            let client_with_location = self.get_s3_client(Some(temp_file_creds)).await;
            let mut response = client_with_location
                .list_objects_v2()
                .delimiter("/")
                .set_prefix(prefix.clone())
                .bucket(bucket.to_owned())
                .into_paginator()
                .send();

            while let Some(result) = response.next().await {
                match result {
                    Ok(output) => {
                        for object in output.contents() {
                            let key = object.key().unwrap_or_default();
                            let size = object
                                .size()
                                .map_or(String::new(), |value| value.to_string());
                            let path = Path::new(key);
                            let file_extension =
                                path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
                            let file_info = FileInfo {
                                file_name: Self::get_filename(key).unwrap_or_default(),
                                size,
                                file_type: file_extension.to_string(),
                                path: key.to_string(),
                                is_directory: false,
                            };
                            let bucket_info = BucketInfo {
                                bucket: Some(bucket.to_string()),
                                region: Some(location.to_string()),
                                is_bucket: false,
                            };
                            all_objects.push(S3DataItem::init(bucket_info, file_info));
                        }
                        for common_prefix in output.common_prefixes() {
                            let prefix = common_prefix.prefix().unwrap_or_default().to_string();
                            self.recursive_list_objects(
                                bucket,
                                Some(prefix),
                                location,
                                all_objects,
                            )
                                .await?;
                        }
                    }
                    Err(err) => {
                        tracing::error!("Err: {:?}", err); // Return the error immediately if encountered
                        return Err(err.into());
                    }
                }
            }
            Ok(())
        })
    }

    async fn get_s3_client(&self, creds: Option<FileCredential>) -> Client {
        let credentials: Credentials;
        let region: String;

        if let Some(crd) = creds {
            let access_key = crd.access_key;
            let secret_access_key = crd.secret_key;
            region = crd.default_region;
            credentials = Credentials::new(
                access_key,
                secret_access_key,
                None,     // Token, if using temporary credentials (like STS)
                None,     // Expiry time, if applicable
                "manual", // Source, just a label for debugging
            );
        } else {
            credentials = self.credentials.clone();
            region = self.default_region.clone();
        }

        // Use the client pool to get or create a client
        self.client_pool.get_or_create(&credentials, &region).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_pool_caches_clients() {
        let pool = S3ClientPool::new();
        let credentials = Credentials::new(
            "test_access_key",
            "test_secret_key",
            None,
            None,
            "test",
        );

        // First call should create a client
        let _client1 = pool.get_or_create(&credentials, "us-east-1").await;
        assert_eq!(pool.cached_count().await, 1);

        // Second call with same credentials and region should reuse
        let _client2 = pool.get_or_create(&credentials, "us-east-1").await;
        assert_eq!(pool.cached_count().await, 1);

        // Different region should create new client
        let _client3 = pool.get_or_create(&credentials, "eu-west-1").await;
        assert_eq!(pool.cached_count().await, 2);
    }

    #[tokio::test]
    async fn test_client_pool_different_credentials() {
        let pool = S3ClientPool::new();
        let creds1 = Credentials::new("key1", "secret1", None, None, "test");
        let creds2 = Credentials::new("key2", "secret2", None, None, "test");

        let _client1 = pool.get_or_create(&creds1, "us-east-1").await;
        let _client2 = pool.get_or_create(&creds2, "us-east-1").await;

        // Different credentials should create different clients
        assert_eq!(pool.cached_count().await, 2);
    }

    #[tokio::test]
    async fn test_client_pool_clear() {
        let pool = S3ClientPool::new();
        let credentials = Credentials::new("key", "secret", None, None, "test");

        let _client = pool.get_or_create(&credentials, "us-east-1").await;
        assert_eq!(pool.cached_count().await, 1);

        pool.clear().await;
        assert_eq!(pool.cached_count().await, 0);
    }

    #[test]
    fn test_should_use_multipart_below_threshold() {
        // 50 MB - should NOT use multipart
        let file_size = 50 * 1024 * 1024;
        assert!(!S3DataFetcher::should_use_multipart(file_size));
    }

    #[test]
    fn test_should_use_multipart_at_threshold() {
        // Exactly 100 MB - should use multipart
        let file_size = 100 * 1024 * 1024;
        assert!(S3DataFetcher::should_use_multipart(file_size));
    }

    #[test]
    fn test_should_use_multipart_above_threshold() {
        // 500 MB - should use multipart
        let file_size = 500 * 1024 * 1024;
        assert!(S3DataFetcher::should_use_multipart(file_size));
    }

    #[test]
    fn test_calculate_part_size_default() {
        // 200 MB file - should use default 8 MB parts
        let file_size = 200 * 1024 * 1024;
        let part_size = S3DataFetcher::calculate_part_size(file_size);
        assert_eq!(part_size, DEFAULT_PART_SIZE);
    }

    #[test]
    fn test_calculate_part_size_large_file() {
        // 100 GB file with 8MB parts would need 12,800 parts (> 10,000)
        // Should increase part size to stay under 10,000 parts
        let file_size: u64 = 100 * 1024 * 1024 * 1024; // 100 GB
        let part_size = S3DataFetcher::calculate_part_size(file_size);

        // Part size should be increased
        assert!(part_size > DEFAULT_PART_SIZE);

        // Should result in <= 10,000 parts
        let parts_needed = (file_size + part_size - 1) / part_size;
        assert!(parts_needed <= MAX_PARTS);
    }

    #[test]
    fn test_calculate_part_size_minimum() {
        // Even small files should have at least MIN_PART_SIZE
        let file_size = 10 * 1024 * 1024; // 10 MB
        let part_size = S3DataFetcher::calculate_part_size(file_size);
        assert!(part_size >= MIN_PART_SIZE);
    }

    #[test]
    fn test_calculate_part_size_very_large_file() {
        // 1 TB file - needs larger parts
        let file_size: u64 = 1024 * 1024 * 1024 * 1024; // 1 TB
        let part_size = S3DataFetcher::calculate_part_size(file_size);

        // Should result in <= 10,000 parts
        let parts_needed = (file_size + part_size - 1) / part_size;
        assert!(parts_needed <= MAX_PARTS);

        // Part size should be at least ~100 MB for 1 TB file
        assert!(part_size >= 100 * 1024 * 1024);
    }
}
