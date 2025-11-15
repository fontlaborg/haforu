// this_file: src/bufpool.rs

//! Thread-local buffer pools for reducing allocation overhead.
//!
//! This module provides pooling for frequently allocated buffers like
//! canvas pixel data to minimize allocations during tight rendering loops.

use std::cell::RefCell;

thread_local! {
    // Thread-local pool for reusable canvas buffers.
    //
    // This reduces allocations during high-frequency rendering by maintaining
    // a small cache of pre-allocated Vec<u8> buffers per thread.
    static CANVAS_POOL: RefCell<CanvasPool> = RefCell::new(CanvasPool::new());
}

/// Pool for canvas pixel buffers.
struct CanvasPool {
    buffers: Vec<Vec<u8>>,
    max_pooled: usize,
}

impl CanvasPool {
    /// Create a new canvas pool.
    fn new() -> Self {
        Self {
            buffers: Vec::with_capacity(8),
            max_pooled: 8,
        }
    }

    /// Get a buffer of at least the specified size.
    ///
    /// Returns a cleared buffer ready for rendering.
    fn get(&mut self, size: usize) -> Vec<u8> {
        // Try to find a suitable buffer in the pool
        if let Some(idx) = self.buffers.iter().position(|buf| buf.capacity() >= size) {
            let mut buf = self.buffers.swap_remove(idx);
            buf.clear();
            buf.resize(size, 0);
            buf
        } else {
            // Allocate a new buffer
            vec![0u8; size]
        }
    }

    /// Return a buffer to the pool for reuse.
    fn put(&mut self, mut buf: Vec<u8>) {
        if self.buffers.len() < self.max_pooled {
            buf.clear();
            self.buffers.push(buf);
        }
        // Otherwise drop the buffer
    }
}

/// Get a canvas buffer from the thread-local pool.
///
/// The buffer will be zeroed and ready for rendering.
#[inline]
pub fn get_canvas_buffer(size: usize) -> Vec<u8> {
    CANVAS_POOL.with(|pool| pool.borrow_mut().get(size))
}

/// Return a canvas buffer to the thread-local pool.
///
/// The buffer may be reused for future renders.
#[inline]
pub fn return_canvas_buffer(buf: Vec<u8>) {
    CANVAS_POOL.with(|pool| pool.borrow_mut().put(buf));
}

/// RAII guard that automatically returns buffer to pool on drop.
pub struct PooledBuffer {
    buf: Option<Vec<u8>>,
}

impl PooledBuffer {
    /// Create a new pooled buffer of the specified size.
    #[inline]
    pub fn new(size: usize) -> Self {
        Self {
            buf: Some(get_canvas_buffer(size)),
        }
    }

    /// Get a mutable reference to the underlying buffer.
    #[inline]
    pub fn as_mut(&mut self) -> &mut Vec<u8> {
        self.buf.as_mut().expect("Buffer already taken")
    }

    /// Get an immutable reference to the underlying buffer.
    #[inline]
    pub fn as_ref(&self) -> &Vec<u8> {
        self.buf.as_ref().expect("Buffer already taken")
    }

    /// Take ownership of the buffer, preventing automatic return to pool.
    #[inline]
    pub fn take(mut self) -> Vec<u8> {
        self.buf.take().expect("Buffer already taken")
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(buf) = self.buf.take() {
            return_canvas_buffer(buf);
        }
    }
}

impl std::ops::Deref for PooledBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl std::ops::DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pooled_buffer_lifecycle() {
        let size = 1024;

        // Create and drop a buffer
        {
            let mut buf = PooledBuffer::new(size);
            assert_eq!(buf.len(), size);
            buf[0] = 42;
        }

        // Get another buffer - might be from pool
        {
            let buf = PooledBuffer::new(size);
            assert_eq!(buf.len(), size);
            // Should be zeroed even if from pool
            assert_eq!(buf[0], 0);
        }
    }

    #[test]
    fn take_prevents_return_to_pool() {
        let size = 512;
        let buf = PooledBuffer::new(size);
        let owned = buf.take();
        assert_eq!(owned.len(), size);
        // Buffer is now owned and won't be returned to pool
    }
}
