#!/usr/bin/env python3
"""P0-07: bounded, loopback-only Observation experiments on official 1.14.0.

Protects Veyra's metric units, attribution limits, DNS capability claims, actual
managed-request diagnostics and owned-child cleanup. No product implementation.
"""
import argparse
import base64
import getpass
import hashlib
import http.server
import ipaddress
import json
import os
from pathlib import Path
import platform
import re
import secrets
import shutil
import signal
import socket
import socketserver
import struct
import subprocess
import tarfile
import tempfile
import threading
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone

TAG = 'v1.14.0'
REVISION = '0b8995879f29a9b98ee027bc17b75e101445b238'
ARCHIVE_URL = 'https://github.com/SagerNet/sing-box/releases/download/v1.14.0/sing-box-1.14.0-darwin-arm64.tar.gz'
DIGEST = 'a150c94012ff768b7261939cd236b9c8554127f45137230295d23a5660225cc9'
DOMAIN = 'p007.example.invalid'
CACHE_DOMAIN = 'p007-cache.example.invalid'
MAX_FRAME = 1024 * 1024


def stamp():
    return {'at': datetime.now(timezone.utc).isoformat(), 'monotonic_ns': time.monotonic_ns()}


def command(args, timeout=20):
    return subprocess.run(args, capture_output=True, text=True, timeout=timeout)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class Stream:
    """Only unfragmented JSON text from the four fixed local endpoints, bounded.

    No reconnect/retry framework. Client control frames are masked; handshake
    checks the accept key. Unexpected framing, size or count fails the probe.
    """
    def __init__(self, port, endpoint, credential):
        assert endpoint in ['/connections', '/logs?level=debug', '/memory', '/traffic']
        self.endpoint = endpoint
        self.sock = socket.create_connection(('127.0.0.1', port), timeout=3)
        self.sock.settimeout(2)
        self.frames, self.errors = [], []
        self.closed = threading.Event()
        self.opened = stamp()
        key = base64.b64encode(secrets.token_bytes(16)).decode()
        try:
            self.sock.sendall((f'GET {endpoint} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n'
                               'Upgrade: websocket\r\nConnection: Upgrade\r\n'
                               f'Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n'
                               f'Authorization: Bearer {credential}\r\n\r\n').encode())
            header = bytearray()
            while not header.endswith(b'\r\n\r\n'):
                header.extend(self.exact(1))
                assert len(header) <= 16384
            lines = header.decode().split('\r\n')
            self.handshake_status = int(lines[0].split()[1])
            assert self.handshake_status == 101, 'WS handshake failed'
            headers = {x.split(': ', 1)[0].lower(): x.split(': ', 1)[1] for x in lines[1:] if ': ' in x}
            accept = base64.b64encode(hashlib.sha1((key + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').encode()).digest()).decode()
            assert headers['sec-websocket-accept'] == accept
        except Exception:
            self.sock.close()
            raise
        self.thread = threading.Thread(target=self.read, daemon=True)
        self.thread.start()

    def exact(self, size):
        data = bytearray()
        while len(data) < size:
            part = self.sock.recv(size - len(data))
            if not part:
                raise EOFError('stream ended')
            data.extend(part)
        return bytes(data)

    def control(self, opcode, payload=b''):
        assert len(payload) <= 125
        mask = secrets.token_bytes(4)
        encoded = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
        self.sock.sendall(bytes([0x80 | opcode, 0x80 | len(payload)]) + mask + encoded)

    def read(self):
        try:
            while not self.closed.is_set():
                try:
                    first = self.exact(1)[0]
                except socket.timeout:
                    continue  # Idle logs are not a failure; no partial frame consumed.
                second = self.exact(1)[0]
                assert first & 0x80 and not first & 0x70 and not second & 0x80
                size = second & 127
                if size == 126:
                    size = struct.unpack('!H', self.exact(2))[0]
                elif size == 127:
                    size = struct.unpack('!Q', self.exact(8))[0]
                assert size <= MAX_FRAME
                payload = self.exact(size)
                opcode = first & 15
                if opcode == 8:
                    if not self.closed.is_set():
                        raise EOFError('unexpected close')
                    break
                if opcode == 9:
                    self.control(10, payload)
                    continue
                assert opcode == 1, 'expected one unfragmented JSON text frame'
                frame = dict(stamp(), sequence=len(self.frames), data=json.loads(payload))
                self.frames.append(frame)
                assert len(self.frames) <= 1000, 'frame budget exceeded'
        except Exception as error:
            if not self.closed.is_set():
                self.errors.append(type(error).__name__ + ': ' + str(error))

    def wait(self, count, timeout=5):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            assert not self.errors, self.errors
            if len(self.frames) >= count:
                return
            time.sleep(.01)
        raise TimeoutError('stream sample deadline')

    def close(self):
        if self.closed.is_set():
            return
        self.closed.set()
        try:
            self.control(8, struct.pack('!H', 1000))
        except OSError:
            pass
        try:
            self.sock.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass
        self.sock.close()
        self.thread.join(timeout=3)
        assert not self.thread.is_alive()
        self.ended = stamp()

    def evidence(self):
        return {'endpoint': self.endpoint, 'handshake_status': self.handshake_status,
                'opened': self.opened, 'ended': self.ended, 'frame_count': len(self.frames),
                'errors': self.errors, 'frames': self.frames}


class Origin(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.respond()

    def do_POST(self):
        self.respond()

    def respond(self):
        size = int(self.headers.get('Content-Length', '0'))
        assert 0 <= size <= MAX_FRAME
        uploaded = self.rfile.read(size)
        event = dict(stamp(), path=self.path, method=self.command, uploaded_bytes=len(uploaded))
        self.server.requests.append(event)
        slow = self.path.startswith('/diagnostic-')
        if slow:
            time.sleep(2.4)
        response_size = 524288 if self.path.startswith('/burst-1-') else 262144 if self.path.startswith('/burst-2-') else 4096
        body = (self.path + '\n').encode() + b'x' * response_size
        self.send_response(200)
        self.send_header('Content-Length', str(len(body)))
        self.send_header('Connection', 'close')
        self.end_headers()
        self.wfile.write(body)
        event.update(completed=stamp(), response_body_bytes=len(body))

    def log_message(self, *_):
        pass


class DNSFixture(socketserver.BaseRequestHandler):
    """One exact owned A/IN question; controllable TEST-NET answers for cache proof.

    This is a fixture, not a DNS resolver/implementation or protocol certification.
    """
    def handle(self):
        packet, sock = self.request
        expected = b''.join(bytes([len(x)]) + x.encode() for x in CACHE_DOMAIN.split('.')) + b'\0' + struct.pack('!HH', 1, 1)
        try:
            assert 12 <= len(packet) <= 512
            assert struct.unpack('!H', packet[4:6])[0] == 1
            assert packet[12:12 + len(expected)] == expected
            event = dict(stamp(), domain=CACHE_DOMAIN, qtype='A', answer=self.server.answer)
            self.server.requests.append(event)
            header = packet[:2] + struct.pack('!HHHHH', 0x8180, 1, 1, 0, 0)
            answer = b'\xc0\x0c' + struct.pack('!HHIH', 1, 1, 300, 4) + socket.inet_aton(self.server.answer)
            sock.sendto(header + expected + answer, self.client_address)
        except Exception as error:
            self.server.errors.append(type(error).__name__ + ': ' + str(error))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert platform.system() == 'Darwin' and platform.machine() == 'arm64'
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    assert not list(output.glob('*.json')), 'use a new evidence directory'
    root = Path(tempfile.mkdtemp(prefix='veyra-p0-07-', dir='/private/tmp'))
    root.chmod(0o700)
    credential = secrets.token_hex(32)
    username = getpass.getuser()
    proc = None
    origin = dns = None
    streams, threads, child_lines, child_errors = [], [], [], []
    ports = []
    status, failure = 'FAIL', None
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    artifacts = {}

    def clean(text):
        text = text.replace(credential, '<redacted>').replace(str(Path.cwd()), '<workspace>')
        text = re.sub(r'/Users/[^\s\"\']+', '<user-path>', text)
        text = re.sub(r'/(?:private/)?tmp/[^\s\"\']+', '<tmp-path>', text)
        text = re.sub(r'(?<![\w.-])' + re.escape(username) + r'(?![\w.-])', '<user>', text)
        text = re.sub(r'network: updated network environment:.*', 'network: <host-network-redacted>', text)
        text = re.sub(r'network: updated default interface.*', 'network: <host-interface-redacted>', text)
        return text

    def save(name, value):
        text = clean(json.dumps(value, ensure_ascii=False, indent=2)) + '\n'
        assert credential not in text and username not in text
        assert not re.search(r'Authorization|Bearer|/Users/|/private/tmp/|\b(?:en|utun|bridge)\d+\b', text, re.I)
        # Scan actual address literals, not versions, dotted timestamps or source identifiers.
        for candidate in re.findall(r'(?<![\w.])(?:\d{1,3}\.){3}\d{1,3}(?![\w.])', text):
            addr = ipaddress.ip_address(candidate)
            assert addr.is_loopback or any(addr in ipaddress.ip_network(net) for net in ['192.0.2.0/24', '198.51.100.0/24', '203.0.113.0/24', '198.18.0.0/15']), 'unapproved address'
        (output / name).write_text(text)

    def download(url, target):
        result = command(['curl', '-fL', '--connect-timeout', '15', '--max-time', '120', '-o', str(target), url], 130)
        assert result.returncode == 0, 'official download failed'

    def api(endpoint, method='GET', body=None, authenticate=True):
        headers = {'Content-Type': 'application/json'}
        if authenticate:
            headers['Authorization'] = 'Bearer ' + credential
        request = urllib.request.Request(f'http://127.0.0.1:{controller}{endpoint}', method=method, headers=headers,
                                         data=None if body is None else json.dumps(body).encode())
        try:
            response = opener.open(request, timeout=3)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            raw = response.read(MAX_FRAME + 1)
            assert len(raw) <= MAX_FRAME
            try:
                data = json.loads(raw) if raw else None
            except json.JSONDecodeError:
                data = raw.decode()
            return dict(stamp(), method=method, endpoint=endpoint, status=response.code, body=data)

    def subscribe(endpoint):
        stream = Stream(controller, endpoint, credential)
        streams.append(stream)
        return stream

    def request(label, upload=0):
        path = '/' + label + '-' + secrets.token_hex(8)  # Public correlation marker, not credential.
        started = stamp()
        with socket.create_connection(('127.0.0.1', mixed), timeout=5) as sock:
            source_port = sock.getsockname()[1]
            method = 'POST' if upload else 'GET'
            sock.sendall((f'{method} http://{DOMAIN}:{origin.server_port}{path} HTTP/1.1\r\n'
                          f'Host: {DOMAIN}:{origin.server_port}\r\nConnection: close\r\n'
                          f'Content-Length: {upload}\r\n\r\n').encode() + b'u' * upload)
            data = bytearray()
            while True:
                part = sock.recv(65536)
                if not part:
                    break
                data.extend(part)
                assert len(data) <= 2 * MAX_FRAME
        header, body = bytes(data).split(b'\r\n\r\n', 1)
        assert header.startswith(b'HTTP/1.0 200') or header.startswith(b'HTTP/1.1 200')
        assert body.startswith((path + '\n').encode())
        return {'started': started, 'ended': stamp(), 'path': path, 'host': DOMAIN,
                'destination_port': origin.server_port, 'source_port': source_port,
                'status': 200, 'response_bytes': len(data), 'body_bytes': len(body),
                'uploaded_body_bytes': upload, 'origin_marker_verified': True}

    try:
        # Tag identity independently resolved, then all inspected files from pinned commit archive.
        ref = command(['git', 'ls-remote', 'https://github.com/SagerNet/sing-box.git', 'refs/tags/' + TAG, 'refs/tags/' + TAG + '^{}'], 40)
        assert ref.returncode == 0 and ref.stdout.split()[0] == REVISION
        source_archive = root / 'source.tar.gz'
        download('https://codeload.github.com/SagerNet/sing-box/tar.gz/' + REVISION, source_archive)
        with tarfile.open(source_archive) as bundle:
            bundle.extractall(root, filter='data')
        source = root / ('sing-box-' + REVISION)
        meanings = {
            'experimental/clashapi/server.go': [(123, 139, 'Authenticated standard controller mounts; no OpenBox wrapper.'),
                (251, 284, 'Header credential or browser WS query credential; probe uses header only.'),
                (298, 353, 'Per subscription Total baseline, 1 second ticker, new minus old int64 bytes, update old after successful send.'),
                (355, 427, 'Logs are level-filtered type/payload strings, event-driven, no structured DNS history.')],
            'experimental/clashapi/connections.go': [(24, 44, 'Active connection snapshot plus aggregate totals; excludes DNS outbound.'),
                (49, 104, 'id/metadata/upload/download/start/chains/rule; dnsMode constant normal, empty rulePayload.'),
                (107, 159, 'Immediate snapshot, then interval default 1000 ms; not event stream.')],
            'experimental/clashapi/api_meta.go': [(23, 34, 'Memory/group/upgrade routes; no connection event or DNS history route.'),
                (36, 101, 'Memory 1 second ticker; first inuse zero sentinel; subsequent Go allocation bytes, oslimit zero placeholder.')],
            'common/trafficcontrol/manager.go': [(98, 139, 'Total includes active, retained closed and evicted closed counts; closed traffic remains in aggregate.'),
                (146, 167, 'Connections() active only; ClosedConnections() is internal, not exposed by standard Clash route.')],
            'common/trafficcontrol/tracker.go': [(36, 54, 'Counters wrap routed TCP/packet byte I/O.'), (67, 104, 'UUID, start time and observed outbound chain stored by actual tracker.')],
            'experimental/clashapi/dns.go': [(16, 19, 'Only GET /dns/query; active tool, not history/records.'), (37, 80, 'Active Exchange response; Server internal is fixed string, not resolver chain.')],
            'experimental/clashapi/cache.go': [(14, 43, 'POST /cache/dns/flush calls DNSRouter.ClearCache; POST /cache/fakeip/flush calls optional CacheFile.FakeIPReset separately.')],
            'experimental/clashapi/configs.go': [(12, 17, 'GET/PUT/PATCH configs routes.'), (53, 70, 'PATCH only Mode; PUT no-op 204; neither DNS rewrite nor log-level update.')],
            'experimental/clashapi/ruleprovider.go': [(11, 58, 'No usable rule-provider update: lookup always 404; update body commented out.')],
            'experimental/clashapi/profile.go': [(16, 23, 'Tracing route is 404 stub; not DNS events.')],
            'dns/client_log.go': [(12, 72, 'Cached/exchanged/etc logs include RR strings; debug summaries include domain/rcode/TTL, no elapsed/source stable contract.')],
            'dns/client.go': [(464, 474, 'ClearCache purges memory and optional managed persisted DNS cache.')],
            'dns/router.go': [(1358, 1365, 'Router clears managed client/reverse cache; no platform interface in this CLI prototype.')],
        }
        files = []
        for path, ranges in meanings.items():
            files.append({'path': path, 'sha256': sha(source / path), 'line_count': len((source / path).read_text().splitlines()),
                          'url': f'https://github.com/SagerNet/sing-box/blob/{REVISION}/{path}',
                          'semantic_ranges': [{'start': a, 'end': b, 'meaning': meaning} for a, b, meaning in ranges]})
        route_lines = []
        for file in sorted((source / 'experimental/clashapi').glob('*.go')):
            for number, line in enumerate(file.read_text().splitlines(), 1):
                if re.search(r'\br\.(Get|Post|Put|Patch|Delete|Mount|Route)\(', line):
                    route_lines.append({'path': str(file.relative_to(source)), 'line': number, 'registration': line.strip()})
        artifacts['kernel-source-evidence.json'] = {'tag': TAG, 'commit': REVISION, 'tag_resolution': ref.stdout.strip(),
            'source_archive_sha256': sha(source_archive), 'files': files, 'route_inventory': route_lines,
            'counter_dependency': 'github.com/sagernet/sing v0.9.0-beta.4; NewInt64CounterConn/PacketConn counts routed byte I/O, not wire/IP headers',
            'reviewed_scope': 'All controller routes and handlers; no runtime hosts/predefined/DNS rule mutation or DNS query event/history API.'}
        archive = root / 'kernel.tar.gz'
        download(ARCHIVE_URL, archive)
        assert sha(archive) == DIGEST, 'digest mismatch: do not execute'
        with tarfile.open(archive) as bundle:
            bundle.extractall(root, filter='data')
        binary = root / 'sing-box-1.14.0-darwin-arm64/sing-box'
        version = command([str(binary), 'version'])
        assert version.returncode == 0 and REVISION in version.stdout and 'darwin/arm64' in version.stdout
        artifacts['kernel-identity.json'] = {'tag': TAG, 'commit': REVISION, 'archive_url': ARCHIVE_URL,
            'archive_sha256': sha(archive), 'expected_sha256': DIGEST, 'digest_match_before_execution': True,
            'binary_sha256': sha(binary), 'version': version.stdout, 'host': platform.platform(),
            'veyra_head': command(['git', 'rev-parse', 'HEAD']).stdout.strip(), 'recorded': stamp()}
        origin = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Origin)
        origin.requests = []
        dns = socketserver.UDPServer(('127.0.0.1', 0), DNSFixture)
        dns.answer, dns.requests, dns.errors = '192.0.2.10', [], []
        for server in [origin, dns]:
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            threads.append(thread)
        ports.append(('tcp', origin.server_port))
        ports.append(('udp', dns.server_address[1]))
        config = {'log': {'level': 'debug', 'timestamp': False},
            'dns': {'servers': [{'type': 'hosts', 'tag': 'p007-hosts', 'predefined': {DOMAIN: ['127.0.0.1']}},
                                {'type': 'udp', 'tag': 'p007-owned-dns', 'server': '127.0.0.1', 'server_port': dns.server_address[1]}],
                    'rules': [{'domain': [CACHE_DOMAIN], 'action': 'route', 'server': 'p007-owned-dns'}],
                    'final': 'p007-hosts'},
            'inbounds': [{'type': 'mixed', 'tag': 'p007-mixed', 'listen': '127.0.0.1', 'listen_port': 0}],
            'outbounds': [{'type': 'direct', 'tag': 'p007-direct'}],
            'route': {'default_domain_resolver': {'server': 'p007-hosts', 'strategy': 'ipv4_only'},
                      'rules': [{'domain': [DOMAIN], 'action': 'resolve', 'server': 'p007-hosts', 'strategy': 'ipv4_only'},
                                {'ip_cidr': ['127.0.0.0/8'], 'action': 'route', 'outbound': 'p007-direct'}, {'action': 'reject'}]},
            'experimental': {'clash_api': {'external_controller': '127.0.0.1:0', 'secret': credential}}}
        config_path = root / 'config.json'
        config_path.write_text(json.dumps(config))
        config_path.chmod(0o600)
        checked = command([str(binary), '--disable-color', 'check', '-c', str(config_path)])
        artifacts['config-check.json'] = {'status': 'PASS' if checked.returncode == 0 else 'FAIL', 'exit_code': checked.returncode,
            'output': checked.stdout + checked.stderr, 'actual_config_sha256': sha(config_path), 'timeout_seconds': 20}
        assert checked.returncode == 0, 'loopback configuration check failed'
        sanitized_config = json.loads(json.dumps(config))
        sanitized_config['experimental']['clash_api'].pop('secret')
        artifacts['config-template.json'] = sanitized_config
        proc = subprocess.Popen([str(binary), '--disable-color', 'run', '-c', str(config_path)], stdout=subprocess.PIPE,
                                 stderr=subprocess.STDOUT, text=True, start_new_session=True)
        identity = dict(stamp(), pid=proc.pid, process_group=proc.pid, config_sha256=sha(config_path))
        def child_reader():
            try:
                for line in proc.stdout:
                    child_lines.append(clean(line))
            except Exception as error:
                child_errors.append(type(error).__name__)
        child_thread = threading.Thread(target=child_reader, daemon=True)
        child_thread.start()
        deadline = time.monotonic() + 10
        controller = mixed = None
        while time.monotonic() < deadline:
            assert proc.poll() is None, 'child exited'
            for line in list(child_lines):
                match = re.search(r'clash-api: restful api listening at 127\.0\.0\.1:([1-9][0-9]{0,4})$', line.strip())
                if match:
                    controller = int(match[1])
                match = re.search(r'inbound/mixed\[p007-mixed\]: tcp server started at 127\.0\.0\.1:([1-9][0-9]{0,4})$', line.strip())
                if match:
                    mixed = int(match[1])
            if controller and mixed:
                break
            time.sleep(.02)
        assert controller and mixed, 'owned-child port discovery deadline'
        ports.extend([('tcp', controller), ('tcp', mixed)])
        ready = api('/version')
        assert ready['status'] == 200 and ready['body']['version'] == 'sing-box 1.14.0'
        assert api('/version', authenticate=False)['status'] == 401
        owned = command(['lsof', '-nP', '-a', '-p', str(proc.pid), '-iTCP', '-sTCP:LISTEN'])
        assert owned.returncode == 0
        for port in [controller, mixed]:
            assert f'127.0.0.1:{port} (LISTEN)' in owned.stdout
        artifacts['controller.json'] = dict(identity, controller_port=controller, mixed_port=mixed,
            discovery='owned child stdout only; PID-scoped lsof independent confirmation', listener_output=owned.stdout,
            ready=ready, unauthenticated_status=401)
        connections = subscribe('/connections')
        logs = subscribe('/logs?level=debug')
        memory = subscribe('/memory')
        traffic1 = subscribe('/traffic')
        traffic1.wait(2)
        assert all(x['data'] == {'up': 0, 'down': 0} for x in traffic1.frames[:2])
        burst1_before = api('/connections')['body']
        burst1 = request('burst-1', 131072)
        traffic1.wait(5)
        assert any(x['data']['down'] > 0 and x['data']['up'] > 0 for x in traffic1.frames)
        assert traffic1.frames[-1]['data'] == {'up': 0, 'down': 0}
        burst1_after = api('/connections')['body']
        traffic1.close()
        # Deliberate observation gap: reconnect must NOT replay these bytes.
        gap = request('unobserved-gap', 8192)
        traffic2 = subscribe('/traffic')
        traffic2.wait(2)
        assert all(x['data'] == {'up': 0, 'down': 0} for x in traffic2.frames[:2])
        burst2_before = api('/connections')['body']
        burst2 = request('burst-2', 65536)
        traffic2.wait(5)
        assert any(x['data']['down'] > 0 and x['data']['up'] > 0 for x in traffic2.frames)
        assert traffic2.frames[-1]['data'] == {'up': 0, 'down': 0}
        burst2_after = api('/connections')['body']
        def phase(stream, before, after, burst):
            total = {key: sum(x['data'][key] for x in stream.frames) for key in ['up', 'down']}
            rest_delta = {'up': after['uploadTotal'] - before['uploadTotal'], 'down': after['downloadTotal'] - before['downloadTotal']}
            assert total == rest_delta, 'stream byte sums differ from authoritative Total delta'
            assert total['down'] >= burst['body_bytes'] and total['up'] >= burst['uploaded_body_bytes']
            return {'request': burst, 'frame_sum_bytes': total, 'rest_total_delta_bytes': rest_delta,
                    'idle_before': stream.frames[:2], 'nonzero': [x for x in stream.frames if x['data']['up'] or x['data']['down']],
                    'idle_after': stream.frames[-1]}
        artifacts['traffic-semantics.json'] = {'status': 'PASS', 'source': 'kernel-source-evidence.json',
            'kind': 'interval_increment', 'unit': 'bytes of tracked routed I/O; not network wire bytes',
            'nominal_interval_ms': 1000, 'interval_query_supported': False,
            'interval_constraint': 'Kernel does not emit timestamp/interval. Arrival monotonic interval estimates generation interval only under timely delivery; stalls/coalescing are not exact.',
            'conversion': 'bytes_per_second = frame_bytes * 1000 / interval_ms; no second difference',
            'observed_intervals_ms': [[(b['monotonic_ns'] - a['monotonic_ns']) / 1e6 for a, b in zip(stream.frames, stream.frames[1:])] for stream in [traffic1, traffic2]],
            'example': {'frame_bytes': 524288, 'interval_ms': 1002, 'bytes_per_second': 524288 * 1000 / 1002},
            'session_rule': 'Rust single collector adds each unique (instance_id, stream_generation, sequence) interval once; no differencing of frame values.',
            'reconnect_rule': 'Close old generation before opening new; retain same-instance confirmed sum but mark gap; reset timing baseline; first window starts at subscription, never replay lifetime totals.',
            'instance_change_rule': 'New instance resets session totals/timing/generation; reject late old-instance frames. REST total counter rollback starts new baseline and marks discontinuity.',
            'burst1': phase(traffic1, burst1_before, burst1_after, burst1),
            'gap_request_excluded_on_reconnect': gap,
            'burst2': phase(traffic2, burst2_before, burst2_after, burst2),
            'rest_lifetime_total_after_burst2': {'up': burst2_after['uploadTotal'], 'down': burst2_after['downloadTotal']},
            'persistent_stream_required': True}
        # Long managed diagnosis: sole pending request in a bounded window.
        connections.wait(len(connections.frames) + 1)
        diag_before_response = api('/connections')
        diag_before = diag_before_response['body']
        long_request = request('diagnostic')
        connections.wait(len(connections.frames) + 1)
        diag_after_response = api('/connections')
        diag_after = diag_after_response['body']
        matches = [(f, c) for f in connections.frames for c in (f['data']['connections'] or [])
                   if c['metadata']['sourcePort'] == str(long_request['source_port'])]
        assert matches and len({c['id'] for _, c in matches}) == 1
        connection = matches[0][1]
        assert connection['metadata']['host'] == DOMAIN
        assert connection['metadata']['type'] == 'mixed/p007-mixed'
        assert connection['chains'] == ['p007-direct']
        # Extremely short request immediately after a connections snapshot.
        connections.wait(len(connections.frames) + 1)
        previous = connections.frames[-1]
        short_before = api('/connections')['body']
        short_request = request('short')
        connections.wait(len(connections.frames) + 1)
        following = connections.frames[-1]
        short_after = api('/connections')['body']
        short_matches = [(f, c) for f in connections.frames for c in (f['data']['connections'] or [])
                         if c['metadata']['sourcePort'] == str(short_request['source_port'])]
        bounded_between = previous['monotonic_ns'] < short_request['started']['monotonic_ns'] < short_request['ended']['monotonic_ns'] < following['monotonic_ns']
        assert bounded_between, 'short request did not fit between snapshots'
        traffic2.wait(len(traffic2.frames) + 2)
        short_logs = [f for f in logs.frames if f"127.0.0.1:{short_request['source_port']}" in f['data']['payload']]
        assert short_logs, 'short inbound log not observed'
        artifacts['connections-gap.json'] = {'status': 'PASS', 'kind': 'active snapshots', 'default_interval_ms': 1000,
            'long_request': long_request, 'long_frames': [{'frame_sequence': f['sequence'], 'received': {'at': f['at'], 'monotonic_ns': f['monotonic_ns']}, 'connection': c} for f, c in matches],
            'short_request': short_request,
            'short_rest_total_delta_bytes': {'up': short_after['uploadTotal'] - short_before['uploadTotal'], 'down': short_after['downloadTotal'] - short_before['downloadTotal']},
            'traffic_frames_near_short_window': [f for f in traffic2.frames if previous['monotonic_ns'] <= f['monotonic_ns'] <= following['monotonic_ns'] + 1000000000],
            'bracketing_frames': [previous, following],
            'short_fully_between_received_snapshots': bounded_between, 'short_seen': bool(short_matches),
            'short_matches': [{'frame_sequence': f['sequence'], 'connection': c} for f, c in short_matches], 'short_logs': short_logs,
            'conclusion': 'Short connection missed by standard snapshots; aggregate totals/log still change.' if not short_matches else 'This short connection was captured; do not infer universal loss.',
            'p3_scope': 'Keep unattributed traffic/gap disclosure; bucket/index/retention/capacity choices belong to P3.'}
        diag_logs = [f for f in logs.frames if f"127.0.0.1:{long_request['source_port']}" in f['data']['payload']]
        log_ids = []
        for f in diag_logs:
            match = re.search(r'\[(\d+) ', f['data']['payload'])
            if match:
                log_ids.append(match[1])
        related_logs = [f for f in logs.frames if any(f'[{value} ' in f['data']['payload'] for value in log_ids)]
        observed_rule_indexes = {int(m[1]) for f in related_logs if (m := re.search(r'router: match\[(\d+)\].* => route\(', f['data']['payload']))}
        assert len(observed_rule_indexes) == 1, 'route rule log not uniquely correlated'
        diag_delta = {'up': diag_after['uploadTotal'] - diag_before['uploadTotal'], 'down': diag_after['downloadTotal'] - diag_before['downloadTotal']}
        artifacts['diagnostic.json'] = {'status': 'PASS', 'complete_path': False, 'request': long_request,
            'origin_receipt': [r for r in origin.requests if r['path'] == long_request['path']],
            'association': 'Unique public origin path/body marker + sole in-flight request + source port + bounded window; snapshot ID unique across frames. Log numeric context ID is not snapshot UUID.',
            'associated_fields': {'request_success': True, 'http_status': 200, 'inbound': connection['metadata']['type'],
                'outbound_tags': connection['chains'], 'destination_host': connection['metadata']['host'],
                'destination_port': connection['metadata']['destinationPort'], 'connection_id': connection['id'],
                'start': connection['start'], 'rule_index_from_correlated_debug_log': next(iter(observed_rule_indexes)),
                'source_ip': connection['metadata']['sourceIP'], 'source_port': connection['metadata']['sourcePort'],
                'connection_upload_bytes_at_snapshot': connection['upload'], 'connection_download_bytes_at_snapshot': connection['download'],
                'isolated_window_total_delta_bytes': diag_delta},
            'raw_controller_rule_text': connection['rule'], 'rule_text_is_not_rule_index': True,
            'rule_index_scope': 'Only the actual route match index in correlated debug logs for this request/configuration; snapshot has rule text, no index; not a persistent rule ID/API guarantee.',
            'absent_unknown': {'dns_event': 'unknown; no unique structured event ID',
                'resolver_chain': 'unknown', 'process_identity': 'absent; processPath empty', 'filter_hit': 'unknown'},
            'logs': related_logs,
            'traffic_frames_in_window': [f for f in traffic2.frames if long_request['started']['monotonic_ns'] < f['monotonic_ns'] < diag_after_response['monotonic_ns']]}
        assert not connection['metadata']['processPath']
        # Cache flush: changed owned upstream answer remains cached until real API flush.
        query_endpoint = '/dns/query?name=' + CACHE_DOMAIN + '&type=A'
        first = api(query_endpoint)
        assert first['status'] == 200 and first['body']['Answer'][0]['data'] == '192.0.2.10'
        before_count = len(dns.requests)
        dns.answer = '192.0.2.20'
        second = api(query_endpoint)
        assert second['body']['Answer'][0]['data'] == '192.0.2.10' and len(dns.requests) == before_count
        flushed = api('/cache/dns/flush', 'POST')
        assert flushed['status'] == 204
        third = api(query_endpoint)
        assert third['body']['Answer'][0]['data'] == '192.0.2.20' and len(dns.requests) == before_count + 1
        assert not dns.errors
        probes = [api(endpoint, method, body) for endpoint, method, body in [
            ('/dns/records', 'GET', None), ('/dns/history', 'GET', None), ('/profile/tracing', 'GET', None),
            ('/dns/rewrite', 'PUT', {'domain': DOMAIN, 'answer': '192.0.2.30'}),
            ('/dns/hosts', 'PUT', {DOMAIN: ['192.0.2.30']}), ('/providers/rules/p007-owned', 'PUT', {}),
            ('/cache/dns/flush', 'GET', None)]]
        assert all(p['status'] == 404 for p in probes[:-1]) and probes[-1]['status'] == 405
        active_before = api('/dns/query?name=' + DOMAIN + '&type=A')
        patch = api('/configs', 'PATCH', {'dns': {'hosts': {DOMAIN: ['192.0.2.30']}}})
        put = api('/configs', 'PUT', {'dns': {'hosts': {DOMAIN: ['192.0.2.30']}}})
        active_after = api('/dns/query?name=' + DOMAIN + '&type=A')
        assert patch['status'] == put['status'] == 204
        assert active_before['body']['Answer'][0]['data'] == active_after['body']['Answer'][0]['data'] == '127.0.0.1'
        time.sleep(.15)
        dns_logs = [f for f in logs.frames if 'dns:' in f['data']['payload'] or 'dns/' in f['data']['payload']]
        assert dns_logs
        artifacts['dns-capabilities.json'] = {'status': 'PASS', 'source': 'kernel-source-evidence.json',
            'records': {'capability': 'UNSUPPORTED', 'product': 'dns_query_records unavailable; explicit reason, no synthetic elapsed/source/filter-hit.',
                'routes': probes[:3], 'ordinary_dns_logs': dns_logs,
                'log_fields': {'domain': 'text only', 'qtype': 'RR text on some messages', 'result': 'RR text on some messages',
                    'time': 'collector receive time / optional formatted context, not stable structured query time',
                    'source': 'absent', 'elapsed': 'absent', 'filter_hit': 'absent'},
                'active_query_is_history': False, 'connections_is_dns_stream': False},
            'flush': {'capability': 'SUPPORTED', 'method': 'POST', 'endpoint': '/cache/dns/flush',
                'route_inventory': [{'method': 'POST', 'endpoint': '/cache/dns/flush', 'effect': 'DNSRouter.ClearCache'},
                    {'method': 'POST', 'endpoint': '/cache/fakeip/flush', 'effect': 'optional CacheFile.FakeIPReset; separate mapping reset, not tested/advertised as DNS response flush'}],
                'first': first, 'upstream_answer_changed_to': '192.0.2.20', 'cached_second': second, 'flush_response': flushed, 'third': third,
                'upstream_request_count_before_change': before_count, 'upstream_requests': dns.requests,
                'wrong_method_response': probes[-1], 'scope': 'Owned CLI instance response cache only; no cache.db deletion, UI history or system DNS operation.'},
            'hot_rewrite': {'capability': 'UNSUPPORTED', 'product': 'macOS first release DNS rewrite save returns RestartRequired; no target-only/remark-only live application promise.',
                'route_probes': probes[3:6], 'configs_patch_no_effect': patch, 'configs_put_no_effect': put,
                'hosts_query_before': active_before, 'hosts_query_after': active_after,
                'source_limit': 'configs PATCH changes mode only, PUT is no-op; providers rule update unusable. No runtime DNS rule/hosts/predefined mutation.'}}
        memory.wait(3)
        assert memory.frames[0]['data']['inuse'] == 0
        assert any(f['data']['inuse'] > 0 for f in memory.frames[1:])
        logs.wait(3)
        status = 'PASS'
    except Exception as error:
        failure = {'type': type(error).__name__, 'message': clean(str(error)), 'at': stamp()}
    finally:
        cleanup_errors = []
        for stream in streams:
            try:
                stream.close()
            except Exception as error:
                cleanup_errors.append(type(error).__name__)
        if proc:
            if proc.poll() is None:
                proc.send_signal(signal.SIGTERM)
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait(timeout=5)
                cleanup_errors.append('forced child kill')
            child_thread.join(timeout=3)
            if child_thread.is_alive() or child_errors:
                cleanup_errors.append('child log reader incomplete')
            proc.stdout.close()
        for server in [origin, dns]:
            if server:
                server.shutdown()
                server.server_close()
        for thread in threads:
            thread.join(timeout=3)
            if thread.is_alive():
                cleanup_errors.append('fixture thread incomplete')
        group_empty = proc is None
        if proc:
            try:
                os.killpg(proc.pid, 0)
            except ProcessLookupError:
                group_empty = True
        released = []
        for protocol, port in ports:
            try:
                if protocol == 'tcp':
                    with socket.socket() as client:
                        client.settimeout(.3)
                        assert client.connect_ex(('127.0.0.1', port)) != 0
                with socket.socket(socket.AF_INET, socket.SOCK_STREAM if protocol == 'tcp' else socket.SOCK_DGRAM) as sock:
                    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                    sock.bind(('127.0.0.1', port))
                released.append({'protocol': protocol, 'port': port, 'connect_refused_and_rebind' if protocol == 'tcp' else 'rebind': True})
            except Exception as error:
                cleanup_errors.append('port release: ' + type(error).__name__ + ': ' + str(error))
        if group_empty:
            shutil.rmtree(root)
        cleanup = {'pid': proc.pid if proc else None, 'child_exit_code': proc.returncode if proc else None,
            'child_reaped': proc is not None and proc.returncode is not None, 'process_group_empty': group_empty,
            'streams_closed_and_joined': all(s.closed.is_set() and not s.thread.is_alive() for s in streams),
            'released_ports': released, 'temp_directory_removed': not root.exists(), 'errors': cleanup_errors}
        if cleanup_errors or not group_empty or (proc and proc.returncode != 0) or any(s.errors for s in streams):
            status = 'FAIL'
        artifacts['stream-frames.json'] = {'status': 'PASS' if status == 'PASS' else 'FAIL',
            'transport': 'standard authenticated WS, handshake 101; no wrapper', 'streams': [s.evidence() for s in streams],
            'shapes': {'connections': {'uploadTotal': 'int bytes', 'downloadTotal': 'int bytes', 'memory': 'int bytes', 'connections': 'array or null; active snapshots'},
                'logs': {'type': 'string', 'payload': 'string'}, 'memory': {'inuse': 'int bytes; first zero sentinel', 'oslimit': 'int; zero placeholder'},
                'traffic': {'up': 'int interval bytes', 'down': 'int interval bytes'}}}
        artifacts['probe-result.json'] = {'task': 'OBG-P0-07', 'status': status, 'completed': stamp(), 'failure': failure,
            'cleanup': cleanup, 'system_proxy_changed': False, 'tun_started': False, 'system_dns_changed': False,
            'public_protocol_tested': False, 'product_rust_changed': False}
        for name, value in artifacts.items():
            save(name, value)
        (output / 'child.log').write_text(clean(''.join(child_lines)))
    print(json.dumps({'status': status, 'failure': failure, 'cleanup': cleanup}, ensure_ascii=False))
    return 0 if status == 'PASS' else 1


if __name__ == '__main__':
    raise SystemExit(main())
