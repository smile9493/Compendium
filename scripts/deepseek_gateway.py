#!/usr/bin/env python3
import os
import sys
import json
from http.server import HTTPServer, BaseHTTPRequestHandler
import urllib.request
import urllib.error

PORT = 5000
DEEPSEEK_BASE_URL = 'https://api.deepseek.com'
THINKING_MODELS = ['deepseek-v4-pro', 'deepseek-v4-flash']

class GatewayHandler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        print(f"{self.log_date_time_string()} {args[0]}")

    def send_json_response(self, status_code, data):
        self.send_response(status_code)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(data).encode())

    def do_GET(self):
        if self.path == '/health' or self.path == '/v1/models':
            self.send_json_response(200, {'status': 'ok'})
        else:
            self.send_json_response(404, {'error': 'Not found', 'path': self.path})

    def do_POST(self):
        if self.path == '/' or self.path.startswith('/v1/chat/completions'):
            content_length = int(self.headers.get('Content-Length', 0))
            body = self.rfile.read(content_length).decode('utf-8')
            
            try:
                data = json.loads(body)
            except json.JSONDecodeError as e:
                self.send_json_response(400, {'error': str(e)})
                return

            api_key = (
                self.headers.get('x-api-key') or 
                self.headers.get('Authorization', '').replace('Bearer ', '') or 
                os.environ.get('DEEPSEEK_API_KEY')
            )

            if not api_key:
                self.send_json_response(401, {'error': 'API key required'})
                return

            if data.get('tools') and isinstance(data['tools'], list):
                data['tools'] = [
                    {
                        'type': 'function',
                        'function': {
                            'name': tool['name'],
                            'description': tool.get('description', ''),
                            'parameters': tool.get('parameters', {'type': 'object', 'properties': {}})
                        }
                    } if tool.get('type') == 'function' and tool.get('name') and not tool.get('function') else tool
                    for tool in data['tools']
                ]

            if data.get('model') in THINKING_MODELS:
                if 'extra_body' not in data:
                    data['extra_body'] = {}
                if 'thinking' not in data['extra_body']:
                    data['extra_body']['thinking'] = {'type': 'enabled'}
                if 'reasoning_effort' not in data:
                    data['reasoning_effort'] = 'high'

            modified_body = json.dumps(data).encode('utf-8')

            req = urllib.request.Request(
                f'{DEEPSEEK_BASE_URL}/v1/chat/completions',
                data=modified_body,
                headers={
                    'Content-Type': 'application/json',
                    'Authorization': f'Bearer {api_key}'
                },
                method='POST'
            )

            try:
                with urllib.request.urlopen(req, timeout=120) as response:
                    self.send_response(response.status)
                    for key, value in response.getheaders():
                        self.send_header(key, value)
                    self.end_headers()
                    self.wfile.write(response.read())
            except urllib.error.HTTPError as e:
                self.send_response(e.code)
                self.wfile.write(e.read())
            except Exception as e:
                self.send_json_response(500, {'error': str(e)})
        else:
            self.send_json_response(404, {'error': 'Not found', 'path': self.path})

if __name__ == '__main__':
    print(f'DeepSeek Gateway running on http://0.0.0.0:{PORT}')
    print(f'Proxying to: {DEEPSEEK_BASE_URL}')
    print(f'Thinking mode enabled for: {", ".join(THINKING_MODELS)}')
    print('Endpoints:')
    print(f'  POST http://localhost:{PORT}/v1/chat/completions')
    print(f'  GET  http://localhost:{PORT}/health')
    
    server = HTTPServer(('0.0.0.0', PORT), GatewayHandler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print('\nShutting down...')
        server.shutdown()