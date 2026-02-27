//! Parallel Indexing Infrastructure
//!
//! Provides configurable parallel processing for large repositories:
//! - Thread pool configuration
//! - Work stealing for load balancing
//! - Batch processing with adaptive sizing
//! - Progress tracking and cancellation

use rayon::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Configuration for parallel indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    /// Number of worker threads (0 = auto-detect based on CPUs)
    pub num_threads: usize,
    /// Minimum batch size before parallel processing kicks in
    pub min_batch_size: usize,
    /// Maximum batch size for work distribution
    pub max_batch_size: usize,
    /// Enable work stealing for load balancing
    pub work_stealing: bool,
    /// Stack size for worker threads (in KB, 0 = default)
    pub stack_size_kb: usize,
    /// Time to wait for thread pool to complete work
    pub shutdown_timeout_secs: u64,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        Self {
            num_threads: num_cpus,
            min_batch_size: 10,
            max_batch_size: 1000,
            work_stealing: true,
            stack_size_kb: 0, // Use default
            shutdown_timeout_secs: 30,
        }
    }
}

/// Parallel processing statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParallelStats {
    /// Total files processed
    pub files_processed: usize,
    /// Files processed in parallel
    pub parallel_batches: usize,
    /// Average batch size
    pub avg_batch_size: f64,
    /// Thread pool utilization (0.0 - 1.0)
    pub utilization: f64,
    /// Time spent in parallel processing
    pub parallel_time_ms: u64,
    /// Number of work stealing events
    pub work_steals: usize,
}

/// Manages parallel indexing operations
pub struct ParallelProcessor {
    config: ParallelConfig,
    pool: ThreadPool,
    /// Flag to signal cancellation
    cancelled: Arc<AtomicBool>,
    /// Files processed counter
    files_processed: Arc<AtomicUsize>,
    /// Batches processed counter
    batches_processed: Arc<AtomicUsize>,
}

impl ParallelProcessor {
    /// Create a new parallel processor with default configuration
    pub fn new() -> Self {
        Self::with_config(ParallelConfig::default())
    }

