export interface PdfWasmModule {
  default: (input?: string | URL | Request) => Promise<void>
  init_panic_hook?: () => void
  WasmPdfEngine: new () => WasmPdfEngine
}

export interface WasmPdfEngine {
  open(data: Uint8Array): void
  get_page_count(): number
  render_page_thumbnail(page: number, max_px: number): OwnedSlice
  extract_page_text(page: number): OwnedSlice
  reset_arena(): void
}

export interface OwnedSlice {
  as_wasm_slice(): { ptr(): number; len(): number }
  as_bytes(): Uint8Array
  free(): void
}

export declare function initPdfWasm(wasmUrl?: string): Promise<PdfWasmModule>
export declare function createEngine(wasm: PdfWasmModule): WasmPdfEngine
