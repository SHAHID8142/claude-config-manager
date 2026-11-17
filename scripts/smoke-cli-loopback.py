#!/usr/bin/env python3
"""Optional packaged CLI smoke: disposable HOME, fake keys, loopback only.
Requires a locally built macOS app and installed Claude Code with --bare.
Runs no paid provider request; removes fixture data and stops its own helper.
"""
import os
import json
import tempfile
import threading
import subprocess
import time
import shutil
from pathlib import Path
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
repo = Path(__file__).resolve().parent.parent
seen = []

class Fixture(BaseHTTPRequestHandler):

    def log_message(self, *args):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        key = self.headers.get('x-api-key') or self.headers.get('authorization')
        seen.append({'path': self.path, 'key': key, 'model': body.get('model')})
        if key == 'fixture-key-one':
            self.send_response(401)
            self.end_headers()
            self.wfile.write(b'{"error":{"type":"authentication_error","message":"fixture rejection"}}')
            return
        events = [('message_start',
          {'message': {'content': [],
                       'id': 'msg_fixture',
                       'model': 'fixture-routed-model',
                       'role': 'assistant',
                       'stop_reason': None,
                       'stop_sequence': None,
                       'type': 'message',
                       'usage': {'input_tokens': 1, 'output_tokens': 0}},
           'type': 'message_start'}),
         ('content_block_start',
          {'content_block': {'text': '', 'type': 'text'},
           'index': 0,
           'type': 'content_block_start'}),
         ('content_block_delta',
          {'delta': {'text': 'LOOPBACK_OK', 'type': 'text_delta'},
           'index': 0,
           'type': 'content_block_delta'}),
         ('content_block_stop', {'index': 0, 'type': 'content_block_stop'}),
         ('message_delta',
          {'delta': {'stop_reason': 'end_turn', 'stop_sequence': None},
           'type': 'message_delta',
           'usage': {'output_tokens': 1}}),
         ('message_stop', {'type': 'message_stop'})]
        wire = ''.join(('event: ' + name + '\ndata: ' + json.dumps(data) + '\n\n' for name, data in events)).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'text/event-stream')
        self.send_header('Content-Length', str(len(wire)))
        self.end_headers()
        self.wfile.write(wire)
server = ThreadingHTTPServer(('127.0.0.1', 0), Fixture)
threading.Thread(target=server.serve_forever, daemon=True).start()
with tempfile.TemporaryDirectory(prefix='ccm-cli-fixture-') as td:
    home = Path(td)
    root = home / '.claude-config-manager'
    secrets = root / 'secrets'
    secrets.mkdir(parents=True, mode=0o700)
    keys = []
    for i, key in enumerate(['fixture-key-one', 'fixture-key-two'], 1):
        ident = 'fixture-' + str(i)
        p = secrets / (ident + '.secret')
        p.write_text(key)
        p.chmod(0o600)
        keys.append(dict(id=ident, providerId='anthropic', label=ident, last4='-one' if i == 1 else '-two', priority=i, enabled=True, createdAt='2026-11-17T00:00:00Z'))
    (root / 'credentials.json').write_text(json.dumps(dict(version=1, credentials=keys)))
    (root / 'routing.json').write_text(json.dumps(dict(version=1, enabled=True, targets=[dict(id='fixture-a', providerId='anthropic', credentialId=None, model='fixture-routed-model', baseUrl='http://127.0.0.1:' + str(server.server_port), enabled=True)])))
    env = {'PATH': os.environ['PATH'], 'HOME': td, 'TMPDIR': td, 'CLAUDE_CONFIG_DIR': str(home / '.claude'), 'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC': '1', 'DISABLE_TELEMETRY': '1', 'DISABLE_ERROR_REPORTING': '1', 'ANTHROPIC_API_KEY': 'fixture-local-cli-only', 'ANTHROPIC_MAX_RETRIES': '0'}
    proc = subprocess.Popen([str(repo / 'src-tauri/target/release/bundle/macos/Claude Config Manager.app/Contents/MacOS/claude-config-manager'), '--proxy-helper'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(100):
            if (root / 'proxy-helper.json').exists():
                break
            if proc.poll() is not None:
                raise RuntimeError('fixture helper exited')
            time.sleep(0.05)
        port = json.loads((root / 'proxy-helper.json').read_text())['port']
        token = (secrets / 'proxy-helper.secret').read_text().strip()
        env['ANTHROPIC_BASE_URL'] = 'http://127.0.0.1:' + str(port)
        env['ANTHROPIC_API_KEY'] = token
        cli = shutil.which('claude')
        result = subprocess.run([cli, '--bare', '-p', 'Respond with LOOPBACK_OK', '--model', 'claude-sonnet-4-5', '--tools', '', '--no-session-persistence', '--strict-mcp-config', '--setting-sources', ''], cwd=td, env=env, capture_output=True, text=True, timeout=25)
        report = {'cliExit': result.returncode, 'expectedText': 'LOOPBACK_OK' in result.stdout, 'observedRequests': seen, 'stderr': result.stderr[:300], 'stdout': result.stdout[:300]}
        print(json.dumps(report, indent=2))
        assert result.returncode == 0 and report['expectedText'], 'CLI did not complete fixture response'
        assert [r['key'] for r in seen] == ['fixture-key-one', 'fixture-key-two'], 'Unexpected key order'
        assert all(r['path'] == '/v1/messages' and r['model'] == 'fixture-routed-model' for r in seen), 'Unexpected protocol/model mapping'
    finally:
        proc.terminate()
        proc.wait(timeout=5)
        server.shutdown()
