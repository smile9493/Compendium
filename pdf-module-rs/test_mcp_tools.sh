#!/bin/bash
# MCP JSON-RPC stdio 端到端测试
# 通过管道发送多个 JSON-RPC 请求，捕获 stdout 响应

REQ_FILE="/tmp/mcp_requests.txt"
RESP_FILE="/tmp/mcp_responses.txt"
DIAG_FILE="/tmp/mcp_diag.txt"

cat > "$REQ_FILE" <<'ENDREQ'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-agent","version":"1.0"}}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_page_count","arguments":{"pdf_path":"/app/test_kb/raw/test.pdf"}}}
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"extract_text","arguments":{"pdf_path":"/app/test_kb/raw/test.pdf"}}}
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"micro_compile","arguments":{"pdf_path":"/app/test_kb/raw/test.pdf"}}}
{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"search_knowledge","arguments":{"query":"rust","knowledge_base":"/app/test_kb"}}}
{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"get_entry_context","arguments":{"entry_path":"IT/test_entry","knowledge_base":"/app/test_kb"}}}
{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"show_wiki_browser","arguments":{}}}
{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"find_orphans","arguments":{"knowledge_base":"/app/test_kb"}}}
{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"check_quality","arguments":{"knowledge_base":"/app/test_kb"}}}
{"jsonrpc":"2.0","id":11,"method":"resources/list","params":{}}
ENDREQ

# Pipe all requests at once, give server time to respond
cat "$REQ_FILE" | stdbuf -oL timeout 10 ./target/debug/pdf-mcp > "$RESP_FILE" 2>"$DIAG_FILE"

echo "=== RESPONSES ==="
cat "$RESP_FILE" 2>/dev/null

echo ""
echo "EXIT CODE: $?"

echo ""
echo "=== DIAG ==="
cat "$DIAG_FILE" 2>/dev/null | head -20