// this_file: src/memory_tracker.rs
//! Memory tracking and optimization utilities

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global memory tracker for monitoring allocation
pub struct MemoryTracker {
    /// Current memory usage in bytes
    current: Arc<AtomicUsize>,
    /// Peak memory usage in bytes
    peak: Arc<AtomicUsize>,
    /// Memory limit in bytes
    limit: usize,
}

impl MemoryTracker {
    /// Create a new memory tracker with the specified limit
    pub fn new(limit_mb: usize) -> Self {
        Self {
            current: Arc::new(AtomicUsize::new(0)),
            peak: Arc::new(AtomicUsize::new(0)),
            limit: limit_mb * 1024 * 1024,
        }
    }

    /// Track memory allocation
    pub fn allocate(&self, bytes: usize) -> Result<(), String> {
        let old = self.current.fetch_add(bytes, Ordering::SeqCst);
        let new = old + bytes;

        // Check limit
        if new > self.limit {
            self.current.fetch_sub(bytes, Ordering::SeqCst);
            return Err(format!(
                "Memory limit exceeded: {} MB requested, {} MB limit",
                new / (1024 * 1024),
                self.limit / (1024 * 1024)
            ));
        }

        // Update peak
        let mut peak = self.peak.load(Ordering::Relaxed);
        while new > peak {
            match self
                .peak
                .compare_exchange_weak(peak, new, Ordering::SeqCst, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }

        Ok(())
    }

    /// Track memory deallocation
    pub fn deallocate(&self, bytes: usize) {
        self.current.fetch_sub(bytes, Ordering::SeqCst);
    }

    /// Get current memory usage in bytes
    pub fn current_usage(&self) -> usize {
        self.current.load(Ordering::Relaxed)
    }

    /// Get peak memory usage in bytes
    pub fn peak_usage(&self) -> usize {
        self.peak.load(Ordering::Relaxed)
    }

    /// Get memory limit in bytes
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Get memory statistics as a formatted string
    pub fn stats(&self) -> String {
        format!(
            "Memory: {} MB / {} MB (peak: {} MB)",
            self.current_usage() / (1024 * 1024),
            self.limit / (1024 * 1024),
            self.peak_usage() / (1024 * 1024)
        )
    }

    /// Reset peak memory tracking
    pub fn reset_peak(&self) {
        let current = self.current.load(Ordering::Relaxed);
        self.peak.store(current, Ordering::Relaxed);
    }
}

/// RAII guard for tracking memory allocation lifetime
pub struct MemoryGuard {
    tracker: Arc<MemoryTracker>,
    bytes: usize,
}

impl MemoryGuard {
    /// Create a new memory guard
    pub fn new(tracker: Arc<MemoryTracker>, bytes: usize) -> Result<Self, String> {
        tracker.allocate(bytes)?;
        Ok(Self { tracker, bytes })
    }
}

impl Drop for MemoryGuard {
    fn drop(&mut self) {
        self.tracker.deallocate(self.bytes);
    }
}

/// Memory pool for efficient batch allocations
pub struct MemoryPool {
    tracker: Arc<MemoryTracker>,
    chunk_size: usize,
    free_chunks: Vec<Vec<u8>>,
    allocated_chunks: Vec<Vec<u8>>,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(tracker: Arc<MemoryTracker>, chunk_size: usize) -> Self {
        Self {
            tracker,
            chunk_size,
            free_chunks: Vec::new(),
            allocated_chunks: Vec::new(),
        }
    }

    /// Allocate a chunk from the pool
    pub fn allocate(&mut self) -> Result<Vec<u8>, String> {
        if let Some(chunk) = self.free_chunks.pop() {
            Ok(chunk)
        } else {
            // Allocate new chunk
            self.tracker.allocate(self.chunk_size)?;
            Ok(vec![0u8; self.chunk_size])
        }
    }

    /// Return a chunk to the pool
    pub fn deallocate(&mut self, mut chunk: Vec<u8>) {
        chunk.clear();
        chunk.resize(self.chunk_size, 0);
        self.free_chunks.push(chunk);
    }

    /// Clear all free chunks to release memory
    pub fn clear(&mut self) {
        let bytes_freed = self.free_chunks.len() * self.chunk_size;
        self.free_chunks.clear();
        self.tracker.deallocate(bytes_freed);
    }
}

/// Batch processor with memory limits
pub struct BatchProcessor {
    tracker: Arc<MemoryTracker>,
    batch_size: usize,
    max_concurrent: usize,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(tracker: Arc<MemoryTracker>, batch_size: usize) -> Self {
        let max_concurrent = num_cpus::get().min(8);
        Self {
            tracker,
            batch_size,
            max_concurrent,
        }
    }

