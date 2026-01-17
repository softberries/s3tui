//! Task registry for tracking and managing spawned Tokio tasks
//!
//! This module provides a registry for tracking background tasks,
//! allowing them to be cancelled and monitored.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::task::{AbortHandle, JoinHandle};

/// Unique identifier for a tracked task
pub type TaskId = u64;

/// Information about an active task
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used in tests and for future task monitoring UI
pub struct TaskInfo {
    /// Unique task identifier
    pub id: TaskId,
    /// Human-readable name for the task
    pub name: String,
    /// When the task was spawned
    pub created_at: Instant,
}

/// Internal tracking entry for a task
#[allow(dead_code)] // Fields used internally for task management
struct TaskEntry {
    info: TaskInfo,
    abort_handle: AbortHandle,
}

/// Registry for tracking spawned tasks
///
/// Provides spawn_tracked, cancel, and listing functionality
/// for background tasks.
pub struct TaskRegistry {
    /// Counter for generating unique task IDs
    next_id: AtomicU64,
    /// Map of active tasks
    tasks: Arc<Mutex<HashMap<TaskId, TaskEntry>>>,
}

impl TaskRegistry {
    /// Create a new task registry
    pub fn new() -> Self {
        TaskRegistry {
            next_id: AtomicU64::new(1),
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a unique task ID
    fn generate_id(&self) -> TaskId {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Spawn a tracked task and return just the ID (fire-and-forget style)
    ///
    /// Useful when you don't need the handle immediately but want tracking.
    pub async fn spawn_tracked<F>(&self, name: impl Into<String>, future: F) -> TaskId
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let id = self.generate_id();
        let name = name.into();
        let tasks = self.tasks.clone();
        let tasks_cleanup = self.tasks.clone();

        let join_handle: JoinHandle<()> = tokio::spawn(async move {
            future.await;
            // Remove from registry when done
            tasks_cleanup.lock().await.remove(&id);
        });

        let abort_handle = join_handle.abort_handle();

        let entry = TaskEntry {
            info: TaskInfo {
                id,
                name,
                created_at: Instant::now(),
            },
            abort_handle,
        };

        tasks.lock().await.insert(id, entry);
        id
    }

    /// Cancel a task by its ID
    ///
    /// Returns true if the task was found and cancelled, false otherwise.
    #[allow(dead_code)] // Used in tests and for future task cancellation UI
    pub async fn cancel(&self, task_id: TaskId) -> bool {
        let mut tasks = self.tasks.lock().await;
        if let Some(entry) = tasks.remove(&task_id) {
            entry.abort_handle.abort();
            true
        } else {
            false
        }
    }

    /// Get information about all active tasks
    #[allow(dead_code)] // Used in tests and for future task monitoring UI
    pub async fn get_active_tasks(&self) -> Vec<TaskInfo> {
        let tasks = self.tasks.lock().await;
        tasks.values().map(|e| e.info.clone()).collect()
    }

    /// Get the number of active tasks
    #[allow(dead_code)] // Used in tests and for future task monitoring UI
    pub async fn active_count(&self) -> usize {
        self.tasks.lock().await.len()
    }

    /// Check if a specific task is still active
    #[allow(dead_code)] // Used in tests and for future task monitoring UI
    pub async fn is_active(&self, task_id: TaskId) -> bool {
        self.tasks.lock().await.contains_key(&task_id)
    }

    /// Cancel all active tasks
    #[allow(dead_code)] // Used in tests and for graceful shutdown
    pub async fn cancel_all(&self) {
        let mut tasks = self.tasks.lock().await;
        for entry in tasks.values() {
            entry.abort_handle.abort();
        }
        tasks.clear();
    }

    /// Clean up finished tasks from the registry
    ///
    /// This is called automatically when tasks complete, but can be
    /// called manually to ensure cleanup.
    #[allow(dead_code)] // Used in tests and for manual cleanup
    pub async fn cleanup_finished(&self) {
        let mut tasks = self.tasks.lock().await;
        tasks.retain(|_, entry| !entry.abort_handle.is_finished());
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_spawn_tracked_task() {
        let registry = TaskRegistry::new();

        let task_id = registry
            .spawn_tracked("test-task", async {
                sleep(Duration::from_millis(50)).await;
            })
            .await;

        assert!(task_id > 0);
        assert!(registry.is_active(task_id).await);

        // Wait for task to complete
        sleep(Duration::from_millis(100)).await;
        registry.cleanup_finished().await;

        assert!(!registry.is_active(task_id).await);
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let registry = TaskRegistry::new();
        let was_cancelled = Arc::new(AtomicBool::new(false));
        let was_cancelled_clone = was_cancelled.clone();

        let task_id = registry
            .spawn_tracked("long-task", async move {
                sleep(Duration::from_secs(10)).await;
                was_cancelled_clone.store(true, Ordering::SeqCst);
            })
            .await;

        // Task should be active
        assert!(registry.is_active(task_id).await);

        // Cancel the task
        let cancelled = registry.cancel(task_id).await;
        assert!(cancelled);

        // Give it a moment to process
        sleep(Duration::from_millis(10)).await;

        // Task should no longer be active
        assert!(!registry.is_active(task_id).await);

        // The task body should not have completed
        assert!(!was_cancelled.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_get_active_tasks() {
        let registry = TaskRegistry::new();

        let _id1 = registry
            .spawn_tracked("task-1", async {
                sleep(Duration::from_secs(10)).await;
            })
            .await;

        let _id2 = registry
            .spawn_tracked("task-2", async {
                sleep(Duration::from_secs(10)).await;
            })
            .await;

        let active = registry.get_active_tasks().await;
        assert_eq!(active.len(), 2);

        let names: Vec<_> = active.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"task-1"));
        assert!(names.contains(&"task-2"));

        // Cleanup
        registry.cancel_all().await;
    }

    #[tokio::test]
    async fn test_cancel_all() {
        let registry = TaskRegistry::new();

        for i in 0..5 {
            registry
                .spawn_tracked(format!("task-{}", i), async {
                    sleep(Duration::from_secs(10)).await;
                })
                .await;
        }

        assert_eq!(registry.active_count().await, 5);

        registry.cancel_all().await;

        assert_eq!(registry.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_task_registry_cleanup() {
        let registry = TaskRegistry::new();

        // Spawn a quick task
        let task_id = registry
            .spawn_tracked("quick-task", async {
                sleep(Duration::from_millis(10)).await;
            })
            .await;

        assert!(registry.is_active(task_id).await);

        // Wait for it to complete
        sleep(Duration::from_millis(50)).await;

        // The task should auto-cleanup, but let's verify
        registry.cleanup_finished().await;
        assert!(!registry.is_active(task_id).await);
        assert_eq!(registry.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_task_id_generation() {
        let registry = TaskRegistry::new();

        let id1 = registry
            .spawn_tracked("task-1", async {})
            .await;
        let id2 = registry
            .spawn_tracked("task-2", async {})
            .await;
        let id3 = registry
            .spawn_tracked("task-3", async {})
            .await;

        assert!(id1 < id2);
        assert!(id2 < id3);
    }
}
