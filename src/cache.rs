// this_file: src/cache.rs

//! Glyph-level caching for repeated renders.
//!
//! The glyph cache stores serialized rendering payloads so identical jobs can
//! bypass shaping and rasterization. It is intentionally small and simple — an
//! `lru` map guarded by a mutex — because the access pattern is dominated by
//! fontsimi’s single-glyph probes.

use crate::JobPayload;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Unique key for cached render payloads.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct GlyphCacheKey {
    pub(crate) font_path: String,
    pub(crate) font_size: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: String,
    pub(crate) encoding: String,
    pub(crate) text: String,
    pub(crate) script: Option<String>,
    pub(crate) variations: Vec<(String, u32)>,
}

/// Lightweight stats for observability.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GlyphCacheStats {
    pub(crate) entries: usize,
    pub(crate) capacity: usize,
    pub(crate) hits: u64,
}

/// Shared glyph cache (LRU).
#[derive(Clone, Debug)]
pub(crate) struct GlyphCache {
    inner: Arc<Mutex<Option<LruCache<GlyphCacheKey, JobPayload>>>>,
    hits: Arc<AtomicU64>,
}

impl GlyphCache {
    /// Create a cache with the requested capacity. Returns `None` when disabled.
    pub(crate) fn new(capacity: usize) -> Option<Self> {
        NonZeroUsize::new(capacity).map(|cap| Self {
            inner: Arc::new(Mutex::new(Some(LruCache::new(cap)))),
            hits: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Retrieve a cached payload if present.
    pub(crate) fn get(&self, key: &GlyphCacheKey) -> Option<JobPayload> {
        let mut guard = self.inner.lock().expect("glyph cache mutex poisoned");
        guard
            .as_mut()
            .and_then(|cache| cache.get(key).cloned())
            .map(|payload| {
                self.hits.fetch_add(1, Ordering::Relaxed);
                payload
            })
    }

    /// Insert a payload into the cache.
    pub(crate) fn insert(&self, key: GlyphCacheKey, payload: JobPayload) {
        if let Some(cache) = self
            .inner
            .lock()
            .expect("glyph cache mutex poisoned")
            .as_mut()
        {
            cache.put(key, payload);
        }
    }

    /// Remove all cached entries.
    pub(crate) fn clear(&self) {
        if let Some(cache) = self
            .inner
            .lock()
            .expect("glyph cache mutex poisoned")
            .as_mut()
        {
            cache.clear();
        }
    }

    /// Resize the cache (capacity of 0 disables caching).
    pub(crate) fn set_capacity(&self, capacity: usize) {
        let mut guard = self.inner.lock().expect("glyph cache mutex poisoned");
        if capacity == 0 {
            *guard = None;
            return;
        }
        let cap = NonZeroUsize::new(capacity).expect("capacity must be > 0");
        match guard.as_mut() {
            Some(cache) => cache.resize(cap),
            None => *guard = Some(LruCache::new(cap)),
        }
    }

    /// Current stats.
    pub(crate) fn stats(&self) -> GlyphCacheStats {
        let guard = self.inner.lock().expect("glyph cache mutex poisoned");
        if let Some(cache) = guard.as_ref() {
            GlyphCacheStats {
                entries: cache.len(),
                capacity: cache.cap().get(),
                hits: self.hits.load(Ordering::Relaxed),
            }
        } else {
            GlyphCacheStats {
                entries: 0,
                capacity: 0,
                hits: self.hits.load(Ordering::Relaxed),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch::RenderingOutput;

    #[test]
    fn glyph_cache_stores_and_retrieves_payloads() {
        let cache = GlyphCache::new(4).expect("cache enabled");
        let key = GlyphCacheKey {
            font_path: "font".into(),
            font_size: 640,
            width: 64,
            height: 64,
            format: "pgm".into(),
            encoding: "base64".into(),
            text: "A".into(),
            script: Some("Latn".into()),
            variations: vec![],
        };
        let payload = JobPayload::Rendering(RenderingOutput {
            format: "pgm".into(),
            encoding: "base64".into(),
            data: "data".into(),
            width: 64,
            height: 64,
            actual_bbox: (0, 0, 0, 0),
        });
        cache.insert(key.clone(), payload.clone());
        let hit = cache.get(&key).expect("payload present");
        match hit {
            JobPayload::Rendering(output) => assert_eq!(output.data, "data"),
            _ => panic!("expected rendering payload"),
        }
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn glyph_cache_can_be_disabled() {
        let cache = GlyphCache::new(2).expect("cache enabled");
        cache.set_capacity(0);
        let stats = cache.stats();
        assert_eq!(stats.capacity, 0);
        assert_eq!(stats.entries, 0);
        let key = GlyphCacheKey {
            font_path: "font".into(),
            font_size: 1,
            width: 1,
            height: 1,
            format: "pgm".into(),
            encoding: "base64".into(),
            text: "a".into(),
            script: None,
            variations: vec![],
        };
        cache.insert(
            key.clone(),
            JobPayload::Rendering(RenderingOutput {
                format: "pgm".into(),
                encoding: "base64".into(),
                data: "x".into(),
                width: 1,
                height: 1,
                actual_bbox: (0, 0, 1, 1),
            }),
        );
        assert!(cache.get(&key).is_none(), "cache disabled should miss");
    }
}
