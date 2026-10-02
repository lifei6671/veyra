import type { ServerPortCheck, SharedServer } from "../../api/types";

export const SERVER_PROTOCOLS = [{ value: "shadowsocks", label: "Shadowsocks" }, { value: "vless", label: "VLESS" }, { value: "tuic", label: "TUIC" }, { value: "hysteria2", label: "Hysteria2" }, { value: "mixed", label: "SOCKS5 / HTTP" }];
export const SERVER_METHODS = ["aes-256-gcm", "aes-128-gcm", "chacha20-ietf-poly1305", "2022-blake3-aes-256-gcm"];
const DEFAULT_PORTS = { shadowsocks: 8388, vless: 8443, tuic: 8444, hysteria2: 8445, mixed: 7080 };
export type ServerDraft = Omit<SharedServer, "port"> & { port: string };

export function generateServerPassword(method = "", bytes = 16) {
  const data = crypto.getRandomValues(new Uint8Array(method === "2022-blake3-aes-256-gcm" ? 32 : bytes));
  const base64 = btoa(String.fromCharCode(...data));
  return method === "2022-blake3-aes-256-gcm" ? base64 : base64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export function serverDraft(server?: SharedServer, hostname = location.hostname): ServerDraft {
  return { id: `s${Date.now().toString(36)}${Math.floor(Math.random() * 1e6).toString(36)}`, enabled: true, name: "", protocol: "shadowsocks", password: generateServerPassword(), method: "aes-256-gcm", uuid: crypto.randomUUID(), tls: true, obfs: "", username: "", ...server, port: String(server?.port ?? 8388), address: server?.address || hostname, ...(server?.protocol === "mixed" ? { username: server.username || "", password: server.password || "" } : {}) };
}

export function changeServerProtocol(draft: ServerDraft, protocol: SharedServer["protocol"]): ServerDraft {
  const next = { ...draft, protocol, port: String(DEFAULT_PORTS[protocol]) };
  if (protocol === "mixed") return { ...next, username: "", password: "" };
  if (protocol === "shadowsocks" && !next.method) next.method = "aes-256-gcm";
  if (!next.password) next.password = generateServerPassword(protocol === "shadowsocks" ? next.method : "");
  if (!next.uuid) next.uuid = crypto.randomUUID();
  return next;
}

export function saveServerDraft(draft: ServerDraft, usedPorts: number[]): SharedServer {
  const name = draft.name.trim(), port = Number(draft.port);
  if (!name) throw new Error("备注不能为空");
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error("端口要在 1 到 65535 之间");
  if (usedPorts.includes(port)) throw new Error(`端口 ${port} 已被另一台服务器占用`);
  const server: SharedServer = { id: draft.id, enabled: draft.enabled !== false, name, protocol: draft.protocol, port, address: draft.address.trim() };
  if (draft.protocol === "shadowsocks") { server.method = draft.method || "aes-256-gcm"; server.password = (draft.password || "").trim(); }
  if (draft.protocol === "vless") { server.uuid = (draft.uuid || "").trim(); server.tls = draft.tls !== false; }
  if (draft.protocol === "tuic") { server.uuid = (draft.uuid || "").trim(); server.password = (draft.password || "").trim(); }
  if (draft.protocol === "hysteria2") { server.password = (draft.password || "").trim(); if (draft.obfs?.trim()) server.obfs = draft.obfs.trim(); }
  if (draft.protocol === "mixed") {
    const username = (draft.username || "").trim(), password = (draft.password || "").trim();
    if (!!username !== !!password) throw new Error("用户名和密码要一起填,或者都留空");
    if (username) { server.username = username; server.password = password; }
  }
  if (server.password !== undefined && !server.password || server.uuid !== undefined && !server.uuid) throw new Error("密码 / UUID 不能为空");
  return server;
}

export function serverShareLink(server: SharedServer | ServerDraft) {
  const address = server.address.trim();
  if (!address) return "";
  const host = address.includes(":") && !address.startsWith("[") ? `[${address}]` : address;
  const name = encodeURIComponent(server.name || server.id), password = encodeURIComponent(server.password || "");
  switch (server.protocol) {
    case "shadowsocks": {
      const bytes = new TextEncoder().encode(`${server.method || ""}:${server.password || ""}`);
      return `ss://${btoa(String.fromCharCode(...bytes))}@${host}:${server.port}#${name}`;
    }
    case "vless": return `vless://${server.uuid || ""}@${host}:${server.port}?encryption=none&security=${server.tls ? "tls&sni=open-box.local&allowInsecure=1" : "none"}&type=tcp#${name}`;
    case "tuic": return `tuic://${encodeURIComponent(server.uuid || "")}:${password}@${host}:${server.port}?congestion_control=bbr&alpn=h3&allow_insecure=1&sni=open-box.local#${name}`;
    case "hysteria2": return `hysteria2://${password}@${host}:${server.port}/?insecure=1&sni=open-box.local${server.obfs ? `&obfs=salamander&obfs-password=${encodeURIComponent(server.obfs)}` : ""}#${name}`;
    case "mixed": return `socks5://${server.username ? `${encodeURIComponent(server.username)}:${password}@` : ""}${host}:${server.port}#${name}`;
  }
}

export function serverPortError(check: ServerPortCheck, port: number) {
  if (check.ok) return "";
  if (check.reason === "reserved") return `端口 ${port} 是面板 / 内核自用端口`;
  if (check.reason === "server") return `端口 ${port} 已被另一台服务器占用`;
  if (check.reason === "listening") return `端口 ${port} 已被路由器上其它服务占用`;
  return "端口要在 1 到 65535 之间";
}

export function serverHasLocalAddress(address: string): boolean {
  const host = address.trim().toLowerCase().replace(/^\[|\]$/g, "");
  if (!host) return false;
  const parts = host.split(".").map(Number);
  if (/^(0|[1-9]\d{0,2})(\.(0|[1-9]\d{0,2})){3}$/.test(host) && parts.every(part => part <= 255)) {
    const [first, second] = parts;
    return first === 0 || first === 10 || first === 127 || first === 169 && second === 254 || first === 172 && second >= 16 && second <= 31 || first === 192 && second === 168 || first === 100 && second >= 64 && second <= 127;
  }
  if (host.includes(":")) {
    try {
      const ipv6 = new URL(`http://[${host}]/`).hostname.slice(1, -1);
      if (ipv6.startsWith("::ffff:")) {
        const words = ipv6.slice(7).split(":").map(word => parseInt(word, 16));
        return serverHasLocalAddress(`${words[0] >> 8}.${words[0] & 255}.${words[1] >> 8}.${words[1] & 255}`);
      }
      return ipv6 === "::" || ipv6 === "::1" || /^f[cd]/.test(ipv6) || /^fe[89ab]/.test(ipv6);
    } catch { /* A non-IP address follows the original hostname check. */ }
  }
  return !host.includes(".") || /\.(lan|local|home|internal|localdomain|home\.arpa)$/.test(host);
}
