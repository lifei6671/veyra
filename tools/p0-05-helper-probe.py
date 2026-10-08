#!/usr/bin/env python3
"""Isolated P0-05 orchestration; standard OS authorization, never passwords.

Build first with MACOSX_DEPLOYMENT_TARGET=15.0. This downloads and verifies
the fixed archive as the ordinary user. The root stage executes only the
reviewable Rust fixed-resource harness. Never run on pre-existing P005 paths.
"""
import argparse
import ctypes
import getpass
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import selectors
import shlex
import shutil
import signal
import socket
import subprocess
import tarfile
import tempfile
import time
import urllib.request

URL = "https://github.com/SagerNet/sing-box/releases/download/v1.14.0/sing-box-1.14.0-darwin-arm64.tar.gz"
DIGEST = "a150c94012ff768b7261939cd236b9c8554127f45137230295d23a5660225cc9"
BINARY_DIGEST = "973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4"
LABEL = "com.lifei6671.veyra.p005"
ROOT = Path("/Library/Application Support/VeyraP005")
HELPER = ROOT / "helper"
PLIST = Path("/Library/LaunchDaemons") / (LABEL + ".plist")
REPO = Path(__file__).resolve().parents[1]


def existing_protected_resources():
    # 安装预检与卸载后残留判定共用固定清单；悬空 symlink 不能当作不存在。
    return [str(resource) for resource in (ROOT, HELPER, PLIST)
            if resource.exists() or resource.is_symlink()]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def process_identity(owned_pid):
    # sys/proc_info.h, PROC_PIDT_SHORTBSDINFO (13), fixed owned Popen child only.
    class Info(ctypes.Structure):
        _fields_ = [(name, ctypes.c_uint32) for name in ["pid", "ppid", "pgid", "status"]] + [
            ("comm", ctypes.c_char * 16)] + [(name, ctypes.c_uint32) for name in
            ["flags", "uid", "gid", "ruid", "rgid", "svuid", "svgid", "reserved"]]
    library = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
    library.proc_pidinfo.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_uint64, ctypes.c_void_p, ctypes.c_int]
    library.proc_pidinfo.restype = ctypes.c_int
    info = Info()
    size = library.proc_pidinfo(owned_pid, 13, 0, ctypes.byref(info), ctypes.sizeof(info))
    if size != ctypes.sizeof(info):
        raise OSError(ctypes.get_errno(), "proc_pidinfo owned child identity failed")
    return {"api": "proc_pidinfo PROC_PIDT_SHORTBSDINFO", **{key: getattr(info, key) for key in ["pid", "pgid", "uid", "ruid", "gid", "rgid"]}}


def command(args, timeout=30):
    result = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
    return {"command": args, "code": result.returncode,
            "stdout": result.stdout, "stderr": result.stderr}


