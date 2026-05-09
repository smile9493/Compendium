//! Error types for PDF module — now unified via pdf-common.
//!
//! This file re-exports the unified error type from pdf-common.
//! Previous `PdfModuleError` enum has been consolidated into `pdf_common::PdfError`.

pub use pdf_common::PdfError;
pub use pdf_common::Result;

pub type PdfModuleError = pdf_common::PdfError;
pub type PdfResult<T> = pdf_common::Result<T>;
