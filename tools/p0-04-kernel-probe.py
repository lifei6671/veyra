#!/usr/bin/env python3
"""P0-04 only: fixed macOS kernel, loopback controller, closed-writer cache.

Uses Python's standard library and curl/file/lsof. Never runs the TUN fixture.
All private inputs and kernel/cache files live in one temporary directory.
"""
import argparse
import copy
import getpass
import hashlib
import http.server
import ipaddress
import json
import os
from pathlib import Path
import platform
import queue
import re
import secrets
import shutil
import signal
import socket
import subprocess
import tarfile
import tempfile
import threading
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone

URL = "https://github.com/SagerNet/sing-box/releases/download/v1.14.0/sing-box-1.14.0-darwin-arm64.tar.gz"
DIGEST = "a150c94012ff768b7261939cd236b9c8554127f45137230295d23a5660225cc9"
CONTROLLER = r"clash-api: restful api listening at (127\.0\.0\.1:([1-9][0-9]{0,4}))$"
MIXED = r"inbound/mixed\[p004-mixed\]: tcp server started at 127\.0\.0\.1:([1-9][0-9]{0,4})$"
DOMAIN = "p004.example.invalid"
CACHE_ID = "veyra-p004-v114-generation-1"


def utc():
    return datetime.now(timezone.utc).isoformat()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def command(args, timeout=20):
    return subprocess.run(args, capture_output=True, text=True, timeout=timeout)


def config():
    return {
        "log": {"level": "info", "timestamp": False},
        "dns": {
            "servers": [
                {"type": "fakeip", "tag": "p004-fakeip", "inet4_range": "198.18.0.0/15"},
                {"type": "hosts", "tag": "p004-hosts", "predefined": {
                    "p004-fallback.example.invalid": ["192.0.2.1"]}},
            ],
            "rules": [{"domain_suffix": ["invalid"], "query_type": ["A"],
                       "action": "route", "server": "p004-fakeip"}],
            "final": "p004-hosts",
            "disable_cache": True,
        },
        "inbounds": [{"type": "mixed", "tag": "p004-mixed", "listen": "127.0.0.1", "listen_port": 0}],
        "outbounds": [
            {"type": "selector", "tag": "p004-selector",
             "outbounds": ["p004-direct-a", "p004-direct-b"], "default": "p004-direct-a"},
            {"type": "direct", "tag": "p004-direct-a"},
            {"type": "direct", "tag": "p004-direct-b"},
        ],
        "route": {"default_domain_resolver": "p004-hosts", "rules": [
            {"ip_cidr": ["127.0.0.0/8"], "action": "route", "outbound": "p004-selector"},
            {"action": "reject"},
        ], "final": "p004-selector"},
        "experimental": {
            "cache_file": {"enabled": True, "path": "<tmp>/generation-1/cache.db",
                           "cache_id": CACHE_ID, "store_fakeip": True, "store_dns": False},
            "clash_api": {"external_controller": "127.0.0.1:0", "secret": "<random-temporary-secret>"},
        },
    }


