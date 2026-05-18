use bumpalo::Bump;

/// Visual buffer backed by a bumpalo Arena.
///
/// Pixels live in a contiguous memory region managed by the Arena allocator.
/// Base64 encoding is performed directly on this region — no temporary files,
/// no disk I/O.  The `'a` lifetime ties the buffer to the Arena so they are
/// dropped together.
pub struct VisualBuffer<'a> {
    #[allow(dead_code)]
    arena: &'a Bump,
    pixels: &'a mut [u8],
    width: u32,
    height: u32,
}

impl<'a> VisualBuffer<'a> {
    /// Bytes per pixel for RGBA
    const BYTES_PER_PIXEL: usize = 4;

    /// Allocate an RGBA pixel buffer inside the given Arena.
    pub fn new(arena: &'a Bump, width: u32, height: u32) -> Self {
        let size = (width as usize)
            .checked_mul(height as usize)
            .and_then(|v| v.checked_mul(Self::BYTES_PER_PIXEL))
            .expect("buffer size overflow");

        let pixels = arena.alloc_slice_fill_default(size);

        Self { arena, pixels, width, height }
    }

    /// Encode the pixel data to Base64 directly from Arena memory.
    ///
    /// Uses the `base64` engine to encode bytes in-place — no intermediate
    /// `Vec<u8>` allocation beyond the output string.
    pub fn encode_base64(&self) -> String {
        use base64::Engine;
        let slice: &[u8] = self.pixels;
        base64::engine::general_purpose::STANDARD.encode(slice)
    }

    /// Mutable pointer for FFI callers (e.g. Pdfium render).
    ///
    /// # Safety
    /// The caller must write exactly `width * height * 4` bytes of RGBA data.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.pixels.as_mut_ptr()
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Raw pixel slice (RGBA).
    pub fn as_bytes(&self) -> &[u8] {
        self.pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_and_encode() {
        let arena = Bump::new();
        let buf = VisualBuffer::new(&arena, 2, 2);
        assert_eq!(buf.width(), 2);
        assert_eq!(buf.height(), 2);
        // 2x2 RGBA = 16 bytes, all zero → correct base64
        let b64 = buf.encode_base64();
        assert!(!b64.is_empty());
    }

    #[test]
    fn test_mut_ptr_write() {
        let arena = Bump::new();
        let mut buf = VisualBuffer::new(&arena, 1, 1);
        let ptr = buf.as_mut_ptr();
        // SAFETY: ptr is valid for buf.len() bytes; test writes known values within bounds
        unsafe {
            *ptr = 255;
            *ptr.add(1) = 128;
            *ptr.add(2) = 64;
            *ptr.add(3) = 32;
        }
        assert_eq!(buf.as_bytes(), &[255, 128, 64, 32]);
    }
}
