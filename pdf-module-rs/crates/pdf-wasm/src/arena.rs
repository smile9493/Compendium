//! WASM PDF engine with Arena allocator for efficient memory management.
//!
//! Uses `bumpalo::Bump` as an arena allocator that can be reset after
//! processing each PDF page, providing O(1) deallocation for temporary
//! buffers and avoiding WASM linear memory fragmentation.

use crate::error::WasmError;
use bumpalo::Bump;

/// WASM PDF engine with Arena allocator for efficient memory management.
///
/// The arena is reset after each PDF processing operation, freeing all
/// temporary allocations in bulk. This prevents memory leaks in long-running
/// WASM sessions and reduces fragmentation compared to individual allocations.
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct WasmPdfEngine {
    arena: Bump,
}

#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
impl WasmPdfEngine {
    /// Create a new WASM PDF engine with a fresh arena.
    pub fn new() -> Self {
        Self { arena: Bump::new() }
    }

    /// Create an engine with a pre-allocated arena capacity hint.
    ///
    /// Pre-allocating reduces re-allocation overhead for known workloads.
    pub fn with_capacity(bytes: usize) -> Self {
        Self {
            arena: Bump::with_capacity(bytes),
        }
    }

    /// Reset arena to free all temporary allocations.
    ///
    /// Should be called after processing each PDF to prevent memory leaks
    /// in long-running WASM sessions.
    pub fn reset_arena(&mut self) {
        self.arena.reset();
    }

    /// Get the number of bytes currently allocated in the arena.
    pub fn arena_allocated_bytes(&self) -> usize {
        self.arena.allocated_bytes()
    }

    /// Allocate a slice in the arena and copy data into it.
    ///
    /// Returns a reference into the arena-allocated memory that is valid
    /// until `reset_arena()` is called.
    pub fn alloc_slice_copy<'a>(&'a self, data: &[u8]) -> &'a [u8] {
        self.arena.alloc_slice_copy(data)
    }

    /// Allocate a string in the arena.
    pub fn alloc_str<'a>(&'a self, s: &str) -> &'a str {
        self.arena.alloc_str(s)
    }

    /// Process a PDF buffer using arena-allocated temporary storage.
    ///
    /// This demonstrates the arena pattern: all temporary allocations
    /// happen in the arena, then the result is extracted as an owned String.
    /// Process a PDF buffer using arena-allocated temporary storage.
    ///
    /// This demonstrates the arena pattern: all temporary allocations
    /// happen in the arena, then the result is extracted as an owned String.
    /// In a real implementation, pdfium would write into the arena-allocated
    /// buffer rather than the original `pdf_data` slice.
    pub fn extract_text_with_arena(&mut self, pdf_data: &[u8]) -> Result<String, WasmError> {
        // Arena copy simulates pdfium writing into arena-allocated buffer
        let arena_buffer = self.arena.alloc_slice_copy(pdf_data);

        // Simulate extraction from the arena buffer instead of copying original data.
        // In a real implementation, pdfium would write into the arena buffer directly.
        let extracted = String::from_utf8(arena_buffer.to_vec())
            .map_err(|e| WasmError::ExtractionError(e.to_string()))?;

        // Reset arena to free all temporary allocations in O(1)
        self.reset_arena();

        Ok(extracted)
    }
}

impl Default for WasmPdfEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = WasmPdfEngine::new();
        assert_eq!(engine.arena_allocated_bytes(), 0);
    }

    #[test]
    fn test_engine_with_capacity() {
        let engine = WasmPdfEngine::with_capacity(1024);
        // Engine created successfully with pre-allocated capacity
        // allocated_bytes is usize (always >= 0), just verify it works
        let _ = engine.arena_allocated_bytes();
    }

    #[test]
    fn test_arena_reset() {
        let mut engine = WasmPdfEngine::new();
        let bytes_before = engine.arena_allocated_bytes();
        let _alloc = engine.alloc_slice_copy(&[1, 2, 3, 4, 5]);
        let bytes_after_alloc = engine.arena_allocated_bytes();
        assert!(bytes_after_alloc > bytes_before);
        engine.reset_arena();
        // After reset, new allocations start from the beginning of the buffer
        let bytes_after_reset = engine.arena_allocated_bytes();
        assert!(bytes_after_reset <= bytes_after_alloc);
    }

    #[test]
    fn test_alloc_str() {
        let engine = WasmPdfEngine::new();
        let s = engine.alloc_str("hello world");
        assert_eq!(s, "hello world");
    }

    #[test]
    fn test_extract_text_with_arena() {
        let mut engine = WasmPdfEngine::new();
        let data = b"Hello, WASM!";
        let result = engine.extract_text_with_arena(data).unwrap();
        assert!(!result.is_empty());
    }
}
