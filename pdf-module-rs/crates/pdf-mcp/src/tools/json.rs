//! JSON tool result helpers.

use crate::protocol::Content;
use serde::Serialize;

pub fn json_content<T: Serialize>(value: &T) -> anyhow::Result<Vec<Content>> {
    let text = serde_json::to_string_pretty(value)?;
    Ok(vec![Content::text(text)])
}

pub fn parse_args<T: serde::de::DeserializeOwned>(args: &serde_json::Value) -> anyhow::Result<T> {
    serde_json::from_value(args.clone()).map_err(|e| anyhow::anyhow!("Invalid tool params: {e}"))
}

/// Wrap arbitrary handler JSON in the standard `{ "result": ... }` output envelope.
#[allow(dead_code)]
pub fn result_output(value: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "result": value })
}
