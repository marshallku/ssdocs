use crate::types::Frontmatter;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

/// Results from parallel build operations
pub enum BuildResult {
    Success {
        path: PathBuf,
        slug: String,
        category: String,
        frontmatter: Frontmatter,
        file_hash: String,
        template_hash: String,
        output_path: String,
    },
    Skipped {
        path: PathBuf,
        reason: SkipReason,
    },
    Error {
        path: PathBuf,
        error: String,
    },
}

#[derive(Debug, Clone)]
pub enum SkipReason {
    Cached,
    Draft,
}

/// Progress tracking for parallel builds
pub struct BuildProgress {
    built: AtomicUsize,
    skipped: AtomicUsize,
}

impl BuildProgress {
    pub fn new() -> Self {
        Self {
            built: AtomicUsize::new(0),
            skipped: AtomicUsize::new(0),
        }
    }

    pub fn increment_built(&self) {
        self.built.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_skipped(&self) {
        self.skipped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_built(&self) -> usize {
        self.built.load(Ordering::Relaxed)
    }

    pub fn get_skipped(&self) -> usize {
        self.skipped.load(Ordering::Relaxed)
    }
}

impl Default for BuildProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Get optimal number of worker threads
pub fn get_thread_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Channel-based work queue for distributing tasks to workers
pub struct WorkQueue<T> {
    sender: mpsc::Sender<T>,
    receiver: Arc<Mutex<mpsc::Receiver<T>>>,
}

impl<T: Send + 'static> WorkQueue<T> {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn send(&self, item: T) -> Result<(), mpsc::SendError<T>> {
        self.sender.send(item)
    }

    pub fn get_receiver(&self) -> Arc<Mutex<mpsc::Receiver<T>>> {
        Arc::clone(&self.receiver)
    }

    pub fn close(self) {
        drop(self.sender);
    }
}

/// Simple worker pool for parallel processing
pub struct WorkerPool {
    handles: Vec<JoinHandle<()>>,
}

impl WorkerPool {
    pub fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }

    pub fn spawn<F>(&mut self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let handle = thread::spawn(f);
        self.handles.push(handle);
    }

    pub fn join(self) -> Result<(), String> {
        for handle in self.handles {
            handle
                .join()
                .map_err(|_| "Worker thread panicked".to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_thread_count() {
        let count = get_thread_count();
        assert!(count >= 1);
    }

    #[test]
    fn test_build_progress() {
        let progress = BuildProgress::new();
        progress.increment_built();
        progress.increment_built();
        progress.increment_skipped();

        assert_eq!(progress.get_built(), 2);
        assert_eq!(progress.get_skipped(), 1);
    }

    #[test]
    fn test_work_queue() {
        let queue = WorkQueue::new();
        let receiver = queue.get_receiver();

        queue.send(1).unwrap();
        queue.send(2).unwrap();
        queue.send(3).unwrap();
        queue.close();

        let rx = receiver.lock().unwrap();
        let items: Vec<i32> = rx.try_iter().collect();
        assert_eq!(items, vec![1, 2, 3]);
    }
}