    /// Create a parallel processor with custom configuration
    pub fn with_config(config: ParallelConfig) -> Self {
        let num_threads = if config.num_threads == 0 {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        } else {
            config.num_threads
        };

        let mut builder = ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .thread_name(|i| format!("cortex-indexer-{}", i));

        if config.stack_size_kb > 0 {
            builder = builder.stack_size(config.stack_size_kb * 1024);
        }

        let pool = builder.build().expect("Failed to create thread pool");

        Self {
            config,
            pool,
            cancelled: Arc::new(AtomicBool::new(false)),
            files_processed: Arc::new(AtomicUsize::new(0)),
            batches_processed: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Get the number of threads in the pool
    pub fn num_threads(&self) -> usize {
        self.pool.current_num_threads()
    }

    /// Check if processing has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Cancel all ongoing processing
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Reset cancellation flag
    pub fn reset_cancel(&self) {
        self.cancelled.store(false, Ordering::Relaxed);
    }

    /// Get the current number of files processed
    pub fn files_processed(&self) -> usize {
        self.files_processed.load(Ordering::Relaxed)
    }

    /// Calculate optimal batch size based on total items
    pub fn calculate_batch_size(&self, total_items: usize) -> usize {
        if total_items < self.config.min_batch_size {
            return total_items;
        }

        let num_threads = self.num_threads();
        let items_per_thread = total_items / num_threads;

        // Aim for at least 4 batches per thread for better work distribution
        let optimal_batch = items_per_thread / 4;

        // Clamp to configured range
        optimal_batch
            .max(self.config.min_batch_size)
            .min(self.config.max_batch_size)
    }

    /// Process items in parallel using the configured thread pool
    pub fn process_parallel<T, R, F>(
        &self,
        items: Vec<T>,
        processor: F,
    ) -> Vec<R>
    where
        T: Send,
        R: Send,
        F: Fn(&T) -> R + Send + Sync,
    {
        if items.is_empty() {
            return Vec::new();
        }

        // Reset counters
        self.files_processed.store(0, Ordering::Relaxed);
        self.batches_processed.store(0, Ordering::Relaxed);
        self.reset_cancel();

        let batch_size = self.calculate_batch_size(items.len());

        if items.len() < self.config.min_batch_size {
            // Process sequentially for small batches
            items.iter().map(&processor).collect()
        } else {
            // Process in parallel
            let processor_arc = Arc::new(processor);
            let cancelled = Arc::clone(&self.cancelled);
            let files_processed = Arc::clone(&self.files_processed);

            self.pool.install(|| {
                items
                    .into_par_iter()
                    .with_min_len(batch_size)
                    .map(|item| {
                        if cancelled.load(Ordering::Relaxed) {
                            return None;
                        }
                        let result = processor_arc(&item);
                        files_processed.fetch_add(1, Ordering::Relaxed);
                        Some(result)
                    })
                    .while_some()
                    .collect()
            })
        }
    }

    /// Process items in parallel with fallible processor
    pub fn process_parallel_fallible<T, R, E, F>(
        &self,
        items: Vec<T>,
        processor: F,
    ) -> Vec<Result<R, E>>
    where
        T: Send,
        R: Send,
        E: Send,
        F: Fn(&T) -> Result<R, E> + Send + Sync,
    {
        if items.is_empty() {
            return Vec::new();
        }

        // Reset counters
        self.files_processed.store(0, Ordering::Relaxed);
        self.batches_processed.store(0, Ordering::Relaxed);
        self.reset_cancel();

        let batch_size = self.calculate_batch_size(items.len());

        if items.len() < self.config.min_batch_size {
            items.iter().map(&processor).collect()
        } else {
            let processor_arc = Arc::new(processor);
            let cancelled = Arc::clone(&self.cancelled);
            let files_processed = Arc::clone(&self.files_processed);

            self.pool.install(|| {
                items
                    .into_par_iter()
                    .with_min_len(batch_size)
                    .map(|item| {
                        if cancelled.load(Ordering::Relaxed) {
                            files_processed.fetch_add(1, Ordering::Relaxed);
                            return None;
                        }
                        let result = processor_arc(&item);
                        files_processed.fetch_add(1, Ordering::Relaxed);
                        Some(result)
                    })
                    .while_some()
                    .collect()
            })
        }
    }

    /// Get statistics about the parallel processing
    pub fn stats(&self) -> ParallelStats {
        let files = self.files_processed.load(Ordering::Relaxed);
        let batches = self.batches_processed.load(Ordering::Relaxed);

        ParallelStats {
            files_processed: files,
            parallel_batches: batches,
            avg_batch_size: if batches > 0 {
                files as f64 / batches as f64
            } else {
                0.0
            },
            utilization: if files > 0 && self.config.num_threads > 0 {
                1.0 // Simplified; real implementation would track actual utilization
            } else {
                0.0
            },
            parallel_time_ms: 0, // Would need timing wrapper
            work_steals: 0,      // Rayon doesn't expose this directly
        }
    }
}

impl Default for ParallelProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptive batch size calculator
#[derive(Debug, Clone)]
pub struct AdaptiveBatcher {
    min_size: usize,
    max_size: usize,
    current_size: usize,
    /// Track processing times for adaptation
    recent_times: Vec<Duration>,
}

impl AdaptiveBatcher {
    /// Create a new adaptive batcher
    pub fn new(min_size: usize, max_size: usize) -> Self {
        Self {
            min_size,
            max_size,
            current_size: min_size,
            recent_times: Vec::with_capacity(10),
        }
    }

    /// Get the current batch size
    pub fn current_size(&self) -> usize {
        self.current_size
    }

    /// Record a processing time and adapt batch size
    pub fn record_time(&mut self, duration: Duration) {
        self.recent_times.push(duration);
        if self.recent_times.len() > 10 {
            self.recent_times.remove(0);
        }

        self.adapt();
    }

    /// Adapt batch size based on recent processing times
    fn adapt(&mut self) {
        if self.recent_times.len() < 3 {
            return;
        }

        // Calculate average processing time
        let avg_nanos: u128 = self.recent_times.iter().map(|d| d.as_nanos()).sum::<u128>()
            / self.recent_times.len() as u128;

        // If processing is fast, increase batch size
        // If processing is slow, decrease batch size
        let target_time_ns = 10_000_000; // 10ms target per batch

        if avg_nanos < target_time_ns / 2 {
            // Processing is fast, increase batch size
            self.current_size = (self.current_size * 2).min(self.max_size);
        } else if avg_nanos > target_time_ns * 2 {
            // Processing is slow, decrease batch size
            self.current_size = (self.current_size / 2).max(self.min_size);
        }
    }

    /// Split items into batches
    pub fn batch<T: Clone>(&self, items: Vec<T>) -> Vec<Vec<T>> {
        let batch_size = self.current_size;
        items
            .chunks(batch_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_config_default() {
        let config = ParallelConfig::default();
        assert!(config.num_threads > 0);
        assert_eq!(config.min_batch_size, 10);
        assert!(config.work_stealing);
    }

    #[test]
    fn parallel_processor_new() {
        let processor = ParallelProcessor::new();
        assert!(processor.num_threads() > 0);
    }

    #[test]
    fn parallel_processor_custom_config() {
        let config = ParallelConfig {
            num_threads: 2,
            min_batch_size: 5,
            ..Default::default()
        };
        let processor = ParallelProcessor::with_config(config);
        assert_eq!(processor.num_threads(), 2);
    }

    #[test]
    fn parallel_processor_small_batch_sequential() {
        let processor = ParallelProcessor::with_config(ParallelConfig {
            min_batch_size: 100,
            ..Default::default()
        });

        let items = vec![1, 2, 3];
        let results: Vec<i32> = processor.process_parallel(items, |x| x * 2);

        assert_eq!(results, vec![2, 4, 6]);
    }

    #[test]
    fn parallel_processor_large_batch_parallel() {
        let processor = ParallelProcessor::new();

        let items: Vec<i32> = (0..1000).collect();
        let results: Vec<i32> = processor.process_parallel(items, |x| x * 2);

        assert_eq!(results.len(), 1000);
        assert_eq!(results[0], 0);
        assert_eq!(results[500], 1000);
    }

    #[test]
    fn parallel_processor_cancellation() {
        let processor = ParallelProcessor::new();
        processor.cancel();
        assert!(processor.is_cancelled());

        processor.reset_cancel();
        assert!(!processor.is_cancelled());
    }

    #[test]
    fn calculate_batch_size() {
        let processor = ParallelProcessor::new();

        // Small batch
        let batch = processor.calculate_batch_size(5);
        assert!(batch >= 5 || batch == 5);

        // Large batch
        let batch = processor.calculate_batch_size(10000);
        assert!(batch >= processor.config.min_batch_size);
        assert!(batch <= processor.config.max_batch_size);
    }

    #[test]
    fn parallel_stats() {
        let processor = ParallelProcessor::new();
        let stats = processor.stats();
        assert_eq!(stats.files_processed, 0);
    }

    #[test]
    fn adaptive_batcher_new() {
        let batcher = AdaptiveBatcher::new(10, 100);
        assert_eq!(batcher.current_size(), 10);
    }

    #[test]
    fn adaptive_batcher_adaptation() {
        let mut batcher = AdaptiveBatcher::new(10, 100);

        // Record fast times - should increase batch size
        for _ in 0..5 {
            batcher.record_time(Duration::from_micros(100));
        }

        // Batch size should have increased
        assert!(batcher.current_size() > 10);
    }

    #[test]
    fn adaptive_batcher_batch() {
        let batcher = AdaptiveBatcher::new(10, 100);
        let items: Vec<i32> = (0..25).collect();
        let batches = batcher.batch(items);

        assert!(!batches.is_empty());
        let total: usize = batches.iter().map(|b| b.len()).sum();
        assert_eq!(total, 25);
    }
}
