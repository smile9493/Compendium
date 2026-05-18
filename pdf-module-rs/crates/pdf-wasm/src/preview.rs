//! Lightweight PDF preview helpers (page count, placeholder thumbnails).
//!
//! Full Pdfium-in-WASM is optional; this module parses PDF structure for
//! page count and generates preview buffers without native FFI.

/// Count pages by scanning for `/Type /Page` markers (heuristic).
pub fn count_pages(pdf_data: &[u8]) -> usize {
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

/// Extract rough plain text from a page range (stream contents between markers).
pub fn extract_page_text(pdf_data: &[u8], page_index: u32) -> String {
    let pages = count_pages(pdf_data);
    if page_index as usize >= pages {
        return String::new();
    }
    let marker = "/Page";
    let mut found = 0u32;
    let mut out = String::new();
    let text = String::from_utf8_lossy(pdf_data);
    for line in text.lines() {
        if line.contains(&marker) {
            if found == page_index {
                out.push_str(line);
                out.push('\n');
            }
            found += 1;
        } else if found == page_index + 1 {
            break;
        } else if found == page_index {
            let trimmed = line.trim();
            if trimmed.starts_with('(') && trimmed.ends_with(')') {
                out.push_str(trimmed.trim_matches(|c| c == '(' || c == ')'));
                out.push(' ');
            }
        }
    }
    out.trim().to_string()
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