def manual(kernel, staging):
    """Protect refusal fallback: authenticated loopback child without helper.

    No SystemConfiguration writes, public targets, cache, DNS upstream or TUN.
    """
    config = staging / "manual.json"
    config.write_text(json.dumps({"log": {"level": "info"},
        "inbounds": [{"type": "mixed", "tag": "p005-manual", "listen": "127.0.0.1", "listen_port": 0}],
        "outbounds": [{"type": "direct", "tag": "p005-direct"}],
        "route": {"rules": [{"ip_cidr": ["127.0.0.0/8", "::1/128"], "action": "route", "outbound": "p005-direct"}, {"action": "reject"}]},
        "experimental": {"clash_api": {"external_controller": "127.0.0.1:0", "secret": os.urandom(32).hex()}}}))
    config.chmod(0o600)
    check = command([str(kernel), "check", "-c", str(config)], 20)
    if check["code"]:
        raise RuntimeError("fixed manual config check failed")
    inherited = config.open("rb")
    child = subprocess.Popen([str(kernel), "run", "-c", f"/dev/fd/{inherited.fileno()}"],
        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        pass_fds=(inherited.fileno(),), start_new_session=True)
    inherited.close()
    ports = {}
    result = {"config_check": "PASS", "uid": os.getuid(), "gid": os.getgid(),
              "helper_absent": not (HELPER.exists() or HELPER.is_symlink()), "system_proxy_writes": 0,
              "kernel_sha256": sha(kernel), "child_pid": child.pid, "config_input": "/dev/fd/<inherited-fd>",
              "root_only_resource_read": "NOT_RUN; ordinary-user-owned 0600 compatibility probe"}
    try:
        with selectors.DefaultSelector() as events:
            events.register(child.stdout, selectors.EVENT_READ)
            output = b""
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline and len(ports) < 2:
                if events.select(.2):
                    data = os.read(child.stdout.fileno(), 4096)
                    if not data:
                        raise RuntimeError("manual child exited before readiness")
                    output += data
                    if len(output) > 65536:
                        raise RuntimeError("manual child startup output budget exceeded")
                    for line in output.decode(errors="replace").splitlines():
                        matched = re.search(r"127\.0\.0\.1:([1-9][0-9]{0,4})$", line)
                        if matched and "inbound/mixed[p005-manual]: tcp server started at" in line:
                            ports["mixed"] = int(matched[1])
                        if matched and "clash-api: restful api listening at" in line:
                            ports["controller"] = int(matched[1])
            if len(ports) != 2:
                raise RuntimeError("manual child readiness timeout")
        secret = json.loads(config.read_text())["experimental"]["clash_api"]["secret"]
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        req = urllib.request.Request(f"http://127.0.0.1:{ports['controller']}/version",
                                     headers={"Authorization": "Bearer " + secret})
        with opener.open(req, timeout=3) as response:
            assert response.status == 200
            result["controller"] = {"status": response.status, "body": json.load(response)}
        with socket.create_connection(("127.0.0.1", ports["mixed"]), timeout=3):
            result["mixed_listener_connect"] = "PASS"
        result["ports"] = ports
        result["process_identity"] = process_identity(child.pid)
        for key in ["uid", "ruid"]:
            assert result["process_identity"][key] == os.getuid()
        for key in ["gid", "rgid"]:
            assert result["process_identity"][key] == os.getgid()
        result["status"] = "PASS"
    finally:
        if child.poll() is None:
            os.killpg(child.pid, signal.SIGTERM)
        child.wait(timeout=10)
        result["exit_code"] = child.returncode
        result["reaped"] = True
        try:
            os.killpg(child.pid, 0)
            raise RuntimeError("manual child group remains")
        except ProcessLookupError:
            result["process_group_empty"] = True
        for port in ports.values():
            try:
                with socket.create_connection(("127.0.0.1", port), timeout=.2):
                    raise RuntimeError("manual child listener remains")
            except (ConnectionRefusedError, TimeoutError):
                pass
        result["listeners_closed"] = True
        config.unlink()
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--no-authorize", action="store_true", help="ordinary-user evidence only; never counts as privileged acceptance")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if list(output.iterdir()):
        raise RuntimeError("use a new empty evidence directory")
    if existing_protected_resources():
        raise RuntimeError("pre-existing P005 resources; no mutation or cleanup")
    artifact = REPO / "target/debug/veyra-helper-prototype"
    original_uid, original_gid = os.getuid(), os.getgid()
    username = getpass.getuser()
    staging = Path(tempfile.mkdtemp(prefix="veyra-p005-", dir="/private/tmp"))
    staging.chmod(0o700)
    recovery_required = False

    def clean(value, key=""):
        if isinstance(value, dict):
            return {k: clean(v, k) for k, v in value.items()}
        if isinstance(value, list):
            return [clean(item, key) for item in value]
        if isinstance(value, int) and key in {"uid", "ruid", "gid", "rgid"}:
            if value == 0:
                return 0
            if value == original_uid and key in {"uid", "ruid"}:
                return "<authorized-uid>"
            if value == original_gid and key in {"gid", "rgid"}:
                return "<authorized-gid>"
            return "<unauthorized-uid>" if key in {"uid", "ruid"} else "<system-gid>"
        if isinstance(value, str):
            # Some fixed-command outputs contain nested JSON; sanitize semantically.
            if value.lstrip().startswith("{"):
                try:
                    return json.dumps(clean(json.loads(value)), ensure_ascii=False)
                except json.JSONDecodeError:
                    pass
            value = value.replace(str(staging), "<tmp>").replace(str(REPO), "<repo>").replace(str(Path.home()), "<home>")
            value = re.sub(r"(?<![A-Za-z0-9_])" + re.escape(username) + r"(?![A-Za-z0-9_])", "<user>", value)
            value = re.sub(r"(?<!\d)" + str(original_uid) + r"(?!\d)", "<authorized-uid>", value)
            value = re.sub(r"(?<!\d)" + str(original_gid) + r"(?!\d)", "<authorized-gid>", value)
            # sing-box emits host network defaults even with no TUN; don't export.
            value = re.sub(r"(?m)^.*(?:default interface|network: updated default).*$", "<host-network-redacted>", value)
        return value

    def save(name, value):
        (output / name).write_text(json.dumps(clean(value), indent=2, ensure_ascii=False) + "\n")

    try:
        shutil.copy2(artifact, staging / "helper")
        save("local-os-api.json", command([str(artifact), "local-platform-probe"], 15))
        save("platform-identity.json", {"rustc": command(["rustc", "-V"]), "cargo": command(["cargo", "-V"]),
            "host": command(["sw_vers"]), "architecture": platform.machine(), "deployment_target": "15.0",
            "macOS_15_device": "NOT_RUN", "helper_client_same_artifact": True, "helper_sha256": sha(artifact),
            "macho": command(["otool", "-l", str(artifact)]),
            "codesign_display": command(["codesign", "-dv", "--verbose=4", str(artifact)]),
            "codesign_verify": command(["codesign", "--verify", "--verbose=4", str(artifact)])})
        archive = staging / "kernel.tar.gz"
        download = command(["curl", "-fL", "--connect-timeout", "15", "--max-time", "120", "-o", str(archive), URL], 130)
        if download["code"] or sha(archive) != DIGEST:
            raise RuntimeError("official archive download/digest failed; kernel NOT RUN")
        with tarfile.open(archive) as bundle:
            member = bundle.getmember("sing-box-1.14.0-darwin-arm64/sing-box")
            if not member.isfile():
                raise RuntimeError("archive kernel is not a regular file")
            with bundle.extractfile(member) as source:
                (staging / "sing-box").write_bytes(source.read())
        kernel = staging / "sing-box"
        kernel.chmod(0o755)
        if sha(kernel) != BINARY_DIGEST:
            raise RuntimeError("binary digest differs from P0-04 identity")
        save("kernel-identity.json", {"url": URL, "archive_sha256": sha(archive), "binary_sha256": sha(kernel),
             "digest_match_before_execution": True, "root_downloads": 0, "version": command([str(kernel), "version"])})
        denied = command([str(staging / "helper"), "install", str(original_uid), sha(staging / "helper")])
        assert denied["code"] != 0 and "AdministratorRequired" in denied["stderr"]
        save("manual-without-helper.json", manual(kernel, staging))
        installation = {"ordinary_install": denied, "protected_resources_created_by_denied_install": False,
            "real_user_cancel": "NOT_RUN", "cancel_evidence": "non-root installer rejection; not a cancelled authorization dialog"}
        assert not existing_protected_resources()
        if args.no_authorize:
            installation["system_authorization"] = {"status": "NOT_RUN", "reason": "--no-authorize"}
        else:
            fixed_command = shlex.join([str(staging / "helper"), "privileged-probe", str(original_uid), sha(staging / "helper")])
            apple_script = "do shell script " + json.dumps(fixed_command) + " with administrator privileges"
            # Timeout covers the OS authorization UI. No password/account enters
            # stdin, argv, files or evidence. Only the administrator owns the UI.
            authorization = command(["/usr/bin/osascript", "-e", apple_script], 240)
            installation["system_authorization"] = authorization
            if authorization["code"] == 0:
                privileged = json.loads(authorization["stdout"])
                installation["privileged_probe"] = privileged
                recovery_required = privileged.get("cleanup", {}).get("status") == "RecoveryRequired"
                save("ipc-peer-identity.json", {"server_api": ["getpeereid", "getsockopt SOL_LOCAL/LOCAL_PEERPID"],
                    "cycles": privileged.get("cycles", "NOT_RUN"), "unauthorized_uid": privileged.get("unauthorized_uid", "NOT_RUN"),
                    "invalid_requests": privileged.get("invalid_requests", "NOT_RUN"), "server_peer_check": privileged.get("server_peer_check", "NOT_RUN")})
                save("network-service.json", {k: privileged.get(k, "NOT_RUN") for k in ["snapshot", "active_readback", "two_cycle_exact_restore", "external_conflict"]})
                save("owner-lifecycle.json", privileged.get("owner_lifecycle", {"status": "NOT_RUN"}))
                save("child-identity.json", {"lifecycle": privileged.get("owner_lifecycle", "NOT_RUN"), "config": privileged.get("config_permissions", "NOT_RUN")})
            else:
                for name in ["ipc-peer-identity", "network-service", "owner-lifecycle", "child-identity"]:
                    save(name + ".json", {"status": "NOT_RUN", "reason": "system administrator authorization did not run privileged harness"})
                if "User canceled" in authorization["stderr"] or "(-128)" in authorization["stderr"]:
                    installation["real_user_cancel"] = "OBSERVED_OS_CANCEL_ERROR_NOT_USER_ATTESTED"
        save("install-uninstall.json", installation)
    except (Exception, KeyboardInterrupt) as error:
        save("probe-error.json", {"error": str(error), "type": type(error).__name__})
        raise
    finally:
        # The root harness has its own finally restoration/stop/uninstall. Ordinary
        # orchestration never tries to bypass permissions to remove protected state.
        residual = existing_protected_resources()
        if residual:
            recovery_required = True
        shutil.rmtree(staging)
        launchd = command(["/bin/launchctl", "print", "system/" + LABEL], 10)
        save("cleanup.json", {"status": "RecoveryRequired" if recovery_required else "PASS", "protected_residual": residual,
            "launchd_exact_label_absent": launchd["code"] != 0, "launchd": launchd,
            "temporary_staging_removed": not staging.exists(), "staging_archive_config_kernel_removed": True})
    print(json.dumps({"output": str(output), "recovery_required": recovery_required}))


if __name__ == "__main__":
    main()
