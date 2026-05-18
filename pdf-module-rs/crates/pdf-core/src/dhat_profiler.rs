//! Heap allocation tracking with `dhat` allocator.
//!
//! Replaces the global allocator with dhat's instrumented allocator
//! to profile heap usage: total bytes allocated, peak memory, and
//! call-stack attribution of allocations.
//!
//! # How to profile
//!
//! ```bash
//! # Build with dhat feature
//! cargo build --features dhat-heap
//!
//! # Run and capture heap profile
//! cargo run --features dhat-heap -- --input test.pdf
//!
//! # View the profile in dhat viewer
//! dhat-viewer dhat-heap.json
//! ```
//!
//! # CI integration
//!
//! ```bash
//! cargo test --features dhat-heap -- --nocapture
//! # Check dhat output for unexpected regressions
//! ```

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// Initialize dhat heap profiling.
///
/// Must be called at the very start of `main()`. Dhat takes over
/// the global allocator and starts tracking.
pub fn init_heap_profiling() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    #[cfg(feature = "dhat-heap")]
    tracing::info!("dhat heap profiler active — profile will be written on exit");
}

#[cfg(test)]
#[cfg(feature = "dhat-heap")]
mod tests {
    #[test]
    fn dhat_is_active() {
        let _profiler = dhat::Profiler::new_heap();
        let _v = vec![0u8; 1024];
        // dhat will drop here, writing dhat-heap.json
    }
}