class Origin(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        body = b"p004-loopback-only\n"
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_):
        pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--archive", type=Path, help="Optional existing official archive; digest is still mandatory")
    args = parser.parse_args()
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        parser.error("Requires real macOS arm64")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    (output / "fixtures").mkdir(exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="veyra-p0-04-", dir="/private/tmp"))
    os.chmod(root, 0o700)
    secret = secrets.token_hex(32)
    local_username = getpass.getuser()
    processes = []
    runs = []
    checks = []
    origin = None
    status = "FAIL"
    cleanup = {}
    failure = None

    def clean(text):
        text = text.replace(secret, "<redacted>").replace(str(root), "<tmp>")
        text = text.replace(str(Path.cwd()), "<workspace>")
        text = re.sub(r"/Users/[^\s\"']+", "<user-path>", text)
        text = re.sub(r"(?<![\w.-])" + re.escape(local_username) + r"(?![\w.-])", "<user>", text)
        # The kernel prints host gateway addresses even without TUN. Do not export them.
        return re.sub(r"network: updated network environment:.*", "network: <host-network-redacted>", text)

    def save(name, value):
        data = clean(json.dumps(value, ensure_ascii=False, indent=2)) + "\n"
        assert secret not in data
        (output / name).write_text(data)

    def check(name, template, cache):
        actual = copy.deepcopy(template)
        actual["experimental"]["cache_file"]["path"] = str(cache)
        actual["experimental"]["clash_api"]["secret"] = secret
        path = root / (name + ".json")
        path.write_text(json.dumps(actual, indent=2) + "\n")
        path.chmod(0o600)
        (output / "fixtures" / (name + ".json")).write_text(json.dumps(template, indent=2) + "\n")
        result = command([str(binary), "--disable-color", "check", "-c", str(path)])
        checks.append({"fixture": "fixtures/" + name + ".json", "actual_config_sha256": sha(path),
                       "command": "sing-box --disable-color check -c <tmp>/" + name + ".json",
                       "exit_code": result.returncode, "output": clean(result.stdout + result.stderr),
                       "status": "PASS" if result.returncode == 0 else "FAIL", "timeout_seconds": 20})
        save("checks.json", checks)
        assert result.returncode == 0, "representative check failed: " + name
        return path

    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def api(run, endpoint, method="GET", body=None, authenticate=True):
        headers = {"Content-Type": "application/json"}
        if authenticate:
            headers["Authorization"] = "Bearer " + secret
        req = urllib.request.Request("http://" + run["endpoint"] + endpoint,
                                     data=None if body is None else json.dumps(body).encode(),
                                     headers=headers, method=method)
        try:
            with opener.open(req, timeout=3) as response:
                data = response.read()
                return response.status, json.loads(data) if data else None
        except urllib.error.HTTPError as error:
            return error.code, json.loads(error.read())

    def start(name, path, cache):
        log = []
        messages = queue.Queue()
        started = time.monotonic_ns()
        proc = subprocess.Popen([str(binary), "--disable-color", "run", "-c", str(path)],
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                text=True, start_new_session=True)
        processes.append(proc)
        run = {"name": name, "pid": proc.pid, "process_group": proc.pid,
               "config_sha256": sha(path), "started_at": utc(), "start_monotonic_ns": started,
               "controller_source": "this Popen child stdout/stderr only", "startup_timeout_seconds": 10,
               "cache_path": str(cache), "raw_output_log": name + "-child.log"}
        runs.append(run)

        def reader():
            for line in proc.stdout:
                log.append(clean(line))
                messages.put(line.rstrip())

        thread = threading.Thread(target=reader, daemon=True)
        thread.start()
        run_handles[name] = (proc, thread, log)
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if proc.poll() is not None:
                raise RuntimeError("child exited during startup")
            try:
                line = messages.get(timeout=.1)
            except queue.Empty:
                continue
            m = re.search(CONTROLLER, line)
            if m:
                assert 0 < int(m[2]) < 65536
                run["endpoint"] = m[1]
                run["controller_log_line"] = clean(line)
            m = re.search(MIXED, line)
            if m:
                run["mixed_port"] = int(m[1])
                run["mixed_log_line"] = clean(line)
            if "endpoint" in run and "mixed_port" in run:
                code, version = api(run, "/version")
                assert code == 200 and version["version"] == "sing-box 1.14.0"
                run["ready_monotonic_ns"] = time.monotonic_ns()
                run["controller_ready_at"] = utc()
                run["readiness"] = {"endpoint": "/version", "status": code, "body": version}
                assert api(run, "/version", authenticate=False)[0] == 401
                run["unauthenticated_status"] = 401
                break
        else:
            raise RuntimeError("dynamic controller discovery timed out")
        # Independent identity check AFTER child-log discovery; never searches other processes.
        result = command(["lsof", "-nP", "-a", "-p", str(proc.pid), "-iTCP", "-sTCP:LISTEN"])
        text = clean(result.stdout)
        run["owned_listener_check"] = {"command": "lsof -nP -a -p <owned-pid> -iTCP -sTCP:LISTEN",
                                       "exit_code": result.returncode, "output": text}
        assert result.returncode == 0 and run["endpoint"] + " (LISTEN)" in text
        assert "127.0.0.1:" + str(run["mixed_port"]) + " (LISTEN)" in text
        result = command(["lsof", "-nP", "-a", "-p", str(proc.pid), str(cache)])
        run["cache_open_check"] = {"exit_code": result.returncode, "output": clean(result.stdout)}
        assert result.returncode == 0 and str(cache) in result.stdout
        return run

    def stop(run):
        proc, thread, log = run_handles[run["name"]]
        if proc.poll() is None:
            proc.send_signal(signal.SIGTERM)
        proc.wait(timeout=10)
        thread.join(timeout=2)
        assert not thread.is_alive()
        proc.stdout.close()
        (output / run["raw_output_log"]).write_text("".join(log))
        run["stop"] = {"signal": "SIGTERM", "exit_code": proc.returncode, "reaped": True,
                       "stopped_at": utc(), "timeout_seconds": 10}
        assert proc.returncode == 0
        try:
            os.killpg(proc.pid, 0)
        except ProcessLookupError:
            run["stop"]["process_group_empty"] = True
        else:
            raise RuntimeError("owned process group still exists")
        for port in [int(run["endpoint"].rsplit(":", 1)[1]), run["mixed_port"]]:
            with socket.socket() as sock:
                sock.settimeout(.3)
                assert sock.connect_ex(("127.0.0.1", port)) != 0
            with socket.socket() as sock:
                sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                sock.bind(("127.0.0.1", port))
        run["stop"]["controller_and_mixed_released"] = True
        result = command(["lsof", "-t", run["cache_path"]])
        assert result.returncode == 1 and not result.stdout.strip()
        run["stop"]["cache_writer_closed"] = True

    def selector(run):
        code, body = api(run, "/proxies/p004-selector")
        assert code == 200 and body["type"] == "Selector"
        return body

    def fakeip(run, domain=DOMAIN):
        code, body = api(run, "/dns/query?name=" + domain + "&type=A")
        assert code == 200 and body["Status"] == 0
        value = body["Answer"][0]["data"]
        assert ipaddress.ip_address(value) in ipaddress.ip_network("198.18.0.0/15")
        return {"domain": domain, "status": code, "ip": value, "response": body}

    def access(run):
        # One HTTP request through the actual mixed inbound to this owned loopback origin.
        port = origin.server_address[1]
        with socket.create_connection(("127.0.0.1", run["mixed_port"]), timeout=3) as sock:
            sock.sendall(("GET http://127.0.0.1:" + str(port) + "/p004 HTTP/1.1\r\n"
                          "Host: 127.0.0.1:" + str(port) + "\r\nConnection: close\r\n\r\n").encode())
            data = b""
            while True:
                part = sock.recv(4096)
                if not part:
                    break
                data += part
        assert b"200 OK" in data and data.endswith(b"p004-loopback-only\n")
        return {"at": utc(), "monotonic_ns": time.monotonic_ns(),
                "target": "http://127.0.0.1:<owned-origin-port>/p004", "status": 200,
                "body": "p004-loopback-only", "public_access": False}

    def reconcile(run):
        # Demonstrate inbound traffic BEFORE reading actual selector / FakeIP state.
        run["pre_reconcile_loopback_access"] = access(run)
        run["restored_selector"] = selector(run)
        run["restored_fakeip"] = fakeip(run)
        assert run["restored_selector"]["now"] == "p004-direct-b"
        assert run["restored_fakeip"]["ip"] == target["ip"]
        run["reconciled_at"] = utc()
        run["reconcile_monotonic_ns"] = time.monotonic_ns()
        run["ready_to_reconcile_ms"] = (run["reconcile_monotonic_ns"] - run["ready_monotonic_ns"]) / 1e6
        run["pre_reconcile_traffic_window"] = True

    run_handles = {}
    try:
        archive = root / "kernel.tar.gz"
        if args.archive:
            shutil.copyfile(args.archive, archive)
        else:
            result = command(["curl", "-fL", "--connect-timeout", "15", "--max-time", "120",
                              "-o", str(archive), URL], timeout=130)
            assert result.returncode == 0, "official download failed"
        actual_digest = sha(archive)
        if actual_digest != DIGEST:
            status = "BLOCKED"
            save("kernel-identity.json", {"archive_sha256": actual_digest, "expected": DIGEST,
                                          "status": "BLOCKED", "binary_executed": False})
            raise RuntimeError("official archive digest mismatch; binary NOT RUN")
        with tarfile.open(archive) as bundle:
            bundle.extractall(root, filter="data")
        binary = root / "sing-box-1.14.0-darwin-arm64" / "sing-box"
        version = command([str(binary), "version"])
        file_info = command(["file", str(binary)])
        assert version.returncode == file_info.returncode == 0
        assert "sing-box version 1.14.0" in version.stdout and "darwin/arm64" in version.stdout
        assert "Mach-O 64-bit executable arm64" in file_info.stdout
        schema = command([str(binary), "schema"])
        assert schema.returncode == 0
        json.loads(schema.stdout)
        save("kernel-identity.json", {
            "repository": "SagerNet/sing-box", "tag": "v1.14.0",
            "published_at_host_verified": "2026-08-31T04:05:01Z", "download_url": URL,
            "asset": archive.name.replace("kernel.tar.gz", "sing-box-1.14.0-darwin-arm64.tar.gz"),
            "official_archive_sha256": DIGEST, "archive_sha256": actual_digest,
            "digest_match_before_execution": True, "binary_sha256": sha(binary),
            "file": clean(file_info.stdout), "version_stdout": version.stdout,
            "version_stderr": version.stderr, "schema_command_exit_code": schema.returncode,
            "schema_sha256": hashlib.sha256(schema.stdout.encode()).hexdigest(),
            "host": platform.platform(), "architecture": platform.machine(),
            "source_head": command(["git", "rev-parse", "HEAD"]).stdout.strip(), "recorded_at": utc(),
        })
        base = config()
        cache = root / "generation-1" / "cache.db"
        cache.parent.mkdir()
        runnable = check("loopback-smoke", base, cache)
        dns = copy.deepcopy(base)
        dns["inbounds"] = []
        check("dns-fakeip", dns, root / "dns-cache.db")
        tun = copy.deepcopy(base)
        tun["inbounds"] = [{"type": "tun", "tag": "p004-tun-check-only", "address": ["192.0.2.1/30"],
                            "auto_route": False, "stack": "system", "dns_mode": "disabled"}]
        check("tun-check-only", tun, root / "tun-cache.db")
        origin = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Origin)
        origin_thread = threading.Thread(target=origin.serve_forever, daemon=True)
        origin_thread.start()
        first = start("initial", runnable, cache)
        first["controlled_loopback_access"] = access(first)
        initial_selection = selector(first)
        assert initial_selection["now"] == "p004-direct-a"
        code, _ = api(first, "/proxies/p004-selector", "PUT", {"name": "p004-direct-b"})
        assert code == 204
        selected = selector(first)
        assert selected["now"] == "p004-direct-b"
        guard = fakeip(first, "p004-sentinel.example.invalid")
        target = fakeip(first)
        stable = fakeip(first)
        assert target["ip"] == stable["ip"] and guard["ip"] != target["ip"]
        first["selector_before"] = initial_selection
        first["selector_write_status"] = code
        first["selector_readback"] = selected
        first["fakeip_sentinel"] = guard
        first["fakeip_target"] = target
        first["fakeip_repeat"] = stable
        stop(first)
        second = start("restart-same-config", runnable, cache)
        reconcile(second)
        stop(second)
        stat = cache.stat()
        old = {"path": str(cache), "size": stat.st_size, "sha256": sha(cache),
               "mtime_ns": stat.st_mtime_ns, "writer_closed": second["stop"]["cache_writer_closed"]}
        assert old["writer_closed"]
        handoff_cache = root / "handoff" / "cache.db"
        handoff_cache.parent.mkdir()
        shutil.copy2(cache, handoff_cache)
        snapshot = {"path": str(handoff_cache), "size": handoff_cache.stat().st_size,
                    "sha256": sha(handoff_cache), "mtime_ns": handoff_cache.stat().st_mtime_ns,
                    "copied_at": utc(), "old_child_reaped_before_copy": second["stop"]["reaped"]}
        assert old["sha256"] == snapshot["sha256"]
        handoff_template = copy.deepcopy(base)
        handoff_template["experimental"]["cache_file"]["path"] = "<tmp>/handoff/cache.db"
        handoff_config = check("handoff-smoke", handoff_template, handoff_cache)
        third = start("closed-writer-handoff", handoff_config, handoff_cache)
        reconcile(third)
        stop(third)
        # Negative control: with no cache and the target queried first, allocation differs.
        cold_cache = root / "cold-control.db"
        cold_config = check("cold-control", base, cold_cache)
        cold = start("fresh-cache-control", cold_config, cold_cache)
        cold["selector"] = selector(cold)
        cold["fakeip_target_first"] = fakeip(cold)
        assert cold["selector"]["now"] == "p004-direct-a"
        assert cold["fakeip_target_first"]["ip"] != target["ip"]
        stop(cold)
        save("cache-recovery.json", {
            "status": "PASS", "selector_restore": "PASS", "fakeip_restore": "PASS", "handoff": "PASS",
            "cache_id": CACHE_ID, "cache_generation": "v1.14.0 / IPv4 198.18.0.0/15 / stable tags",
            "writer_close_before_copy": True, "old_closed_cache": old, "handoff_snapshot": snapshot,
            "fakeip_evidence": {"sentinel": guard, "target": target, "repeat": stable,
                                "dns_response_cache_disabled": True, "store_dns": False,
                                "restart_target_is_first_query": True,
                                "fresh_cache_target": cold["fakeip_target_first"]},
            "runs": runs, "conclusion": "存在对账前流量窗口",
            "p2_rule": "selector/cache 对账完成前不报业务 Ready，不打开新的系统代理接管；inbound gate 留给后续 Runtime。",
            "compatibility": "Only same binary/version, cache_id/generation, outbound tags and pool. No cross-version/owner/changed-tag guarantee.",
        })
        status = "PASS"
    except Exception as error:
        failure = {"type": type(error).__name__, "message": clean(str(error)), "at": utc()}
    finally:
        forced = []
        for proc in processes:
            if proc.poll() is None:
                proc.send_signal(signal.SIGTERM)
                try:
                    proc.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(proc.pid, signal.SIGKILL)
                    proc.wait(timeout=5)
                    forced.append(proc.pid)
        for name, (proc, thread, log) in run_handles.items():
            thread.join(timeout=2)
            (output / (name + "-child.log")).write_text("".join(log))
            proc.stdout.close()
        if origin:
            origin.shutdown()
            origin.server_close()
            origin_thread.join(timeout=2)
        groups_empty = True
        for proc in processes:
            try:
                os.killpg(proc.pid, 0)
                groups_empty = False
            except ProcessLookupError:
                pass
        cleanup = {"child_count": len(processes), "all_children_reaped": all(p.returncode is not None for p in processes),
                   "all_owned_process_groups_empty": groups_empty, "forced_kill_pids": forced,
                   "origin_closed": origin is not None, "temporary_directory_removed": False}
        # Do not delete resources while any writer might still be alive.
        if groups_empty:
            shutil.rmtree(root)
            cleanup["temporary_directory_removed"] = not root.exists()
        if forced or not groups_empty:
            status = "FAIL"
        save("controller-dynamic-port.json", {
            "status": "PASS" if status == "PASS" else status, "requested": "127.0.0.1:0",
            "regex": CONTROLLER, "startup_timeout_seconds": 10, "request_timeout_seconds": 3,
            "authority": "owned child stdout/stderr; lsof only independent PID-scoped verification after parsing",
            "dynamic_port_supported": status == "PASS", "runs": runs,
            "fallback_implemented": False, "failure": failure,
        })
        save("probe-result.json", {"task": "OBG-P0-04", "status": status, "completed_at": utc(),
                                   "cleanup": cleanup, "failure": failure, "runs_completed": len(runs),
                                   "system_proxy_changed": False, "system_dns_changed": False,
                                   "tun_started": False, "public_connectivity_tested": False})
    print(json.dumps({"status": status, "cleanup": cleanup, "failure": failure}, ensure_ascii=False))
    return 0 if status == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
