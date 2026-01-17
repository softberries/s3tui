//! Transfer Queue Manager
//!
//! Provides centralized management for file transfers with:
//! - Queue management with FIFO ordering
//! - Configurable concurrency limits
//! - Pause/resume/cancel functionality
//! - Priority adjustment

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

/// Unique identifier for a transfer job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(u64);

impl JobId {
    fn new(id: u64) -> Self {
        JobId(id)
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Job-{}", self.0)
    }
}

/// Status of a transfer job
#[derive(Debug, Clone, PartialEq)]
pub enum TransferStatus {
    /// Waiting in queue
    Queued,
    /// Currently transferring
    Active { progress: f64 },
    /// Paused by user
    Paused { progress: f64 },
    /// Successfully completed
    Completed,
    /// Failed with error
    Failed { error: String },
    /// Cancelled by user
    Cancelled,
}


/// Represents a single transfer job
#[derive(Debug, Clone)]
pub struct TransferJob {
    /// Unique job identifier
    pub id: JobId,
    /// Current status
    pub status: TransferStatus,
    /// Priority (higher = more urgent, default is 0)
    pub priority: i32,
}

impl TransferJob {
    /// Create a new job
    fn new(id: JobId) -> Self {
        TransferJob {
            id,
            status: TransferStatus::Queued,
            priority: 0,
        }
    }
}

/// Priority queue for transfer jobs
#[derive(Debug)]
struct TransferQueue {
    jobs: VecDeque<TransferJob>,
}

impl TransferQueue {
    fn new() -> Self {
        TransferQueue {
            jobs: VecDeque::new(),
        }
    }

    /// Add a job to the queue (maintains priority ordering)
    fn enqueue(&mut self, job: TransferJob) {
        // Find insertion point based on priority (higher priority first)
        let pos = self
            .jobs
            .iter()
            .position(|j| j.priority < job.priority)
            .unwrap_or(self.jobs.len());
        self.jobs.insert(pos, job);
    }

    /// Remove and return the next job to process
    fn dequeue(&mut self) -> Option<TransferJob> {
        self.jobs.pop_front()
    }

    /// Get a job by ID without removing it
    fn get(&self, job_id: JobId) -> Option<&TransferJob> {
        self.jobs.iter().find(|j| j.id == job_id)
    }

    /// Remove a job from the queue
    fn remove(&mut self, job_id: JobId) -> Option<TransferJob> {
        if let Some(pos) = self.jobs.iter().position(|j| j.id == job_id) {
            self.jobs.remove(pos)
        } else {
            None
        }
    }

    /// Move a job to the front of the queue
    fn prioritize(&mut self, job_id: JobId) {
        if let Some(pos) = self.jobs.iter().position(|j| j.id == job_id) {
            if let Some(job) = self.jobs.remove(pos) {
                self.jobs.push_front(job);
            }
        }
    }

}

/// Central coordinator for all transfers
pub struct TransferManager {
    /// Counter for generating unique job IDs
    next_job_id: AtomicU64,
    /// Pending jobs queue
    pending: Arc<Mutex<TransferQueue>>,
    /// Active jobs (currently transferring)
    active: Arc<Mutex<Vec<TransferJob>>>,
    /// Paused jobs
    paused: Arc<Mutex<Vec<TransferJob>>>,
    /// Completed/failed/cancelled jobs history
    history: Arc<Mutex<Vec<TransferJob>>>,
    /// Semaphore for concurrency control
    semaphore: Arc<Semaphore>,
}

