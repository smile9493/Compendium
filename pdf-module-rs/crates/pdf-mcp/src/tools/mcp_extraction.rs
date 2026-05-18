//! Extraction envelope helpers for MCP tool outputs.

use pdf_core::quality_probe::ExtractionMethod;
use pdf_mcp_contracts::ExtractionEnvelope;

use crate::tools::ToolContext;

pub fn envelope_from_router(
    ctx: &ToolContext,
    file_path: &std::path::Path,
    fallback_used: bool,
) -> anyhow::Result<ExtractionEnvelope> {
    let (backend_id, method) =
        ctx.pipeline.extraction_router().select_backend_id(file_path)?;
    Ok(envelope_from_parts(backend_id, method, fallback_used, None, None))
}

pub fn envelope_from_parts(
    backend_id: String,
    method: ExtractionMethod,
    fallback_used: bool,
    quality_score: Option<f64>,
    needs_vlm: Option<bool>,
) -> ExtractionEnvelope {
    let method_str = match method {
        ExtractionMethod::Pdfium => "pdfium",
        ExtractionMethod::Vlm => "vlm",
        ExtractionMethod::Hybrid => "hybrid",
    };
    ExtractionEnvelope {
        backend_id,
        method: method_str.to_string(),
        fallback_used,
        quality_score,
        needs_vlm,
    }
}

pub fn extraction_health_from_ctx(ctx: &ToolContext) -> pdf_mcp_contracts::ExtractionHealth {
    let backends: Vec<String> = ctx
        .pipeline
        .extraction_router()
        .backend_ids()
        .into_iter()
        .map(String::from)
        .collect();
    let vlm_configured = std::env::var("VLM_GATEWAY_URL").is_ok()
        || std::env::var("RSUT_VLM_ENDPOINT").is_ok();
    pdf_mcp_contracts::ExtractionHealth {
        backends,
        vlm_configured,
        default_method: "pdfium".to_string(),
    }
}
