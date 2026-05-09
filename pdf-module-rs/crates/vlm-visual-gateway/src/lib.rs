//! # vlm-visual-gateway
//!
//! VLM Visual Gateway: conditional escalation from Pdfium local parsing
//! to remote VLM layout understanding.
//!
//! ## Architecture
//!
//! ```text
//! MCP Call -> PDF Pipeline -> Pdfium (local) -> EscalationDetector
//!                                               |
//!                            +------ Normal -----+----- Escalate ----+
//!                            |                                       |
//!                      Return local                          VlmGateway -> VLM API
//!                      result fast                           Return enhanced result
//! ```
//!
//! ## Key components
//!
//! - [`VlmGateway`] — async VLM client with timeout + degradation
//! - [`VlmGatewayHandle`] — graceful shutdown for in-flight requests
//! - [`VisualBuffer`] — bumpalo Arena-backed pixel buffer (zero temp files)
//! - [`EscalationDetector`] — zero-text + layout-chaos probes
//! - [`PdfiumGuard`] — FFI panic isolation + serialisation
//! - [`MetricsCollector`] — Prometheus metrics (requests, duration, degradations)

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(clippy::all)]
#![warn(clippy::await_holding_lock)]
#![warn(clippy::await_holding_refcell_ref)]
#![warn(clippy::large_stack_frames)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]
#![cfg_attr(not(test), warn(clippy::unwrap_used))]

pub mod buffer;
pub mod detector;
pub mod error;
pub mod gateway;
pub mod metrics;
pub mod pdfium_guard;
pub mod types;

pub use buffer::VisualBuffer;
pub use detector::{DetectionResult, EscalationDetector};
pub use error::{PdfiumGuardError, PdfiumGuardResult, VlmError, VlmResult};
pub use gateway::{render_page_pixels, VlmGateway, VlmGatewayHandle};
pub use metrics::{MetricsCollector, RequestTimer};
pub use pdfium_guard::{catch_pdfium, PdfiumGuard};
pub use types::{
    LayoutResult, PageComplexity, PayloadMetadata, PdfiumExtraction, Region, RegionType, VlmConfig,
    VlmModel,
};
