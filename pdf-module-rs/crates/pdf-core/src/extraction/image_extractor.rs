//! Standalone image → text via VLM gateway (built-in path before plugins).

#[cfg(feature = "vlm")]
use std::path::Path;
#[cfg(feature = "vlm")]
use std::sync::Arc;

#[cfg(feature = "vlm")]
use crate::error::{PdfModuleError, PdfResult};
#[cfg(feature = "vlm")]
use vlm_visual_gateway::VlmGateway;

/// Maximum raw image size accepted for VLM upload (20 MiB).
#[cfg(feature = "vlm")]
pub const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;

/// MIME type for image files sent to the VLM API.
#[cfg(feature = "vlm")]
pub fn image_mime_from_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/png",
    }
}

/// Describes raster images through the configured VLM HTTP gateway.
#[cfg(feature = "vlm")]
pub struct ImageExtractor {
    gateway: Arc<VlmGateway>,
}

#[cfg(feature = "vlm")]
impl ImageExtractor {
    pub fn new(gateway: Arc<VlmGateway>) -> Self {
        Self { gateway }
    }

    pub async fn describe_path(&self, path: &Path) -> PdfResult<String> {
        let bytes =
            std::fs::read(path).map_err(|e| PdfModuleError::Storage(format!("read image: {e}")))?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(PdfModuleError::Extraction(format!(
                "image exceeds {MAX_IMAGE_BYTES} bytes (got {})",
                bytes.len()
            )));
        }
        let mime = image_mime_from_path(path);
        self.gateway
            .describe_image(&bytes, mime)
            .await
            .map_err(|e| PdfModuleError::Extraction(format!("VLM image describe: {e}")))
    }
}