    /// Calculate optimal batch size based on available memory
    pub fn optimal_batch_size(&self, item_size_estimate: usize) -> usize {
        let available = self.tracker.limit() - self.tracker.current_usage();
        let max_items = available / item_size_estimate;

        max_items.min(self.batch_size)
    }

    /// Check if we should process the next batch
    pub fn can_process_batch(&self, estimated_size: usize) -> bool {
        let current = self.tracker.current_usage();
        let limit = self.tracker.limit();

        // Keep 20% buffer
        let threshold = limit * 80 / 100;

        current + estimated_size < threshold
    }

    /// Get the number of concurrent workers based on memory
    pub fn concurrent_workers(&self) -> usize {
        let current = self.tracker.current_usage();
        let limit = self.tracker.limit();
        let usage_ratio = current as f64 / limit as f64;

        if usage_ratio < 0.5 {
            self.max_concurrent
        } else if usage_ratio < 0.7 {
            self.max_concurrent / 2
        } else {
            1 // Single threaded when memory is tight
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tracker_basic() {
        let tracker = MemoryTracker::new(100); // 100 MB limit

        // Allocate some memory
        assert!(tracker.allocate(10 * 1024 * 1024).is_ok());
        assert_eq!(tracker.current_usage(), 10 * 1024 * 1024);

        // Deallocate
        tracker.deallocate(5 * 1024 * 1024);
        assert_eq!(tracker.current_usage(), 5 * 1024 * 1024);
    }

    #[test]
    fn test_memory_tracker_limit() {
        let tracker = MemoryTracker::new(10); // 10 MB limit

        // Try to exceed limit
        assert!(tracker.allocate(11 * 1024 * 1024).is_err());
        assert_eq!(tracker.current_usage(), 0);
    }

    #[test]
    fn test_memory_tracker_peak() {
        let tracker = MemoryTracker::new(100);

        tracker.allocate(50 * 1024 * 1024).unwrap();
        assert_eq!(tracker.peak_usage(), 50 * 1024 * 1024);

        tracker.deallocate(30 * 1024 * 1024);
        assert_eq!(tracker.peak_usage(), 50 * 1024 * 1024); // Peak unchanged

        tracker.allocate(10 * 1024 * 1024).unwrap();
        assert_eq!(tracker.peak_usage(), 50 * 1024 * 1024); // Still at 50
    }

    #[test]
    fn test_memory_guard() {
        let tracker = Arc::new(MemoryTracker::new(100));

        {
            let _guard = MemoryGuard::new(tracker.clone(), 10 * 1024 * 1024).unwrap();
            assert_eq!(tracker.current_usage(), 10 * 1024 * 1024);
        } // Guard drops here

        assert_eq!(tracker.current_usage(), 0); // Memory freed
    }

    #[test]
    fn test_memory_pool() {
        let tracker = Arc::new(MemoryTracker::new(100));
        let mut pool = MemoryPool::new(tracker.clone(), 1024);

        // Allocate chunks
        let chunk1 = pool.allocate().unwrap();
        assert_eq!(chunk1.len(), 1024);
        assert_eq!(tracker.current_usage(), 1024);

        // Return to pool
        pool.deallocate(chunk1);

        // Reuse from pool (no new allocation)
        let chunk2 = pool.allocate().unwrap();
        assert_eq!(chunk2.len(), 1024);
        assert_eq!(tracker.current_usage(), 1024); // Still just 1024
    }

    #[test]
    fn test_batch_processor() {
        let tracker = Arc::new(MemoryTracker::new(100));
        let processor = BatchProcessor::new(tracker.clone(), 1000);

        // Test optimal batch size
        let batch_size = processor.optimal_batch_size(1024); // 1KB per item
        assert!(batch_size > 0);
        assert!(batch_size <= 1000);

        // Test can process batch
        assert!(processor.can_process_batch(10 * 1024 * 1024)); // 10 MB

        // Use most of the memory
        tracker.allocate(75 * 1024 * 1024).unwrap();
        assert!(!processor.can_process_batch(20 * 1024 * 1024)); // Can't fit 20 MB more
    }

    #[test]
    fn test_concurrent_workers_scaling() {
        let tracker = Arc::new(MemoryTracker::new(100));
        let processor = BatchProcessor::new(tracker.clone(), 1000);
        let max_workers = processor.concurrent_workers();

        // Low memory usage - max workers
        assert_eq!(processor.concurrent_workers(), max_workers);

        // Medium memory usage - half workers
        tracker.allocate(60 * 1024 * 1024).unwrap();
        assert_eq!(processor.concurrent_workers(), max_workers / 2);

        // High memory usage - single worker
        tracker.allocate(20 * 1024 * 1024).unwrap();
        assert_eq!(processor.concurrent_workers(), 1);
    }
}
