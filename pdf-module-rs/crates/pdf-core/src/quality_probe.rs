//! Quality probe — extractor confidence scoring and fallback recommendations.
//!
//! Analyzes text density, font/logo presence, and PDFium extraction quality
//! to produce a `QualityReport`. Low-confidence reports trigger VLM OCR fallback.
//!
//! ## Confidence thresholds
//! - `≥0.95`: High — text extraction is reliable
//! - `≥0.90`: Medium — may have minor issues
//! - `<0.70`: Low — scanned PDF, recommend OCR fallback

use std::panic::catch_unwind;

use crate::engine::PdfiumEngine;
use crate::error::{PdfModuleError, PdfResult};
use crate::mmap_loader::PdfQuality;

const CONFIDENCE_HIGH: f64 = 0.95;
const CONFIDENCE_MEDIUM: f64 = 0.90;
const CONFIDENCE_LOW: f64 = 0.70;

const TEXT_DENSITY_THRESHOLD: f64 = 0.3;
const TEXT_DENSITY_SCANNED_THRESHOLD: f64 = 0.05;

/// Result of PDF quality analysis — confidence score and extraction quality metadata.
///
/// Low-confidence reports (< 0.70) indicate scanned/image-based PDFs that
/// should use VLM OCR instead of text extraction.
#[derive(Debug, Clone)]
pub struct QualityReport {
    pub quality: PdfQuality,
    pub text_density: f64,
    pub has_fonts: bool,
    pub has_images: bool,
    pub confidence: f64,
    pub needs_vlm: bool,
    pub extraction_method: ExtractionMethod,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExtractionMethod {
    Pdfium,
    Vlm,
    Hybrid,
}

impl QualityReport {
    pub fn needs_vlm_enhancement(&self) -> bool {
        matches!(self.quality, PdfQuality::Scanned | PdfQuality::LowQuality)
    }
}

pub struct QualityProbe;

impl QualityProbe {
    pub fn analyze(data: &[u8]) -> PdfResult<QualityReport> {
        if data.len() < 4 || &data[0..4] != b"%PDF" {
            return Ok(QualityReport {
                quality: PdfQuality::Invalid,
                text_density: 0.0,
                has_fonts: false,
                has_images: false,
                confidence: 1.0,
                needs_vlm: false,
                extraction_method: ExtractionMethod::Pdfium,
            });
        }

        let (text_density, has_fonts, has_images) = Self::analyze_content(data);

        let quality = Self::determine_quality(text_density, has_fonts, has_images);

        let (needs_vlm, extraction_method) = match quality {
            PdfQuality::Scanned => (true, ExtractionMethod::Vlm),
            PdfQuality::LowQuality => (true, ExtractionMethod::Hybrid),
            PdfQuality::Digital => (false, ExtractionMethod::Pdfium),
            PdfQuality::Unknown => (false, ExtractionMethod::Pdfium),
            PdfQuality::Invalid => (false, ExtractionMethod::Pdfium),
        };

        let confidence = if has_fonts && text_density > TEXT_DENSITY_THRESHOLD {
            CONFIDENCE_HIGH
        } else if has_images && !has_fonts {
            CONFIDENCE_MEDIUM
        } else {
            CONFIDENCE_LOW
        };

        Ok(QualityReport {
            quality,
            text_density,
            has_fonts,
            has_images,
            confidence,
            needs_vlm,
            extraction_method,
        })
    }

    fn analyze_content(data: &[u8]) -> (f64, bool, bool) {
        let sample_size = 2048.min(data.len());
        let sample = &data[0..sample_size];

        let text_chars =
            sample.iter().filter(|&&b| b.is_ascii_graphic() || b.is_ascii_whitespace()).count();
        let text_density = text_chars as f64 / sample_size as f64;

        let sample_str = String::from_utf8_lossy(sample);
        let has_fonts = sample_str.contains("/Font")
            || sample_str.contains("/Type0")
            || sample_str.contains("/CIDFont");

        let has_images = sample_str.contains("/Image")
            || sample_str.contains("/XObject")
            || sample_str.contains("/DCTDecode")
            || sample_str.contains("/FlateDecode");

        (text_density, has_fonts, has_images)
    }

