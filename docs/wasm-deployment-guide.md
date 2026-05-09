# WASM Cross-Origin Isolation Deployment Guide

## Overview

The `pdf-wasm` crate uses `SharedArrayBuffer` for zero-copy data transfer between JavaScript and WebAssembly. This requires cross-origin isolation headers to be configured on your web server.

## Requirements

### COOP/COEP Headers (Required)

For `SharedArrayBuffer` to be available in the browser, the following HTTP headers **must** be set:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

### Nginx Configuration

```nginx
server {
    listen 443 ssl;

    # Cross-origin isolation headers (required for SharedArrayBuffer)
    add_header Cross-Origin-Opener-Policy "same-origin" always;
    add_header Cross-Origin-Embedder-Policy "require-corp" always;

    location / {
        root /var/www/pdf-wasm;
        # If serving WASM files
        types {
            application/wasm wasm;
        }
    }
}
```

### Apache Configuration

```apache
<VirtualHost *:443>
    # Cross-origin isolation headers
    Header always set Cross-Origin-Opener-Policy "same-origin"
    Header always set Cross-Origin-Embedder-Policy "require-corp"

    DocumentRoot /var/www/pdf-wasm
</VirtualHost>
```

### Express.js (Node.js)

```javascript
const express = require('express');
const app = express();

app.use((req, res, next) => {
    res.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
    res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
    next();
});

app.use(express.static('public'));
app.listen(3000);
```

## JavaScript Detection

Detect whether cross-origin isolation is active and `SharedArrayBuffer` is available:

```javascript
if (typeof SharedArrayBuffer === 'undefined') {
    console.warn(
        'SharedArrayBuffer is not available. ' +
        'Please ensure COOP/COEP headers are configured. ' +
        'See: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer'
    );
    // Fallback to non-shared-memory mode
} else {
    // Use SharedArrayBuffer for zero-copy data transfer
}
```

## Worker Isolation Mode

For running the WASM engine in a Web Worker:

```javascript
// worker.js
import init, { WasmPdfEngine } from './pdf_wasm.js';

let wasmEngine = null;

self.onmessage = async (e) => {
    if (!wasmEngine) {
        await init();
        wasmEngine = new WasmPdfEngine();
    }

    const { id, method, args } = e.data;

    try {
        const result = await wasmEngine[method](...args);
        self.postMessage({ id, result });
    } catch (error) {
        self.postMessage({ id, error: error.message });
    }
};
```

## Cross-Origin Resource Sharing (CORS)

If loading WASM from a different origin, the WASM file must be served with:

```
Access-Control-Allow-Origin: *
Cross-Origin-Resource-Policy: cross-origin
```

## Production Checklist

- [ ] COOP header set to `same-origin`
- [ ] COEP header set to `require-corp`
- [ ] `SharedArrayBuffer` is available in browser console
- [ ] WASM files served with correct MIME type (`application/wasm`)
- [ ] Worker isolation mode tested
- [ ] Fallback path for browsers without cross-origin isolation

## Browser Support

| Browser | COOP/COEP Support | SharedArrayBuffer |
|---------|-------------------|-------------------|
| Chrome 87+ | Yes | Yes |
| Firefox 79+ | Yes | Yes |
| Safari 15.2+ | Yes | Yes |
| Edge 87+ | Yes | Yes |

## Debugging

If `SharedArrayBuffer` is undefined:

1. Open browser DevTools
2. Check the Console tab for COOP/COEP errors
3. Verify headers using Network tab → Response Headers
4. Ensure no service worker is intercepting responses
5. Check that `crossOriginIsolated` is `true` in the console:
   ```javascript
   console.log(window.crossOriginIsolated); // Should be true
   ```
