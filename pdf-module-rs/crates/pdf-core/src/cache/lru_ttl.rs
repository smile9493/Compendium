//! Generic LRU + TTL cache implementation.
//!
//! Provides bounded-memory caching with configurable:
//! - `max_entries`: hard capacity cap (evicts LRU when exceeded)
//! - `ttl`: time-to-live per entry (soft eviction on access)
//!
//! Thread-safe via `Arc<RwLock<>>` — read path allows concurrent lookups.
//! Atomic hit/miss counters for Prometheus observability.
//!
//! # Example
//!
//! ```ignore
//! use std::time::Duration;
//! use pdf_core::cache::LruTtlCache;
//!
//! let cache: LruTtlCache<String, String> = LruTtlCache::new(100, Duration::from_secs(300));
//! cache.insert("key".into(), "value".into());
//! assert_eq!(cache.get("key"), Some("value".into()));
//! ```

use std::borrow::Borrow;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use tracing::debug;

struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
}

struct Inner<K, V> {
    map: HashMap<K, CacheEntry<V>>,
    order: VecDeque<K>,
    max_entries: usize,
    ttl: Duration,
}

/// A generic LRU + TTL cache.
///
/// - **LRU eviction**: when `max_entries` is exceeded, the oldest inserted entry is removed.
/// - **TTL eviction**: entries are lazily evicted on `get()` if their age exceeds `ttl`.
/// - **Thread-safe**: all operations go through `Arc<RwLock<>>`.
/// - **Observable**: `hit_count()` and `miss_count()` for Prometheus metrics.
pub struct LruTtlCache<K, V> {
    inner: Arc<RwLock<Inner<K, V>>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl<K, V> LruTtlCache<K, V>
where
    K: Clone + Hash + Eq,
    V: Clone,
{
    /// Create a new cache with the given capacity and TTL.
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                map: HashMap::with_capacity(max_entries),
                order: VecDeque::with_capacity(max_entries),
                max_entries,
                ttl,
            })),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Get a value by key, returning `None` if missing or TTL-expired.
    ///
    /// Uses the `Borrow` pattern so you can pass `&str` when `K: String`.
    /// On a hit, the entry's position is **not** refreshed in the LRU order
    /// (we use insertion-order eviction, not access-order). This keeps
    /// `get()` as a fast O(1) read without write-lock contention.
    pub fn get<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let inner = self.inner.read().ok()?;
        match inner.map.get(key) {
            Some(entry) if entry.inserted_at.elapsed() < inner.ttl => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(entry.value.clone())
            }
            Some(_) => {
                // TTL expired — drop the lock and evict under write lock
                drop(inner);
                self.evict_expired(key);
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Insert or update a value, evicting the oldest entry if at capacity.
    pub fn insert(&self, key: K, value: V) {
        let mut inner = self.inner.write().expect("cache lock poisoned");

        // Evict oldest if at capacity (and the key doesn't already exist)
        if inner.map.len() >= inner.max_entries && !inner.map.contains_key(&key) {
            if let Some(old) = inner.order.pop_front() {
                inner.map.remove(&old);
                debug!(evicted = true, "LruTtlCache evicted oldest entry");
            }
        }

        // If key already exists, remove old order entry — new insert at back
        if inner.map.contains_key(&key) {
            inner.order.retain(|k| k != &key);
        }

        inner.map.insert(
            key.clone(),
            CacheEntry {
                value,
                inserted_at: Instant::now(),
            },
        );
        inner.order.push_back(key);
    }

    /// Remove a specific key from the cache.
    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let mut inner = self.inner.write().expect("cache lock poisoned");
        let removed = inner.map.remove(key);
        if removed.is_some() {
            inner.order.retain(|k| k.borrow() != key);
        }
        removed.map(|e| e.value)
    }

    /// Clear all entries.
    pub fn clear(&self) {
        let mut inner = self.inner.write().expect("cache lock poisoned");
        inner.map.clear();
        inner.order.clear();
    }

    /// Current number of entries in the cache.
    pub fn len(&self) -> usize {
        self.inner.read().map(|i| i.map.len()).unwrap_or(0)
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Maximum number of entries.
    pub fn max_entries(&self) -> usize {
        self.inner.read().map(|i| i.max_entries).unwrap_or(0)
    }

    /// Total cache hits since creation.
    pub fn hit_count(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Total cache misses since creation.
    pub fn miss_count(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Hit ratio as a float in [0.0, 1.0].
    pub fn hit_ratio(&self) -> f64 {
        let hits = self.hit_count();
        let misses = self.miss_count();
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Evict a key whose TTL has expired (called internally during get).
    fn evict_expired<Q>(&self, key: &Q)
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let mut inner = self.inner.write().expect("cache lock poisoned");
        if let Some(entry) = inner.map.get(key) {
            if entry.inserted_at.elapsed() >= inner.ttl {
                inner.map.remove(key);
                inner.order.retain(|k| k.borrow() != key);
            }
        }
    }
}

impl<K, V> Clone for LruTtlCache<K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            hits: AtomicU64::new(self.hits.load(Ordering::Relaxed)),
            misses: AtomicU64::new(self.misses.load(Ordering::Relaxed)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_hit_on_fresh_insert() {
        let cache: LruTtlCache<String, String> = LruTtlCache::new(10, Duration::from_secs(60));
        cache.insert("a".into(), "alpha".into());
        assert_eq!(cache.get("a"), Some("alpha".into()));
        assert_eq!(cache.hit_count(), 1);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn test_miss_on_missing_key() {
        let cache: LruTtlCache<String, String> = LruTtlCache::new(10, Duration::from_secs(60));
        assert_eq!(cache.get("nonexistent"), None);
        assert_eq!(cache.hit_count(), 0);
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn test_miss_after_ttl() {
        let cache: LruTtlCache<String, String> = LruTtlCache::new(10, Duration::from_millis(1));
        cache.insert("a".into(), "alpha".into());
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.hit_count(), 0);
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn test_eviction_at_capacity() {
        let cache: LruTtlCache<u64, String> = LruTtlCache::new(3, Duration::from_secs(60));
        cache.insert(1, "one".into());
        cache.insert(2, "two".into());
        cache.insert(3, "three".into());
        cache.insert(4, "four".into()); // should evict key 1

        assert_eq!(cache.get(&1), None); // evicted
        assert_eq!(cache.get(&2), Some("two".into()));
        assert_eq!(cache.get(&3), Some("three".into()));
        assert_eq!(cache.get(&4), Some("four".into()));
    }

    #[test]
    fn test_update_replaces_value() {
        let cache: LruTtlCache<String, u64> = LruTtlCache::new(5, Duration::from_secs(60));
        cache.insert("counter".into(), 1);
        cache.insert("counter".into(), 2);
        assert_eq!(cache.get("counter"), Some(2));
    }

    #[test]
    fn test_remove() {
        let cache: LruTtlCache<String, String> = LruTtlCache::new(5, Duration::from_secs(60));
        cache.insert("a".into(), "alpha".into());
        assert_eq!(cache.remove("a"), Some("alpha".into()));
        assert_eq!(cache.get("a"), None);
    }

    #[test]
    fn test_clear() {
        let cache: LruTtlCache<u64, String> = LruTtlCache::new(5, Duration::from_secs(60));
        cache.insert(1, "one".into());
        cache.insert(2, "two".into());
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_len_and_max_entries() {
        let cache: LruTtlCache<String, String> = LruTtlCache::new(42, Duration::from_secs(60));
        assert_eq!(cache.max_entries(), 42);
        assert!(cache.is_empty());
        cache.insert("a".into(), "alpha".into());
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_hit_ratio() {
        let cache: LruTtlCache<String, String> = LruTtlCache::new(5, Duration::from_secs(60));
        assert_eq!(cache.hit_ratio(), 0.0);
        cache.insert("a".into(), "alpha".into());
        cache.get("a"); // hit
        cache.get("b"); // miss
        assert!((cache.hit_ratio() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_clone() {
        let cache: LruTtlCache<u64, String> = LruTtlCache::new(5, Duration::from_secs(60));
        cache.insert(1, "one".into());
        let cloned = cache.clone();
        assert_eq!(cloned.get(&1), Some("one".into()));
        cloned.insert(2, "two".into());
        assert_eq!(cache.get(&2), Some("two".into())); // shared Arc
    }
}
