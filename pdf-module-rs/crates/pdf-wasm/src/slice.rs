//! Zero-copy slice wrapper for JS-WASM boundary data passing.
//!
//! Avoids serialization overhead by passing raw pointers with length
//! across the JS-WASM boundary. This is critical for large PDF images
//! and extracted text buffers.

use std::marker::PhantomData;

/// Zero-copy slice wrapper for JS-WASM boundary.
///
/// Avoids serialization overhead by passing raw pointers with length.
/// This enables passing large buffers (rendered pages, extracted text)
/// without copying.
///
/// # Safety
///
/// The caller must ensure the pointer and length are valid for the
/// lifetime of the `WasmSlice`. The slice must not be used after
/// the underlying memory is freed.
///
/// Prefer [`OwnedSlice`] for safe ownership management.
#[repr(C)]
pub struct WasmSlice {
    ptr: *const u8,
    len: usize,
    _marker: PhantomData<*const [u8]>,
}

// SAFETY: WasmSlice is designed for controlled JS-WASM interop.
// The caller must ensure thread safety and lifetime validity.
// WASM is single-threaded so Send+Sync is safe in that context.
// On non-WASM targets, raw pointers should NOT be Send+Sync.
#[cfg(target_arch = "wasm32")]
unsafe impl Send for WasmSlice {}

// SAFETY: WasmSlice is designed for controlled JS-WASM interop.
// The caller must ensure thread safety and lifetime validity.
// WASM is single-threaded so Send+Sync is safe in that context.
// On non-WASM targets, raw pointers should NOT be Send+Sync.
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for WasmSlice {}

impl WasmSlice {
    /// Create a new WasmSlice from a pointer and length.
    ///
    /// # Safety
    ///
    /// The caller must ensure `ptr` is valid for `len` bytes and
    /// remains valid for the lifetime of the returned `WasmSlice`.
    /// The underlying memory must not be freed while this slice exists.
    pub unsafe fn new(ptr: *const u8, len: usize) -> Self {
        Self { ptr, len, _marker: PhantomData }
    }

    /// Convert to Rust slice for internal processing.
    ///
    /// # Safety
    ///
    /// Caller must ensure the pointer and length are valid.
    pub unsafe fn as_slice(&self) -> &[u8] {
        // SAFETY: Caller guarantees ptr is valid for len bytes
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    /// Get the raw pointer for JS interop.
    pub fn ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Get the length for JS interop.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// A safe wrapper that owns data and provides a zero-copy [`WasmSlice`] view.
///
/// This prevents dangling pointer issues by keeping the underlying data alive
/// for the entire lifetime of the `OwnedSlice`. The `WasmSlice` returned by
/// [`OwnedSlice::as_wasm_slice`] is only valid while the `OwnedSlice` is alive.
///
/// # Example
///
/// ```
/// use pdf_wasm::slice::OwnedSlice;
///
/// let owned = OwnedSlice::from_vec(vec![1, 2, 3, 4, 5]);
/// let wasm_slice = owned.as_wasm_slice();
/// assert_eq!(wasm_slice.len(), 5);
/// // Data stays alive as long as `owned` is alive
/// ```
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct OwnedSlice {
    data: Vec<u8>,
}

/// Non-`#[wasm_bindgen]` methods — `WasmSlice` and `&[u8]` return types are
/// unsupported by wasm-bindgen.
impl OwnedSlice {
    /// Get a zero-copy view of the data as a WasmSlice.
    ///
    /// The returned slice is valid for the lifetime of this `OwnedSlice`.
    pub fn as_wasm_slice(&self) -> WasmSlice {
        // SAFETY: self.data owns the memory and remains alive for the lifetime
        // of this &self, which outlives the returned WasmSlice.
        // This routes through WasmSlice::new() to keep the safety contract in one place.
        unsafe { WasmSlice::new(self.data.as_ptr(), self.data.len()) }
    }

    /// Get a reference to the underlying data.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
impl OwnedSlice {
    /// Create an OwnedSlice from a byte vector.
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Get the length of the data.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the data is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Consume the OwnedSlice and return the underlying Vec.
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }
}

impl From<Vec<u8>> for OwnedSlice {
    fn from(data: Vec<u8>) -> Self {
        Self::from_vec(data)
    }
}

impl AsRef<[u8]> for OwnedSlice {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_slice_creation() {
        let data = vec![1u8, 2, 3, 4, 5];
        let owned = OwnedSlice::from_vec(data);
        let slice = owned.as_wasm_slice();
        assert_eq!(slice.len(), 5);
        assert!(!slice.is_empty());
    }

    #[test]
    fn test_wasm_slice_empty() {
        let owned = OwnedSlice::from_vec(vec![]);
        let slice = owned.as_wasm_slice();
        assert!(slice.is_empty());
        assert_eq!(slice.len(), 0);
    }

    #[test]
    fn test_wasm_slice_as_slice() {
        let owned = OwnedSlice::from_vec(vec![10, 20, 30]);
        let slice = owned.as_wasm_slice();
        unsafe {
            let inner = slice.as_slice();
            assert_eq!(inner, &[10, 20, 30]);
        }
    }

    #[test]
    fn test_wasm_slice_ptr() {
        let owned = OwnedSlice::from_vec(vec![42u8]);
        let slice = owned.as_wasm_slice();
        assert_eq!(slice.ptr(), owned.data.as_ptr());
    }

    #[test]
    fn test_owned_slice_into_vec() {
        let data = vec![1, 2, 3];
        let owned = OwnedSlice::from_vec(data);
        let recovered = owned.into_vec();
        assert_eq!(recovered, vec![1, 2, 3]);
    }

    #[test]
    fn test_from_trait() {
        let owned: OwnedSlice = vec![1u8, 2, 3].into();
        assert_eq!(owned.len(), 3);
    }

    #[test]
    fn test_as_ref_trait() {
        let owned = OwnedSlice::from_vec(vec![1, 2, 3]);
        let slice: &[u8] = owned.as_ref();
        assert_eq!(slice, &[1, 2, 3]);
    }

    #[test]
    fn test_no_dangling_ptr() {
        // Verify WasmSlice is valid for the lifetime of OwnedSlice.
        // After OwnedSlice is dropped, accessing the WasmSlice would be UB,
        // so we can only test the valid-lifetime case here.
        let owned = OwnedSlice::from_vec(vec![100, 200]);
        let wasm_slice = owned.as_wasm_slice();
        assert_eq!(wasm_slice.ptr(), owned.as_bytes().as_ptr());
        assert_eq!(wasm_slice.len(), 2);
        unsafe {
            assert_eq!(wasm_slice.as_slice(), &[100, 200]);
        }
        // Ensure slice remains valid through multiple reads while owned is alive
        assert_eq!(wasm_slice.len(), 2);
        assert!(!wasm_slice.is_empty());
        drop(owned);
    }
}
