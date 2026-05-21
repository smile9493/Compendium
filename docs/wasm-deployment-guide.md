# WASM 跨源隔离部署指南

## 概述

`pdf-wasm` crate 使用 `SharedArrayBuffer` 在 JavaScript 与 WebAssembly 之间进行零拷贝数据传输。这需要 Web 服务器配置跨源隔离（Cross-Origin Isolation）响应头。

## 环境要求

### COOP / COEP 响应头（必需）

浏览器中要启用 `SharedArrayBuffer`，**必须**设置以下 HTTP 响应头：

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

### Nginx 配置

```nginx
server {
    listen 443 ssl;

    # 跨源隔离响应头（SharedArrayBuffer 必需）
    add_header Cross-Origin-Opener-Policy "same-origin" always;
    add_header Cross-Origin-Embedder-Policy "require-corp" always;

    location / {
        root /var/www/pdf-wasm;
        types {
            application/wasm wasm;
        }
    }
}
```

### Apache 配置

```apache
<VirtualHost *:443>
    # 跨源隔离响应头
    Header always set Cross-Origin-Opener-Policy "same-origin"
    Header always set Cross-Origin-Embedder-Policy "require-corp"

    DocumentRoot /var/www/pdf-wasm
</VirtualHost>
```

### Express.js（Node.js）

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

## JavaScript 检测

检测跨源隔离是否生效以及 `SharedArrayBuffer` 是否可用：

```javascript
if (typeof SharedArrayBuffer === 'undefined') {
    console.warn(
        'SharedArrayBuffer 不可用。' +
        '请确认 COOP/COEP 响应头已配置。' +
        '参考：https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer'
    );
    // 回退到非共享内存模式
} else {
    // 使用 SharedArrayBuffer 进行零拷贝数据传输
}
```

## Worker 隔离模式

在 Web Worker 中运行 WASM 引擎：

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

## 跨源资源共享（CORS）

若从不同源加载 WASM 文件，需设置：

```
Access-Control-Allow-Origin: *
Cross-Origin-Resource-Policy: cross-origin
```

## 生产环境检查清单

- [ ] COOP 响应头已设置为 `same-origin`
- [ ] COEP 响应头已设置为 `require-corp`
- [ ] 浏览器控制台中 `SharedArrayBuffer` 可用
- [ ] WASM 文件以正确 MIME 类型（`application/wasm`）提供
- [ ] Worker 隔离模式已测试
- [ ] 已为不支持跨源隔离的浏览器准备回退路径

## 浏览器支持

| 浏览器 | COOP/COEP 支持 | SharedArrayBuffer |
|---------|-------------------|-------------------|
| Chrome 87+ | 是 | 是 |
| Firefox 79+ | 是 | 是 |
| Safari 15.2+ | 是 | 是 |
| Edge 87+ | 是 | 是 |

## 调试

如果 `SharedArrayBuffer` 未定义：

1. 打开浏览器开发者工具（DevTools）
2. 检查 Console 面板中的 COOP/COEP 错误
3. 在 Network 面板 → Response Headers 中验证响应头
4. 确保没有 Service Worker 拦截响应
5. 在控制台中检查 `crossOriginIsolated` 是否为 `true`：

   ```javascript
   console.log(window.crossOriginIsolated); // 应为 true
   ```
