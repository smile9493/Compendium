//! Tool registry and manifest hashing.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::schema_for;

#[derive(Debug, Clone, Serialize)]
pub struct McpToolSpec {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(rename = "outputSchema")]
    pub output_schema: Value,
}

impl McpToolSpec {
    pub fn new<I: schemars::JsonSchema, O: schemars::JsonSchema>(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: schema_for::<I>(),
            output_schema: schema_for::<O>(),
        }
    }
}

pub fn all_tool_specs() -> Vec<McpToolSpec> {
    let mut tools = Vec::with_capacity(50);
    tools.extend(crate::extract::tool_specs());
    tools.extend(crate::knowledge::tool_specs());
    tools.extend(crate::index::tool_specs());
    tools.extend(crate::management::tool_specs());
    tools.extend(crate::platform::tool_specs());
    tools
}

pub fn tool_count() -> usize {
    all_tool_specs().len()
}

pub fn manifest_sha256() -> String {
    let specs = all_tool_specs();
    let mut names: Vec<_> = specs.iter().map(|t| t.name.as_str()).collect();
    names.sort_unstable();
    let manifest: Vec<_> = names
        .iter()
        .filter_map(|name| specs.iter().find(|t| t.name == *name))
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "inputSchema": t.input_schema,
                "outputSchema": t.output_schema,
            })
        })
        .collect();
    let bytes = serde_json::to_vec(&manifest).expect("manifest JSON");
    let hash = Sha256::digest(bytes);
    hex::encode(hash)
}
