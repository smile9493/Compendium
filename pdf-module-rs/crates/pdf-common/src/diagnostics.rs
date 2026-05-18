//! Rich diagnostic error reporting via `miette`.
//!
//! This module provides integration with the [`miette`] crate for
//! structured, user-friendly error reports with:
//!
//! - Error codes (e.g. `E001`, `E404`)
//! - Help text with actionable suggestions
//! - Source code snippets (when span info is available)
//! - Related error chains
//!
//! # Feature flag
//!
//! Enable with `features = ["diagnostics"]` in `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! pdf-common = { workspace = true, features = ["diagnostics"] }
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use pdf_common::diagnostics::DiagnosticExt;
//!
//! let err = PdfError::FileNotFound("/tmp/missing.pdf".into());
//! let report = err.into_diagnostic("/tmp/missing.pdf".into());
//! eprintln!("{:?}", report);
//! ```
//!
//! # Global error handler
//!
//! Call `diagnostics::install_handler()` early in `main()` to enable
//! pretty-printed error reports for all `anyhow::Error` / `miette::Report`
//! in the application.

use miette::Diagnostic;

/// Extension trait to convert `PdfError` into a `miette::Report` with
/// rich diagnostic information.
///
/// Automatically populates error codes, help text, and severity from
/// the error variant.
pub trait DiagnosticExt {
    fn into_diagnostic(self, source_label: String) -> miette::Report;
    fn code(&self) -> &'static str;
    fn help(&self) -> Option<&'static str>;
    fn severity(&self) -> miette::Severity;
}

#[derive(Debug, Diagnostic, thiserror::Error)]
#[error("{message}")]
#[diagnostic(
    code({error_code}),
    help("{help_text}"),
    severity({severity_level})
)]
struct PdfDiagnostic {
    message: String,
    error_code: String,
    help_text: String,
    severity_level: String,

    #[source_code]
    src: miette::NamedSource<String>,

    #[label]
    span: Option<miette::SourceSpan>,
}

fn error_code_for(err: &crate::PdfError) -> &'static str {
    match err {
        crate::PdfError::FileNotFound(_) => "E404",
        crate::PdfError::InvalidFileType(_) => "E400",
        crate::PdfError::FileTooLarge(_) => "E413",
        crate::PdfError::CorruptedFile(_) => "E422",
        crate::PdfError::Extraction(_) => "E500",
        crate::PdfError::AdapterNotFound(_) => "E501",
        crate::PdfError::ToolRegistration(_) => "E502",
        crate::PdfError::ToolExecution(_) => "E503",
        crate::PdfError::ToolNotFound(_) => "E504",
        crate::PdfError::ToolAlreadyRegistered(_) => "E409",
        crate::PdfError::InvalidToolDefinition(_) => "E505",
        crate::PdfError::PluginLoad(_) => "E506",
        crate::PdfError::ToolUnavailable(_) => "E507",
        crate::PdfError::Discovery(_) => "E508",
        crate::PdfError::Timeout(_) => "E408",
        crate::PdfError::Validation(_) => "E422",
        crate::PdfError::SchemaValidation(_) => "E422",
        crate::PdfError::Config(_) => "E509",
        crate::PdfError::Storage(_) => "E510",
        crate::PdfError::Audit(_) => "E511",
        crate::PdfError::Http(_) => "E512",
        crate::PdfError::Database(_) => "E513",
        crate::PdfError::LLM(_) => "E514",
        crate::PdfError::ParameterMissing(_) => "E400",
        crate::PdfError::ParameterType(_) => "E400",
        crate::PdfError::Io(_) => "E515",
        crate::PdfError::Json(_) => "E516",
        crate::PdfError::Unknown(_) => "E999",
    }
}

