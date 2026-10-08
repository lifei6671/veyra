#!/usr/bin/env python3
"""P0-06 只读网络身份快照。绝不启动/停止 TUN 或写系统网络配置。"""
import datetime
import hashlib
import json
import pathlib
import plistlib
import re
import socket
import subprocess
import sys


def run(args):
    value = subprocess.run(args, capture_output=True, text=True, timeout=10)
    return value.returncode, value.stdout


def digest(value):
    if not isinstance(value, bytes):
        value = value.encode()
    return hashlib.sha256(value).hexdigest()


def snapshot():
    _, names = run(["/sbin/ifconfig", "-l"])
    utuns = []
    for name in names.split():
        if name.startswith("utun"):
            _, text = run(["/sbin/ifconfig", name])
            # 不导出用户地址；名称/index 和完整配置摘要可独立比较资源身份。
            utuns.append({"name": name, "index": socket.if_nametoindex(name),
                          "configuration_sha256": digest(text)})
    routes = {}
    for family, args in [("ipv4", []), ("ipv6", ["-inet6"])]:
        code, text = run(["/sbin/route", "-n", "get", *args, "default"])
        match = re.search(r"interface:\s*(\S+)", text)
        # route get 的 RTT/use 等动态计数不属于路由身份。
        stable = "\n".join(line.strip() for line in text.splitlines()
                           if re.match(r"\s*(route to|destination|mask|gateway|interface|flags):", line))
        routes[family] = {"exit_code": code, "interface": match[1] if match else None,
                          "identity_sha256": digest(stable)}
    _, route_table = run(["/usr/sbin/netstat", "-rn"])
    _, dns = run(["/usr/sbin/scutil", "--dns"])
    # DNS完整配置只留hash；显式测试upstream/结果在Rust probe另行记录。
    dns_servers = re.findall(r"nameserver\[\d+\]\s*:\s*(\S+)", dns)
    _, proxy = run(["/usr/sbin/scutil", "--proxy"])
    proxy_flags = {k: int(v) for k, v in re.findall(r"(\w+(?:Enable|Port))\s*:\s*(\d+)", proxy)}
    preferences = plistlib.loads(pathlib.Path(
        "/Library/Preferences/SystemConfiguration/preferences.plist").read_bytes())
    hashes = {k: digest(plistlib.dumps(preferences.get(k), sort_keys=True))
              for k in ["NetworkServices", "Sets", "CurrentSet"]}
    services = [{"service_id_sha256": digest(k), "disabled": bool(v.get("__INACTIVE__")),
                 "proxies_sha256": digest(plistlib.dumps(v.get("Proxies", {}), sort_keys=True))}
                for k, v in preferences.get("NetworkServices", {}).items()]
    _, processes = run(["/bin/ps", "-axo", "pid=,lstart=,comm="])
    owners = []
    for line in processes.splitlines():
        if re.search(r"sing-box|clash|mihomo|verge|veyra|surge|tailscale|tun2socks|karing|warp|networkextension|vpn", line, re.I):
            fields = line.split(maxsplit=6)
            if len(fields) == 7 and "node_modules" not in fields[6]:
                path = fields[6]
                owners.append({"pid": int(fields[0]), "started": " ".join(fields[1:6]),
                               "executable_basename": pathlib.Path(path).name,
                               "executable_path_sha256": digest(path)})
    # utun本身不证明所有接口的owner；default=utun才是本轮外部TUN活跃事实。
    active = any((v["interface"] or "").startswith("utun") for v in routes.values())
    return {"recorded_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "resource_classification": "PROTECTED_EXTERNAL_RESOURCE",
            "external_tun_active": active, "utun_interfaces": utuns,
            "default_routes": routes, "route_table_sha256": digest(route_table),
            "dns_sha256": digest(dns), "dns_server_hashes": sorted(set(map(digest, dns_servers))),
            "system_proxy": proxy_flags, "system_proxy_sha256": digest(proxy),
            "network_hashes": hashes, "network_services": services,
            "possible_external_owners": owners,
            "owner_attribution": "candidate processes only; no ownership takeover or port scan",
            "network_mutations": 0}


if __name__ == "__main__":
    result = snapshot()
    pathlib.Path(sys.argv[1]).write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"external_tun_active": result["external_tun_active"],
                      "utun_interfaces": result["utun_interfaces"],
                      "default_routes": result["default_routes"],
                      "system_proxy": result["system_proxy"],
                      "possible_external_owners": result["possible_external_owners"]}))
