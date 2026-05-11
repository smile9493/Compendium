//! # Output Formatting
//!
//! Unified output formatting — structured JSON (for AI clients) or human-readable text.
#![allow(dead_code)]

use serde_json::Value;

/// Output mode for command results
#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Pretty-printed human-readable text
    Text,
    /// Structured JSON output
    Json,
}

/// Print a command result according to the output format.
pub fn print_result(format: OutputFormat, label: &str, value: &Value) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
        }
        OutputFormat::Text => {
            println!("── {} ──", label);
            print_value(value, 0);
        }
    }
}

/// Print an informational message (always text, even in JSON mode).
pub fn print_info(msg: impl AsRef<str>) {
    eprintln!("{}", msg.as_ref());
}

/// Print an error message to stderr.
pub fn print_error(msg: impl AsRef<str>) {
    eprintln!("Error: {}", msg.as_ref());
}

/// Pretty-print a JSON value with indentation for text mode.
fn print_value(value: &Value, depth: usize) {
    let indent = "  ".repeat(depth);
    match value {
        Value::Null => println!("{}(null)", indent),
        Value::Bool(b) => println!("{}{}", indent, b),
        Value::Number(n) => println!("{}{}", indent, n),
        Value::String(s) => {
            if depth == 0 {
                // Top-level string: print directly
                println!("{}", s);
            } else {
                println!("{}{}", indent, s);
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                println!("{}[]", indent);
                return;
            }
            for (i, v) in arr.iter().enumerate() {
                if depth == 0 {
                    print!("{}{}. ", indent, i + 1);
                } else {
                    println!("{}-", indent);
                }
                print_value(v, depth + 1);
            }
        }
        Value::Object(map) => {
            if map.is_empty() {
                println!("{{{}}}", indent);
                return;
            }
            for (k, v) in map.iter() {
                match v {
                    Value::Null => {
                        println!("{}{}: null", indent, k);
                    }
                    Value::String(s) => {
                        if s.contains('\n') {
                            println!("{}{}:", indent, k);
                            for line in s.lines() {
                                println!("{}  {}", indent, line);
                            }
                        } else {
                            println!("{}{}: {}", indent, k, s);
                        }
                    }
                    Value::Array(arr) if arr.len() <= 3 && arr.iter().all(|e| matches!(e, Value::String(_))) => {
                        let items: Vec<&str> = arr.iter().map(|e| e.as_str().unwrap_or("")).collect();
                        println!("{}{}: [{}]", indent, k, items.join(", "));
                    }
                    _ => {
                        print!("{}{}: ", indent, k);
                        print_value(v, depth + 1);
                    }
                }
            }
        }
    }
}

/// Format a compile result as a human-readable string.
pub fn format_compile_result(result: &Value) -> String {
    let source = result.get("source").and_then(|s| s.as_str()).unwrap_or("unknown");
    let page_count = result.get("page_count").and_then(|p| p.as_u64()).unwrap_or(0);
    let entries = result.get("entries").and_then(|e| e.as_array()).map(|a| a.len()).unwrap_or(0);
    format!(
        "Compiled: {} ({} pages, {} entries pending)",
        source, page_count, entries
    )
}

/// Format incremental compile result.
pub fn format_incremental_result(result: &Value) -> String {
    let compiled = result.get("compiled").and_then(|c| c.as_u64()).unwrap_or(0);
    let skipped = result.get("skipped").and_then(|s| s.as_u64()).unwrap_or(0);
    let total = result.get("total_scanned").and_then(|t| t.as_u64()).unwrap_or(0);
    format!(
        "Scanned {} files — {} compiled, {} skipped",
        total, compiled, skipped
    )
}
