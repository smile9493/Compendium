use tracing::warn;

use crate::types::{DetectorConfig, PdfiumExtraction};

/// Result of the escalation detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionResult {
    /// Normal document — keep local parsing speed.
    Normal,
    /// Zero / low text — trigger VLM OCR.
    ZeroText,
    /// Layout chaos — trigger VLM layout understanding.
    LayoutChaos,
}

/// Conditional escalation detector.
///
/// Two independent probes:
/// 1. **Zero-text** — character count <= threshold
/// 2. **Layout chaos** — confidence < threshold
///
/// The goal is 90 % Normal / 10 % escalated in production.
pub struct EscalationDetector {
    config: DetectorConfig,
}

impl EscalationDetector {
    pub fn new(config: DetectorConfig) -> Self {
        Self::validate_config(&config);
        Self { config }
    }

    /// Inspect a Pdfium extraction and decide whether VLM is needed.
    pub fn detect(&self, extraction: &PdfiumExtraction) -> DetectionResult {
        // Rule 1: zero / low text -> OCR
        if extraction.character_count <= self.config.zero_text_threshold {
            return DetectionResult::ZeroText;
        }

        // Rule 2: layout chaos -> layout understanding
        if extraction.layout_confidence < self.config.layout_confidence_threshold {
            return DetectionResult::LayoutChaos;
        }

        DetectionResult::Normal
    }

    fn validate_config(cfg: &DetectorConfig) {
        if cfg.zero_text_threshold > 100 {
            warn!(
                "zero_text_threshold={} is out of range [0,100], falling back to 0",
                cfg.zero_text_threshold
            );
        }
        if !(0.0..=1.0).contains(&cfg.layout_confidence_threshold) {
            warn!(
                "layout_confidence_threshold={} is out of range [0.0,1.0], falling back to 0.3",
                cfg.layout_confidence_threshold
            );
        }
    }
}

impl Default for EscalationDetector {
    fn default() -> Self {
        Self::new(DetectorConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extraction(char_count: u32, confidence: f32) -> PdfiumExtraction {
        PdfiumExtraction {
            character_count: char_count,
            layout_confidence: confidence,
            text: String::new(),
            page_width: 612.0,
            page_height: 792.0,
        }
    }

    #[test]
    fn zero_text_triggers_ocr() {
        let det = EscalationDetector::default();
        assert_eq!(det.detect(&extraction(0, 0.9)), DetectionResult::ZeroText);
    }

    #[test]
    fn low_text_triggers_ocr() {
        let det = EscalationDetector::new(DetectorConfig {
            zero_text_threshold: 50,
            ..Default::default()
        });
        assert_eq!(det.detect(&extraction(30, 0.9)), DetectionResult::ZeroText);
    }

    #[test]
    fn layout_chaos_triggers_vlm() {
        let det = EscalationDetector::default();
        assert_eq!(
            det.detect(&extraction(200, 0.1)),
            DetectionResult::LayoutChaos
        );
    }

    #[test]
    fn normal_document_stays_local() {
        let det = EscalationDetector::default();
        assert_eq!(det.detect(&extraction(500, 0.9)), DetectionResult::Normal);
    }

    #[test]
    fn boundary_zero_text() {
        let det = EscalationDetector::new(DetectorConfig {
            zero_text_threshold: 50,
            ..Default::default()
        });
        assert_eq!(det.detect(&extraction(50, 0.9)), DetectionResult::ZeroText);
        assert_eq!(det.detect(&extraction(51, 0.9)), DetectionResult::Normal);
    }

    #[test]
    fn boundary_confidence() {
        let det = EscalationDetector::default();
        assert_eq!(det.detect(&extraction(500, 0.3)), DetectionResult::Normal);
        assert_eq!(
            det.detect(&extraction(500, 0.299)),
            DetectionResult::LayoutChaos
        );
    }
}
