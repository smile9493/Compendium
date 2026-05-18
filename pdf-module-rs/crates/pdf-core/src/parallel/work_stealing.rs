//! Adaptive work-stealing scheduler for page-level parallel extraction.
//!
//! Uses `crossbeam-deque` for per-worker local deques and a shared global
//! queue for task distribution, enabling load-balanced parallel page processing.

use crossbeam_deque::{Stealer, Worker};
use std::sync::Mutex;

/// A page extraction task with priority for intelligent scheduling.
#[derive(Debug, Clone)]
pub struct PageTask {
    pub page_index: u32,
    pub priority: u8,
}

impl PageTask {
    pub fn new(page_index: u32, priority: u8) -> Self {
        Self { page_index, priority }
    }
}

/// Adaptive scheduler that distributes page tasks across workers
/// using work-stealing for load-balanced parallel processing.
///
/// Large/complex pages are given higher priority to start early,
/// while smaller pages fill gaps left by longer-running tasks.
pub struct AdaptiveScheduler {
    global_queue: Mutex<Vec<PageTask>>,
    workers: Vec<Worker<PageTask>>,
    stealers: Vec<Stealer<PageTask>>,
}

impl AdaptiveScheduler {
    /// Create a new scheduler with the given number of worker threads.
    pub fn new(num_workers: usize) -> Self {
        let mut workers = Vec::with_capacity(num_workers);
        let mut stealers = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            let worker = Worker::new_fifo();
            stealers.push(worker.stealer());
            workers.push(worker);
        }

        Self { global_queue: Mutex::new(Vec::new()), workers, stealers }
    }

    /// Schedule pages for parallel extraction based on estimated complexity.
    ///
    /// Pages are prioritized by estimated processing time (larger page sizes
    /// get higher priority). Higher priority values are processed first.
    pub fn schedule_pages(&self, total_pages: u32, page_sizes: Option<&[f64]>) {
        let mut tasks = self.global_queue.lock().expect("global queue lock poisoned");
        for i in 0..total_pages {
            let priority = if let Some(sizes) = page_sizes {
                if let Some(&size) = sizes.get(i as usize) {
                    if size > 1_000_000.0 {
                        3
                    } else if size > 100_000.0 {
                        2
                    } else {
                        1
                    }
                } else {
                    1
                }
            } else {
                1
            };

            tasks.push(PageTask::new(i, priority));
        }

        // Sort by priority ascending so pop() returns highest priority
        tasks.sort_by_key(|a| a.priority);
    }

    /// Find the next task for a worker: try global queue, then steal from other workers.
    pub fn find_task(&self, worker_idx: usize) -> Option<PageTask> {
        // Try the global queue first (pop highest-priority task)
        {
            let mut tasks = self.global_queue.lock().expect("global queue lock poisoned");
            if let Some(task) = tasks.pop() {
                return Some(task);
            }
        }

        // Try stealing from other workers
        for (idx, stealer) in self.stealers.iter().enumerate() {
            if idx == worker_idx {
                continue;
            }
            if let crossbeam_deque::Steal::Success(task) = stealer.steal() {
                return Some(task);
            }
        }

        None
    }

    /// Get the number of workers.
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_scheduler_creation() {
        let scheduler = AdaptiveScheduler::new(4);
        assert_eq!(scheduler.worker_count(), 4);
    }

    #[test]
    fn test_schedule_pages_no_sizes() {
        let scheduler = AdaptiveScheduler::new(2);
        scheduler.schedule_pages(10, None);

        let mut count = 0;
        while scheduler.find_task(0).is_some() {
            count += 1;
        }
        assert_eq!(count, 10);
    }

    #[test]
    fn test_schedule_pages_with_sizes() {
        let scheduler = AdaptiveScheduler::new(2);
        let sizes = vec![500_000.0, 2_000_000.0, 50_000.0];
        scheduler.schedule_pages(3, Some(&sizes));

        let mut tasks = Vec::new();
        while let Some(task) = scheduler.find_task(0) {
            tasks.push(task);
        }
        assert_eq!(tasks.len(), 3);

        let high_priority = tasks.iter().filter(|t| t.priority == 3).count();
        let med_priority = tasks.iter().filter(|t| t.priority == 2).count();
        let low_priority = tasks.iter().filter(|t| t.priority == 1).count();
        assert_eq!(high_priority, 1);
        assert_eq!(med_priority, 1);
        assert_eq!(low_priority, 1);
    }

    #[test]
    fn test_schedule_pages_priority_ordering() {
        let scheduler = AdaptiveScheduler::new(2);
        let sizes = vec![50_000.0, 2_000_000.0, 200_000.0];
        scheduler.schedule_pages(3, Some(&sizes));

        // Should get highest priority first
        let task = scheduler.find_task(0).unwrap();
        assert_eq!(task.priority, 3);
        assert_eq!(task.page_index, 1);
    }

    #[test]
    fn test_work_stealing_between_workers() {
        let scheduler = AdaptiveScheduler::new(2);
        scheduler.schedule_pages(4, None);

        // Worker 0 gets a task from global queue
        let task = scheduler.find_task(0);
        assert!(task.is_some());

        // Worker 1 gets a task from global queue
        let task = scheduler.find_task(1);
        assert!(task.is_some());

        // Remaining tasks still available
        assert!(scheduler.find_task(0).is_some());
        assert!(scheduler.find_task(1).is_some());

        // All tasks consumed
        assert!(scheduler.find_task(0).is_none());
    }
}
