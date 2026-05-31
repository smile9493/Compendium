//! Unit tests for pdf-wasm preview functionality.
//! These tests run on native target (not wasm32).

use pdf_wasm::preview;

#[test]
fn test_count_pages_empty_data() {
    // An empty slice should return 0 pages
    assert_eq!(preview::count_pages(&[]), 0);
}

#[test]
fn test_count_pages_minimal_pdf() {
    // A minimal PDF with one /Type /Page marker
    let data = b"%PDF-1.4\n1 0 obj\n<< /Type /Page >>\nendobj\n%%EOF";
    let count = preview::count_pages(data);
    assert!(count >= 1, "Should detect at least 1 page, got {}", count);
}

#[test]
fn test_extract_page_text_empty() {
    let text = preview::extract_page_text(&[], 0);
    assert!(text.is_empty());
}