impl TransferManager {
    /// Create a new transfer manager with specified concurrency limit
    pub fn new(concurrency: usize) -> Self {
        TransferManager {
            next_job_id: AtomicU64::new(1),
            pending: Arc::new(Mutex::new(TransferQueue::new())),
            active: Arc::new(Mutex::new(Vec::new())),
            paused: Arc::new(Mutex::new(Vec::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(concurrency)),
        }
    }

    /// Generate a new unique job ID
    fn generate_job_id(&self) -> JobId {
        JobId::new(self.next_job_id.fetch_add(1, Ordering::SeqCst))
    }

    /// Add a new upload job to the queue
    pub async fn enqueue_upload(&self, _local_path: String, _s3_path: String, _size: Option<u64>) -> JobId {
        let job_id = self.generate_job_id();
        let job = TransferJob::new(job_id);
        self.pending.lock().await.enqueue(job);
        job_id
    }

    /// Add a new download job to the queue
    pub async fn enqueue_download(&self, _s3_path: String, _local_path: String, _size: Option<u64>) -> JobId {
        let job_id = self.generate_job_id();
        let job = TransferJob::new(job_id);
        self.pending.lock().await.enqueue(job);
        job_id
    }

    /// Pause a transfer (move from active to paused)
    pub async fn pause(&self, job_id: JobId) -> Result<(), String> {
        let mut active = self.active.lock().await;
        if let Some(pos) = active.iter().position(|j| j.id == job_id) {
            let mut job = active.remove(pos);
            if let TransferStatus::Active { progress } = job.status {
                job.status = TransferStatus::Paused { progress };
            }
            self.paused.lock().await.push(job);
            // Release the concurrency slot so other jobs can run
            drop(active);
            self.release_slot();
            Ok(())
        } else {
            Err(format!("Job {} is not active", job_id))
        }
    }

    /// Resume a paused transfer (move from paused back to pending queue front)
    pub async fn resume(&self, job_id: JobId) -> Result<(), String> {
        let mut paused = self.paused.lock().await;
        if let Some(pos) = paused.iter().position(|j| j.id == job_id) {
            let mut job = paused.remove(pos);
            if let TransferStatus::Paused { progress } = job.status {
                job.status = TransferStatus::Active { progress };
            } else {
                job.status = TransferStatus::Queued;
            }
            // Add to front of queue for immediate processing
            let mut pending = self.pending.lock().await;
            pending.prioritize(job_id);
            if pending.get(job_id).is_none() {
                job.priority = i32::MAX; // Highest priority
                pending.enqueue(job);
            }
            Ok(())
        } else {
            Err(format!("Job {} is not paused", job_id))
        }
    }

    /// Cancel a transfer (remove from any queue and mark as cancelled)
    pub async fn cancel(&self, job_id: JobId) -> Result<(), String> {
        // Try to remove from pending (no slot to release - wasn't started)
        {
            let mut pending = self.pending.lock().await;
            if let Some(mut job) = pending.remove(job_id) {
                job.status = TransferStatus::Cancelled;
                self.history.lock().await.push(job);
                return Ok(());
            }
        }

        // Try to remove from paused (no slot to release - was paused)
        {
            let mut paused = self.paused.lock().await;
            if let Some(pos) = paused.iter().position(|j| j.id == job_id) {
                let mut job = paused.remove(pos);
                job.status = TransferStatus::Cancelled;
                self.history.lock().await.push(job);
                return Ok(());
            }
        }

        // Try to remove from active (need to release slot)
        {
            let mut active = self.active.lock().await;
            if let Some(pos) = active.iter().position(|j| j.id == job_id) {
                let mut job = active.remove(pos);
                job.status = TransferStatus::Cancelled;
                self.history.lock().await.push(job);
                drop(active); // Release lock before adding permit
                self.release_slot();
                return Ok(());
            }
        }

        Err(format!("Job {} not found", job_id))
    }

    /// Mark a job as completed
    pub async fn mark_completed(&self, job_id: JobId) {
        let mut active = self.active.lock().await;
        if let Some(pos) = active.iter().position(|j| j.id == job_id) {
            let mut job = active.remove(pos);
            job.status = TransferStatus::Completed;
            self.history.lock().await.push(job);
            // Release the concurrency slot
            drop(active); // Release lock before adding permit
            self.release_slot();
        }
    }

    /// Mark a job as failed
    pub async fn mark_failed(&self, job_id: JobId, error: String) {
        let mut active = self.active.lock().await;
        if let Some(pos) = active.iter().position(|j| j.id == job_id) {
            let mut job = active.remove(pos);
            job.status = TransferStatus::Failed { error };
            self.history.lock().await.push(job);
            // Release the concurrency slot
            drop(active); // Release lock before adding permit
            self.release_slot();
        }
    }

    /// Get the next job to process (if concurrency allows)
    /// Returns None if no jobs are pending or concurrency limit is reached
    pub async fn try_get_next(&self) -> Option<TransferJob> {
        // Check if we can acquire a permit (don't actually acquire yet)
        let available = self.semaphore.available_permits();
        if available == 0 {
            return None;
        }

        // Dequeue job while holding pending lock, then release it
        let job = {
            let mut pending = self.pending.lock().await;
            pending.dequeue()
        };

        if let Some(mut job) = job {
            // Now acquire the permit (we know one is available)
            let _permit = self.semaphore.try_acquire().ok()?;
            // Forget the permit - it will be released when mark_completed/mark_failed is called
            std::mem::forget(_permit);

            job.status = TransferStatus::Active { progress: 0.0 };
            self.active.lock().await.push(job.clone());
            Some(job)
        } else {
            None
        }
    }

    /// Release a concurrency slot (called when transfer completes/fails/cancels)
    pub fn release_slot(&self) {
        self.semaphore.add_permits(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_id_generation() {
        let manager = TransferManager::new(4);
        let id1 = manager.enqueue_upload("local1".into(), "s3/path1".into(), Some(100)).await;
        let id2 = manager.enqueue_upload("local2".into(), "s3/path2".into(), Some(200)).await;
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_transfer_queue_ordering() {
        let manager = TransferManager::new(4);

        // Enqueue jobs
        let id1 = manager.enqueue_upload("file1".into(), "s3/file1".into(), None).await;
        let id2 = manager.enqueue_upload("file2".into(), "s3/file2".into(), None).await;
        let id3 = manager.enqueue_upload("file3".into(), "s3/file3".into(), None).await;

        // Should come out in FIFO order
        let job1 = manager.try_get_next().await.unwrap();
        assert_eq!(job1.id, id1);

        let job2 = manager.try_get_next().await.unwrap();
        assert_eq!(job2.id, id2);

        let job3 = manager.try_get_next().await.unwrap();
        assert_eq!(job3.id, id3);
    }

    #[tokio::test]
    async fn test_transfer_pause_resume() {
        let manager = TransferManager::new(4);

        let job_id = manager.enqueue_upload("file".into(), "s3/file".into(), None).await;

        // Start the job
        let job = manager.try_get_next().await.unwrap();
        assert_eq!(job.id, job_id);

        // Pause
        manager.pause(job_id).await.unwrap();

        // Resume
        manager.resume(job_id).await.unwrap();

        // Job should be back in pending queue - can get it again
        let job = manager.try_get_next().await.unwrap();
        assert_eq!(job.id, job_id);
    }

    #[tokio::test]
    async fn test_transfer_cancel() {
        let manager = TransferManager::new(4);

        let job_id = manager.enqueue_upload("file".into(), "s3/file".into(), None).await;

        // Cancel from pending
        manager.cancel(job_id).await.unwrap();

        // Job is no longer available
        assert!(manager.try_get_next().await.is_none());
    }

    #[tokio::test]
    async fn test_concurrent_transfer_limit() {
        let manager = TransferManager::new(2); // Only 2 concurrent

        // Enqueue 4 jobs
        manager.enqueue_upload("file1".into(), "s3/file1".into(), None).await;
        manager.enqueue_upload("file2".into(), "s3/file2".into(), None).await;
        manager.enqueue_upload("file3".into(), "s3/file3".into(), None).await;
        manager.enqueue_upload("file4".into(), "s3/file4".into(), None).await;

        // Should only get 2 jobs (concurrency limit)
        assert!(manager.try_get_next().await.is_some());
        assert!(manager.try_get_next().await.is_some());
        assert!(manager.try_get_next().await.is_none()); // No more permits
    }

    #[tokio::test]
    async fn test_mark_completed() {
        let manager = TransferManager::new(4);

        let job_id = manager.enqueue_upload("file".into(), "s3/file".into(), None).await;
        manager.try_get_next().await;

        manager.mark_completed(job_id).await;

        // After completion, slot is released - can process another job
        manager.enqueue_upload("file2".into(), "s3/file2".into(), None).await;
        assert!(manager.try_get_next().await.is_some());
    }

    #[tokio::test]
    async fn test_mark_failed() {
        let manager = TransferManager::new(4);

        let job_id = manager.enqueue_upload("file".into(), "s3/file".into(), None).await;
        manager.try_get_next().await;

        manager.mark_failed(job_id, "Network error".into()).await;

        // After failure, slot is released - can process another job
        manager.enqueue_upload("file2".into(), "s3/file2".into(), None).await;
        assert!(manager.try_get_next().await.is_some());
    }
}
