import { initPdfWasm, createEngine } from '../../packages/pdf-wasm/index.js'

const fileInput = document.getElementById('file')
const canvas = document.getElementById('thumb')
const textEl = document.getElementById('text')

fileInput.addEventListener('change', async (e) => {
  const file = e.target.files?.[0]
  if (!file) return
  const wasm = await initPdfWasm()
  const engine = createEngine(wasm)
  const buf = new Uint8Array(await file.arrayBuffer())
  engine.open(buf)
  const pages = engine.get_page_count()
  textEl.textContent = `Pages: ${pages}\n` + new TextDecoder().decode(engine.extract_page_text(0).as_bytes())
  const thumb = engine.render_page_thumbnail(0, 256)
  const rgba = thumb.as_bytes()
  const ctx = canvas.getContext('2d')
  const img = ctx.createImageData(256, 256)
  img.data.set(rgba.subarray(0, 256 * 256 * 4))
  ctx.putImageData(img, 0, 0)
  thumb.free()
})
