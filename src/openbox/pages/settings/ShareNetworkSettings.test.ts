import { describe, expect, it } from "vitest";
import type { SharedServer } from "../../api/types";
import { changeServerProtocol, generateServerPassword, serverDraft, serverHasLocalAddress, serverPortError, serverShareLink, saveServerDraft } from "./ShareNetworkSettings.helpers";

const server: SharedServer = { id: "home", name: "家", enabled: false, protocol: "shadowsocks", address: "example.com", port: 8388, method: "aes-256-gcm", password: "sample" };

describe("shared network server contracts", () => {
  it("creates the original defaults and restores existing servers without enabling them", () => {
    expect(serverDraft(undefined, "panel.example")).toMatchObject({ protocol: "shadowsocks", port: "8388", address: "panel.example", enabled: true, method: "aes-256-gcm", tls: true });
    expect(serverDraft(server, "panel.example")).toMatchObject({ ...server, port: "8388" });
    expect(serverDraft({ ...server, protocol: "mixed", address: "", password: undefined }, "panel.example")).toMatchObject({ address: "panel.example", username: "", password: "" });
  });
  it("switches protocol ports, clears mixed auth and generates credentials when switching back", () => {
    const draft = serverDraft(server, "panel.example");
    for (const [protocol, port] of [["vless", "8443"], ["tuic", "8444"], ["hysteria2", "8445"], ["mixed", "7080"]] as const) expect(changeServerProtocol(draft, protocol).port).toBe(port);
    const mixed = changeServerProtocol(draft, "mixed");
    expect(mixed).toMatchObject({ username: "", password: "" });
    expect(changeServerProtocol(mixed, "shadowsocks").password).toBeTruthy();
    expect(changeServerProtocol(draft, "tuic").password).toBe("sample");
  });
  it("validates required fields, ports, duplicates and paired mixed auth before the port-check API", () => {
    const draft = serverDraft(server, "panel.example");
    expect(() => saveServerDraft({ ...draft, name: " " }, [])).toThrow("备注不能为空");
    for (const port of ["", "0", "65536", "1.5", "bad"]) expect(() => saveServerDraft({ ...draft, port }, [])).toThrow("端口要在 1 到 65535 之间");
    expect(() => saveServerDraft(draft, [8388])).toThrow("已被另一台服务器占用");
    expect(() => saveServerDraft({ ...draft, password: " " }, [])).toThrow("密码 / UUID 不能为空");
    expect(() => saveServerDraft({ ...draft, protocol: "vless", uuid: " " }, [])).toThrow("密码 / UUID 不能为空");
    const mixed = changeServerProtocol(draft, "mixed");
    expect(() => saveServerDraft({ ...mixed, username: "user" }, [])).toThrow("用户名和密码要一起填");
    expect(saveServerDraft(mixed, [])).toEqual({ id: "home", name: "家", enabled: false, protocol: "mixed", address: "example.com", port: 7080 });
  });
  it("saves only fields for the selected protocol, retaining disabled state and trimming credentials", () => {
    const draft = { ...serverDraft(server, "panel.example"), name: " 家 ", address: " example.com ", password: " sample ", uuid: " uuid ", obfs: " obfs " };
    expect(saveServerDraft(draft, [])).toEqual(server);
    expect(saveServerDraft({ ...draft, protocol: "vless", tls: false }, [])).toEqual({ id: "home", enabled: false, name: "家", address: "example.com", port: 8388, protocol: "vless", uuid: "uuid", tls: false });
    expect(saveServerDraft({ ...draft, protocol: "hysteria2" }, [])).toMatchObject({ password: "sample", obfs: "obfs" });
    expect(saveServerDraft({ ...draft, protocol: "tuic" }, [])).toMatchObject({ password: "sample", uuid: "uuid" });
  });
  it("generates the original five share URI formats, including Unicode, IPv6, TLS and obfuscation", () => {
    expect(serverShareLink(server)).toBe(`ss://${btoa("aes-256-gcm:sample")}@example.com:8388#%E5%AE%B6`);
    const unicode = serverShareLink({ ...server, password: "测试" });
    expect(new TextDecoder().decode(Uint8Array.from(atob(unicode.slice(5).split("@")[0]), char => char.charCodeAt(0)))).toBe("aes-256-gcm:测试");
    expect(serverShareLink({ ...server, protocol: "vless", uuid: "uuid", address: "2001:db8::1", tls: true })).toBe("vless://uuid@[2001:db8::1]:8388?encryption=none&security=tls&sni=open-box.local&allowInsecure=1&type=tcp#%E5%AE%B6");
    expect(serverShareLink({ ...server, protocol: "vless", uuid: "uuid", tls: false })).toContain("security=none&type=tcp");
    expect(serverShareLink({ ...server, protocol: "tuic", uuid: "uuid", password: "p:@" })).toContain("tuic://uuid:p%3A%40@example.com:8388?congestion_control=bbr&alpn=h3&allow_insecure=1&sni=open-box.local");
    expect(serverShareLink({ ...server, protocol: "hysteria2", obfs: "p @" })).toContain("/?insecure=1&sni=open-box.local&obfs=salamander&obfs-password=p%20%40");
    expect(serverShareLink({ ...server, protocol: "mixed", username: "u@", password: "p:" })).toBe("socks5://u%40:p%3A@example.com:8388#%E5%AE%B6");
    expect(serverShareLink({ ...server, address: " " })).toBe("");
  });
  it("generates URL-safe credentials and 32-byte base64 keys for Shadowsocks 2022", () => {
    expect(generateServerPassword()).toMatch(/^[A-Za-z0-9_-]{22}$/);
    expect(atob(generateServerPassword("2022-blake3-aes-256-gcm"))).toHaveLength(32);
    expect(generateServerPassword("", 12)).toMatch(/^[A-Za-z0-9_-]{16}$/);
  });
  it("explains backend port rejection and warns only for local share addresses", () => {
    expect(serverPortError({ ok: true }, 8388)).toBe("");
    expect(serverPortError({ ok: false, reason: "reserved" }, 8388)).toContain("面板 / 内核自用");
    expect(serverPortError({ ok: false, reason: "server" }, 8388)).toContain("另一台服务器");
    expect(serverPortError({ ok: false, reason: "listening" }, 8388)).toContain("其它服务");
    for (const address of ["localhost", "router.lan", "router.home.arpa", "10.0.0.1", "127.0.0.1", "192.168.1.1", "172.16.0.1", "169.254.1.1", "100.64.0.1", "0.0.0.0", "[::1]", "::", "fd00::1", "fe80::1", "::ffff:192.168.1.1"]) expect(serverHasLocalAddress(address), address).toBe(true);
    for (const address of ["", "openbox.disign.me", "8.8.8.8", "172.32.0.1", "100.128.0.1", "2001:4860:4860::8888", "::ffff:8.8.8.8"]) expect(serverHasLocalAddress(address), address).toBe(false);
  });
});
