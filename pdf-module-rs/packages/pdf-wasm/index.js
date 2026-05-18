/**
 * @rsut/pdf-wasm — load compiled pdf_wasm module from pkg/
 */
export async function initPdfWasm(wasmUrl) {
  const wasm = await import('./pkg/pdf_wasm.js')
  const url = wasmUrl ?? new URL('./pkg/pdf_wasm_bg.wasm', import.meta.url)
  await wasm.default(url)
  wasm.init_panic_hook?.()
  return wasm
}

export function createEngine(wasm) {
  return new wasm.WasmPdfEngine()
}