    fn determine_quality(text_density: f64, has_fonts: bool, has_images: bool) -> PdfQuality {
        if !has_fonts && has_images {
            PdfQuality::Scanned
        } else if text_density < 0.1 && has_images {
            PdfQuality::LowQuality
        } else if has_fonts {
            PdfQuality::Digital
        } else {
            PdfQuality::Unknown
        }
    }

    pub fn probe_with_pdfium(data: &[u8]) -> PdfResult<QualityReport> {
        let mut report = Self::analyze(data)?;

        if report.quality.is_extractable() {
            let pdfium_text_density = Self::probe_text_density_via_pdfium(data)?;
            report.text_density = pdfium_text_density;

            if pdfium_text_density < TEXT_DENSITY_SCANNED_THRESHOLD && report.has_images {
                report.quality = PdfQuality::Scanned;
                report.needs_vlm = true;
                report.extraction_method = ExtractionMethod::Vlm;
                report.confidence = CONFIDENCE_HIGH;
            }
        }

        Ok(report)
    }

    fn probe_text_density_via_pdfium(data: &[u8]) -> PdfResult<f64> {
        let pdfium = match PdfiumEngine::get_pdfium() {
            Ok(p) => p,
            Err(_) => return Ok(0.0f64),
        };

        let result: Result<f64, PdfModuleError> =
            // SAFETY: AssertUnwindSafe is used around Pdfium FFI calls for panic containment.
            // The Pdfium library state behind LazyLock<Option<Arc<Pdfium>>> is captured by
            // the closure via `pdfium`. If a panic corrupts Pdfium internal state, subsequent
            // calls are unsound - but this is an accepted trade-off (rare Pdfium panics,
            // no in-process state survives, container restart handles corruption).
            // See engine/pdfium.rs for full rationale.
            catch_unwind(std::panic::AssertUnwindSafe(|| {
                let document = match pdfium.load_pdf_from_byte_slice(data, None) {
                    Ok(doc) => doc,
                    Err(_) => return Ok(0.0f64),
                };

                let pages = document.pages();
                if pages.is_empty() {
                    return Ok(0.0f64);
                }

                let sample_pages = 3.min(pages.len());
                let mut total_chars = 0usize;
                let mut total_area = 0.0f64;

                for i in 0..sample_pages {
                    if let Ok(page) = pages.get(i) {
                        if let Ok(text) = page.text() {
                            total_chars += text.all().chars().count();
                        }
                        let width = page.width().value as f64;
                        let height = page.height().value as f64;
                        total_area += width * height;
                    }
                }

                if total_area > 0.0 {
                    let density = (total_chars as f64) / (total_area / 1000.0);
                    Ok((density / 10.0).min(1.0))
                } else {
                    Ok(0.0f64)
                }
            }))
            .map_err(|_| PdfModuleError::Extraction("Pdfium panic in quality probe".to_string()))?;

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_quality_probe_invalid() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Not a PDF").unwrap();

        let data = std::fs::read(temp_file.path()).unwrap();
        let report = QualityProbe::analyze(&data).unwrap();

        assert_eq!(report.quality, PdfQuality::Invalid);
        assert!(!report.needs_vlm);
    }

    #[test]
    fn test_quality_probe_pdf_header() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"%PDF-1.4\n%test content with /Font and /Image").unwrap();

        let data = std::fs::read(temp_file.path()).unwrap();
        let report = QualityProbe::analyze(&data).unwrap();

        assert_ne!(report.quality, PdfQuality::Invalid);
        assert!(report.has_fonts);
        assert!(report.has_images);
    }

    #[test]
    fn test_determine_quality_scanned() {
        let quality = QualityProbe::determine_quality(0.05, false, true);
        assert_eq!(quality, PdfQuality::Scanned);
    }

    #[test]
    fn test_determine_quality_digital() {
        let quality = QualityProbe::determine_quality(0.5, true, false);
        assert_eq!(quality, PdfQuality::Digital);
    }
}
