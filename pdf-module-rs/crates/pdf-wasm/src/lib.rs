//! WASM PDF extraction engine with Arena allocator.
//!
//! Provides efficient memory management for PDF processing in WASM environments
//! using bump-allocated arenas that can be reset per-frame, avoiding memory leaks
//! and reducing fragmentation in the WASM linear memory.
//!
//! # Features
//!
//! - **Arena allocator**: Per-frame batch memory release via `bumpalo`
//! - **Global allocator**: `talc` for reduced binary size (~10KB → ~1KB)
//! - **Zero-copy interop**: `WasmSlice` for efficient JS-WASM boundary data passing
//! - **Structured errors**: Typed error variants for WASM-friendly error handling
//! - **Tracing support**: Optional `tracing-wasm` for logging

#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(clippy::all)]
#![deny(clippy::await_holding_lock)]
#![deny(clippy::await_holding_refcell_ref)]
#![deny(clippy::large_stack_frames)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![cfg_attr(test, allow(clippy::undocumented_unsafe_blocks))]
#![deny(clippy::todo)]
#![deny(clippy::dbg_macro)]
#![cfg_attr(not(test), warn(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::unwrap_used))]

#[cfg(feature = "wasm")]
#[global_allocator]
// SAFETY: TalckWasm provides a thread-safe bump-allocated global allocator
// backed by WASM linear memory. It implements the GlobalAlloc trait correctly
// for the wasm32-unknown-unknown target. `new_global()` initializes the allocator
// with WASM's linear memory as the backing storage.
static ALLOC: talc::TalckWasm = talc::TalckWasm::new_global();

pub mod arena;
pub mod error;
pub mod slice;

pub use arena::WasmPdfEngine;
pub use error::WasmError;
pub use slice::{OwnedSlice, WasmSlice};

/// Initialize WASM panic hook and tracing for better error messages.
///
/// This function should be called once when the WASM module is loaded.
/// It installs:
/// - A panic hook that logs panic messages to the JavaScript console
/// - A tracing subscriber that sends logs to the browser console
#[cfg(feature = "wasm")]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}

/// Auto-initialize panic hook on WASM instantiation.
///
/// This runs automatically when the WASM module is loaded by JavaScript,
/// ensuring panic messages are always visible in the browser console.
#[cfg(feature = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    init_panic_hook();
}
