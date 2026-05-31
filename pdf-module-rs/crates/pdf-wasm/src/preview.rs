//! Lightweight PDF preview helpers (page count, placeholder thumbnails).
//!
//! Full Pdfium-in-WASM is optional; this module parses PDF structure for
//! page count and generates preview buffers without native FFI.
//!
//! ## gen keyword evaluation (Rust 2024)
//!
//! The `gen` keyword (generator blocks) is experimental as of Rust 1.87.
//! `extract_page_text` already uses iterator chains which provide similar
//! benefits — lazy evaluation and minimal intermediate allocations. When
//! `gen` stabilizes, the inner closure could become a `gen fn` yielding
//! lines without materializing the final `String` eagerly.

use std::borrow::Cow;

/// Count pages by scanning for `/Type /Page` markers (heuristic).
///
/// Uses `memchr`-style windowed comparison for efficient byte scanning.
pub fn count_pages(pdf_data: &[u8]) -> usize {
    if pdf_data.is_empty() {
        return 0;
    }
    let needle = b"/Type /Page";
    let mut count = 0usize;
    let mut i = 0usize;
    while i + needle.len() <= pdf_data.len() {
        if &pdf_data[i..i + needle.len()] == needle {
            count += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    count.max(1)
}

/// Iterator that yields extracted text fragments for a specific page.
///
/// Avoids allocating the full `String` upfront — callers can `.collect()`
/// or process fragments lazily.
struct PageTextIter<'a> {
    lines: std::str::Lines<'a>,
    marker: &'a str,
    target_page: u32,
    current_page: u32,
    done: bool,
}

impl<'a> PageTextIter<'a> {
    fn new(text: &'a str, marker: &'a str, target_page: u32) -> Self {
        Self { lines: text.lines(), marker, target_page, current_page: 0, done: false }
    }
}

impl<'a> Iterator for PageTextIter<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        loop {
            let line = self.lines.next()?;
            if line.contains(self.marker) {
                if self.current_page == self.target_page {
                    return Some(Cow::Borrowed(line));
                }
                self.current_page += 1;
            } else if self.current_page == self.target_page + 1 {
                self.done = true;
                return None;
            } else if self.current_page == self.target_page {
                let trimmed = line.trim();
                if trimmed.starts_with('(') && trimmed.ends_with(')') {
                    return Some(Cow::Owned(
                        trimmed.trim_matches(|c| c == '(' || c == ')').to_string(),
                    ));
                }
            }
        }
    }
}

/// Extract rough plain text from a specific page (stream contents between markers).
///
/// Uses an iterator-based approach to avoid materializing intermediate allocations.
/// Only the final output string is allocated.
pub fn extract_page_text(pdf_data: &[u8], page_index: u32) -> String {
    let pages = count_pages(pdf_data);
    if page_index as usize >= pages {
        return String::new();
    }
    let text = String::from_utf8_lossy(pdf_data);
    let iter = PageTextIter::new(&text, "/Page", page_index);

    // Pre-allocate with a reasonable estimate to reduce reallocations.
    let mut out = String::with_capacity(256);
    for fragment in iter {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&fragment);
    }
    out.truncate(out.trim_end().len());
    out
}

/// Generate a small RGBA thumbnail placeholder for a page (IRON-02 friendly fixed size).
pub fn render_page_thumbnail_rgba(page_index: u32, max_px: u32) -> Vec<u8> {
    let size = max_px.clamp(32, 512);
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let hue = page_index.wrapping_mul(97) % 255;
    for y in 0..size {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            rgba[i] = hue as u8;
            rgba[i + 1] = (x * 255 / size.max(1)) as u8;
            rgba[i + 2] = (y * 255 / size.max(1)) as u8;
            rgba[i + 3] = 255;
        }
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_pages_empty() {
        assert_eq!(count_pages(b""), 0);
    }

    #[test]
    fn test_count_pages_single() {
        assert_eq!(count_pages(b"/Type /Page some content"), 1);
    }

    #[test]
    fn test_count_pages_multiple() {
        let data = b"header\n/Type /Page\ncontent\n/Type /Page\nmore";
        assert_eq!(count_pages(data), 2);
    }

    #[test]
    fn test_extract_page_text_out_of_range() {
        assert_eq!(extract_page_text(b"no pages here", 5), "");
    }

    #[test]
    fn test_extract_page_text_basic() {
        let data = b"/Type /Page\n(Hello World)\n/Type /Page";
        let text = extract_page_text(data, 0);
        assert!(text.contains("Hello World"));
    }

    #[test]
    fn test_render_thumbnail_size_clamped() {
        let rgba = render_page_thumbnail_rgba(0, 9999);
        // Clamped to 512
        assert_eq!(rgba.len(), 512 * 512 * 4);
    }

    #[test]
    fn test_render_thumbnail_min_size() {
        let rgba = render_page_thumbnail_rgba(0, 1);
        // Clamped to 32
        assert_eq!(rgba.len(), 32 * 32 * 4);
    }
}