fn help_text_for(err: &crate::PdfError) -> Option<&'static str> {
    match err {
        crate::PdfError::FileNotFound(_) => {
            Some("Verify the file path exists and is accessible. Check for typos in the path.")
        }
        crate::PdfError::InvalidFileType(_) => {
            Some("Ensure the file is a valid PDF. Accepted MIME type: application/pdf.")
        }
        crate::PdfError::FileTooLarge(_) => {
            Some("Reduce the file size or increase the max_file_size_mb configuration.")
        }
        crate::PdfError::CorruptedFile(_) => {
            Some("The PDF appears malformed. Try re-saving from the source application.")
        }
        crate::PdfError::Extraction(_) => {
            Some("Extraction failed. Ensure pdfium is available and the PDF is not password-protected.")
        }
        crate::PdfError::Timeout(ms) if *ms > 30_000 => {
            Some("Operation timed out. Consider increasing the timeout or processing in smaller batches.")
        }
        crate::PdfError::Config(key) => {
            Some("Check your configuration. Use `pdf-cli config show` to inspect current values.")
        }
        crate::PdfError::ParameterMissing(name) => {
            Some("This parameter is required. Provide it via CLI flag, env var, or config file.")
        }
        _ => None,
    }
}

fn severity_for(err: &crate::PdfError) -> miette::Severity {
    match err.category() {
        crate::error::ErrorCategory::FileSystem | crate::error::ErrorCategory::Validation => {
            miette::Severity::Warning
        }
        crate::error::ErrorCategory::Extraction
        | crate::error::ErrorCategory::Plugin
        | crate::error::ErrorCategory::Network
        | crate::error::ErrorCategory::Database
        | crate::error::ErrorCategory::LLM
        | crate::error::ErrorCategory::Config => miette::Severity::Error,
    }
}

impl DiagnosticExt for crate::PdfError {
    fn into_diagnostic(self, source_label: String) -> miette::Report {
        let code = error_code_for(&self);
        let help = help_text_for(&self).unwrap_or("See documentation for details.");
        let severity = severity_for(&self);
        let message = self.to_string();

        let severity_str = match severity {
            miette::Severity::Error => "Error",
            miette::Severity::Warning => "Warning",
            miette::Severity::Advice => "Advice",
        };

        let diag = PdfDiagnostic {
            message,
            error_code: code.to_string(),
            help_text: help.to_string(),
            severity_level: severity_str.to_string(),
            src: miette::NamedSource::new(source_label, String::new()),
            span: None,
        };

        diag.into()
    }

    fn code(&self) -> &'static str {
        error_code_for(self)
    }

    fn help(&self) -> Option<&'static str> {
        help_text_for(self)
    }

    fn severity(&self) -> miette::Severity {
        severity_for(self)
    }
}

/// Install miette's `ReportHandler` as the global error hook.
///
/// Call this early in `main()` to enable pretty-printed error diagnostics
/// everywhere `anyhow::Error` or `miette::Report` is used.
///
/// ```ignore
/// fn main() -> anyhow::Result<()> {
///     pdf_common::diagnostics::install_handler();
///     // ... rest of app
/// }
/// ```
pub fn install_handler() {
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .unicode(true)
                .context_lines(3)
                .tab_width(4)
                .build(),
        )
    }))
    .expect("failed to install miette error hook");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PdfError;

    #[test]
    fn file_not_found_has_code_e404() {
        let err = PdfError::FileNotFound("test.pdf".into());
        assert_eq!(err.code(), "E404");
        assert!(err.help().is_some());
    }

    #[test]
    fn extraction_error_is_severe() {
        let err = PdfError::Extraction("parse failure".into());
        assert_eq!(err.code(), "E500");
        assert!(matches!(err.severity(), miette::Severity::Error));
    }

    #[test]
    fn timeout_has_code_e408() {
        let err = PdfError::Timeout(5000);
        assert_eq!(err.code(), "E408");
    }

    #[test]
    fn into_diagnostic_works() {
        let err = PdfError::FileNotFound("/tmp/missing.pdf".into());
        let report = err.into_diagnostic("test_input".into());
        let rendered = format!("{:?}", report);
        assert!(rendered.contains("E404"));
        assert!(rendered.contains("missing.pdf"));
    }
}
