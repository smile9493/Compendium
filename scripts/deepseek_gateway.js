#!/usr/bin/env node
const http = require('http');
const https = require('https');
const { URL } = require('url');

const PORT = 5000;
const DEEPSEEK_BASE_URL = 'https://api.deepseek.com';
const THINKING_MODELS = ['deepseek-v4-pro', 'deepseek-v4-flash'];

const server = http.createServer((req, res) => {
    console.log(`${new Date().toISOString()} ${req.method} ${req.url}`);
    
    if (req.url === '/health' || req.url === '/v1/models') {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ status: 'ok' }));
        return;
    }

    if ((req.url === '/' || req.url.startsWith('/v1/chat/completions')) && req.method === 'POST') {
        let body = '';
        
        req.on('data', chunk => { body += chunk.toString(); });
        req.on('end', () => {
            try {
                let data = JSON.parse(body);
                const apiKey = req.headers['x-api-key'] || req.headers['authorization']?.replace('Bearer ', '') || process.env.DEEPSEEK_API_KEY;
                
                if (!apiKey) {
                    res.writeHead(401, { 'Content-Type': 'application/json' });
                    res.end(JSON.stringify({ error: 'API key required' }));
                    return;
                }

                if (data.tools && Array.isArray(data.tools)) {
                    data.tools = data.tools.map(tool => {
                        if (tool.type === 'function' && tool.name && !tool.function) {
                            return {
                                type: 'function',
                                function: {
                                    name: tool.name,
                                    description: tool.description || '',
                                    parameters: tool.parameters || { type: 'object', properties: {} }
                                }
                            };
                        }
                        return tool;
                    });
                }

                if (data.model && THINKING_MODELS.includes(data.model)) {
                    if (!data.extra_body) {
                        data.extra_body = {};
                    }
                    if (!data.extra_body.thinking) {
                        data.extra_body.thinking = { type: "enabled" };
                    }
                    if (!data.reasoning_effort) {
                        data.reasoning_effort = "high";
                    }
                }

                const modifiedBody = JSON.stringify(data);
                const url = new URL(`${DEEPSEEK_BASE_URL}/v1/chat/completions`);

                const options = {
                    hostname: url.hostname,
                    port: 443,
                    path: url.pathname,
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                        'Authorization': `Bearer ${apiKey}`
                    }
                };

                const proxyReq = https.request(options, (proxyRes) => {
                    res.writeHead(proxyRes.statusCode, proxyRes.headers);
                    proxyRes.pipe(res);
                });

                proxyReq.on('error', (err) => {
                    res.writeHead(500, { 'Content-Type': 'application/json' });
                    res.end(JSON.stringify({ error: err.message }));
                });

                proxyReq.write(modifiedBody);
                proxyReq.end();
            } catch (err) {
                res.writeHead(400, { 'Content-Type': 'application/json' });
                res.end(JSON.stringify({ error: err.message }));
            }
        });
        return;
    }

    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: 'Not found', path: req.url }));
});

server.listen(PORT, '0.0.0.0', () => {
    console.log(`DeepSeek Gateway running on http://0.0.0.0:${PORT}`);
    console.log(`Proxying to: ${DEEPSEEK_BASE_URL}`);
    console.log(`Thinking mode enabled for: ${THINKING_MODELS.join(', ')}`);
    console.log('Endpoints:');
    console.log(`  POST http://localhost:${PORT}/v1/chat/completions`);
    console.log(`  GET  http://localhost:${PORT}/health`);
});