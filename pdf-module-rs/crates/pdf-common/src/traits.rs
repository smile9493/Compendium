//! Core trait definitions.
//!
//! Provides unified interface abstractions shared across the workspace.

use crate::dto::{ToolContext, ToolExecutionOptions};
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// File metadata returned by storage backends.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: std::time::SystemTime,
    pub is_file: bool,
}

/// Tool definition for registration.
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool message for streaming.
#[derive(Debug, Clone)]
pub struct ToolMessage {
    pub tool_name: String,
    pub execution_id: String,
    pub content: serde_json::Value,
}

/// File storage trait.
#[async_trait]
pub trait FileStorage: Send + Sync {
    /// Read a file and return its contents.
    async fn read(&self, path: &str) -> Result<Vec<u8>>;

    /// Write data to a file.
    async fn write(&self, path: &str, data: &[u8]) -> Result<()>;

    /// Check whether a file exists.
    async fn exists(&self, path: &str) -> Result<bool>;

    /// Retrieve file metadata.
    async fn metadata(&self, path: &str) -> Result<FileMetadata>;
}

/// Plugin registry trait.
#[async_trait]
pub trait PluginRegistry: Send + Sync {
    /// Register a tool handler.
    async fn register(&mut self, name: String, handler: Arc<dyn ToolHandler>) -> Result<()>;

    /// Unregister a tool by name.
    async fn unregister(&mut self, tool_name: &str) -> Result<()>;

    /// Look up a tool handler by name.
    fn get(&self, tool_name: &str) -> Option<Arc<dyn ToolHandler>>;

    /// List all registered tool definitions.
    fn list(&self) -> Vec<ToolDefinition>;
}

/// Generic tool handler trait.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Execute the tool with given context and parameters.
    async fn execute(
        &self,
        context: ToolContext,
        params: serde_json::Value,
        options: ToolExecutionOptions,
    ) -> Result<serde_json::Value>;

    /// Return the tool definition / metadata.
    fn metadata(&self) -> ToolDefinition;

    /// Health check (default: healthy).
    fn health_check(&self) -> bool {
        true
    }
}

/// Message streamer trait.
#[async_trait]
pub trait MessageStreamer: Send + Sync {
    /// Send a message.
    async fn stream(&self, message: ToolMessage) -> Result<()>;
}
