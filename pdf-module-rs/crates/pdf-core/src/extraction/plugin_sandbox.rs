//! WASM-based plugin sandbox for untrusted extraction plugins.
//!
//! Provides process isolation for third-party PDF extraction plugins
//! using WebAssembly as a sandboxing boundary. Plugins receive raw PDF
//! bytes and return extracted text, with no filesystem or network access.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     ┌──────────────┐     ┌──────────────┐
//! │ Extraction   │────>│ WasmRuntime   │────>│ Plugin WASM  │
//! │ Router       │     │ (wasmtime)    │     │ Module       │
//! └─────────────┘     └──────────────┘     └──────────────┘
//! ```
//!
//! DEVIATION: Using trait-based abstraction over concrete WASM runtime
//! to allow swapping wasmtime/wasmer without changing caller code.

use crate::error::{PdfModuleError, PdfResult};

/// Trait for sandboxed plugin execution.
///
/// Implementors provide WASM-based isolation for untrusted code.
/// The host retains full control over memory limits, fuel/consumption
/// limits, and available imports.
pub trait PluginSandbox: Send + Sync {
    /// Execute a plugin with the given PDF data and return extracted text.
    ///
    /// The implementation should enforce:
    /// - Memory limit (default 64MB)
    /// - Execution fuel limit (prevents infinite loops)
    /// - No filesystem or network imports
    fn execute(&self, plugin_name: &str, pdf_data: &[u8]) -> PdfResult<String>;

    /// List available plugins in the sandbox.
    fn list_plugins(&self) -> Vec<PluginInfo>;

    /// Load a plugin WASM module from bytes.
    fn load_plugin(&mut self, name: &str, wasm_bytes: &[u8]) -> PdfResult<()>;

    /// Unload a plugin and free its resources.
    fn unload_plugin(&mut self, name: &str) -> PdfResult<()>;
}

/// Metadata about a loaded plugin.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    /// Memory usage in bytes.
    pub memory_bytes: usize,
    /// Whether the plugin is currently loaded and ready.
    pub loaded: bool,
}

/// Configuration for the plugin sandbox.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SandboxConfig {
    /// Maximum memory per plugin in bytes (default: 64MB).
    pub max_memory_bytes: usize,
    /// Maximum execution fuel (instruction count limit).
    pub max_fuel: u64,
    /// Directory to load plugin WASM files from.
    pub plugin_dir: Option<std::path::PathBuf>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            max_fuel: 1_000_000_000,            // 1B instructions
            plugin_dir: None,
        }
    }
}

/// Stub implementation that always returns an error.
///
/// Replace with actual wasmtime/wasmer implementation when the dependency
/// is added to the workspace.
pub struct StubSandbox {
    #[allow(dead_code)]
    config: SandboxConfig,
}

impl StubSandbox {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }
}

impl PluginSandbox for StubSandbox {
    fn execute(&self, plugin_name: &str, _pdf_data: &[u8]) -> PdfResult<String> {
        Err(PdfModuleError::Extraction(format!(
            "WASM plugin sandbox not yet implemented; cannot execute plugin '{plugin_name}'"
        )))
    }

    fn list_plugins(&self) -> Vec<PluginInfo> {
        Vec::new()
    }

    fn load_plugin(&mut self, _name: &str, _wasm_bytes: &[u8]) -> PdfResult<()> {
        Err(PdfModuleError::Extraction("WASM plugin sandbox not yet implemented".to_string()))
    }

    fn unload_plugin(&mut self, _name: &str) -> PdfResult<()> {
        Ok(())
    }
}
