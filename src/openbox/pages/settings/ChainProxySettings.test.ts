import { describe, expect, it } from "vitest";
import type { ChainProxy, GroupsResponse } from "../../api/types";
import { chainConnection, chainDraft, chainIpDetailsUrl, chainLatencyRequest, chainLocation, chainTestResult, chainUpstreamOptions, parseChainFields, saveChainDraft } from "./ChainProxySettings.helpers";

const item: ChainProxy = { id: "residential", enabled: false, name: "住宅-美国", link: "http://test:sample@example.com:1080", upstream: "所有-自动", node: { type: "http", server: "example.com", port: 1080 } };

describe("chain proxy editor and test contracts", () => {
  it("opens editable HTTP/SOCKS credentials in fields mode while preserving disabled state", () => {
    const draft = chainDraft(item);
    expect(draft).toMatchObject({ mode: "fields", enabled: false, fields: { scheme: "http", host: "example.com", port: "1080", username: "test", password: "sample" } });
    expect(saveChainDraft(draft)).toEqual({ id: item.id, enabled: false, name: item.name, link: item.link, upstream: item.upstream });
    expect(parseChainFields("SOCKS5H://u%40ser:p%3Aa%40ss@[2001:db8::1]:1080/path")).toEqual({ scheme: "socks5", host: "[2001:db8::1]", port: "1080", username: "u@ser", password: "p:a@ss" });
    expect(parseChainFields("https://user@example.com:443")).toMatchObject({ scheme: "https", username: "user", password: "" });
    expect(parseChainFields("http://invalid%ZZ:sample@example.com:1080")).toMatchObject({ username: "invalid%ZZ", password: "sample" });
  });
  it("keeps share links and node snippets untouched in link mode, and starts new drafts in fields mode", () => {
    for (const link of ["vless://sample@example.com:443", "ss://sample", '{"type":"socks","server":"example.com"}']) {
      const draft = chainDraft({ ...item, link });
      expect(draft.mode).toBe("link");
      expect(saveChainDraft(draft).link).toBe(link);
    }
    expect(chainDraft()).toMatchObject({ enabled: true, mode: "fields", fields: { scheme: "http", host: "", port: "" } });
  });
  it("encodes field credentials exactly and preserves password whitespace", () => {
    const draft = { ...chainDraft(item), name: " 住宅 ", fields: { scheme: "https", host: " example.com ", port: " 443 ", username: " u@ser ", password: " p:a@ss " } };
    expect(saveChainDraft(draft)).toMatchObject({ name: "住宅", link: "https://u%40ser:%20p%3Aa%40ss%20@example.com:443" });
    expect(chainConnection({ ...draft, fields: { ...draft.fields, username: "", password: "" } }).link).toBe("https://example.com:443");
  });
  it("validates save fields and self-reference, but tests can run without a name or saving", () => {
    const draft = chainDraft(item);
    expect(chainConnection({ ...draft, name: "" })).toEqual({ link: item.link, upstream: item.upstream });
    expect(() => saveChainDraft({ ...draft, name: " " })).toThrow("名称不能为空");
    expect(() => chainConnection({ ...draft, fields: { ...draft.fields, host: " " } })).toThrow("请填 IP 或域名");
    for (const port of ["0", "65536", "-1", "1080x", "1.5", ""]) expect(() => chainConnection({ ...draft, fields: { ...draft.fields, port } })).toThrow("端口要在 1 到 65535 之间");
    expect(() => chainConnection({ ...draft, upstream: "" })).toThrow("请选择上游");
    expect(() => chainConnection({ ...draft, mode: "link", link: " " })).toThrow("请填节点内容");
    expect(() => saveChainDraft({ ...draft, upstream: item.name })).toThrow("上游不能是它自己");
  });
  it("excludes builtins, disabled groups and the edited node from upstream candidates", () => {
    const group = { id: "auto", name: "所有-自动", type: "urltest", mode: "dynamic", icon: "", members: [], keywords: [] };
    const groups: GroupsResponse = { types: [], availableGroups: [], groups: [{ ...group, name: "直连", kind: "direct" }, group, { ...group, name: "停用", enabled: false }], availableNodes: [{ name: item.name, subscription: "链式代理" }, { name: "节点", subscription: "订阅" }] };
    expect(chainUpstreamOptions(groups, item)).toEqual({ ...groups, groups: [group], availableNodes: [groups.availableNodes[1]] });
    expect(chainUpstreamOptions(groups)!.availableNodes).toHaveLength(2);
    expect(chainUpstreamOptions(null)).toBeNull();
  });
  it("sends the panel URL, timeout and ordered IP providers, with the original Chinese query", () => {
    const request = chainLatencyRequest(item, { entries: { "config/geoip-info-api": "ipwho.is", "config/speedtest-url": "https://example.com/204", "config/speedtest-timeout": "7000", "config/language": "zh-CN" } }, 123);
    expect(request).toEqual({ link: item.link, upstream: item.upstream, testUrl: "https://example.com/204", timeoutMs: 7000, ipUrls: ["https://ipwho.is?t=123&lang=zh-CN", "https://api.ip.sb/geoip?t=123", "https://api.ipapi.is?t=123"] });
    expect(chainLatencyRequest(item, { entries: { "config/language": "en-US" } }, 123).ipUrls[1]).toBe("https://ipwho.is?t=123");
  });
  it("renders success, location and ASN from each returned IP provider, without another browser lookup", () => {
    const providers = [
      { url: "https://api.ip.sb/geoip", body: { ip: "203.0.113.1", country: "United States", country_code: "US", city: "Example City", asn: 64500, organization: "Example Org" } },
      { url: "https://ipwho.is", body: { ip: "203.0.113.1", country: "United States", country_code: "US", city: "Example City", connection: { asn: 64500, org: "Example Org" } } },
      { url: "https://api.ipapi.is", body: { ip: "203.0.113.1", location: { country: "United States", country_code: "US", city: "Example City" }, asn: { asn: 64500, org: "Example Org" } } },
      { url: "https://api.ipapi.is", body: { ip: "203.0.113.1", country: "United States", city: "Example City", asn: "AS64500 Example Org" } },
    ];
    for (const provider of providers) {
      const result = chainTestResult({ ok: true, ms: 123, via: "节点", ip: { ok: true, url: provider.url, body: JSON.stringify(provider.body) } });
      expect(result).toMatchObject({ ok: true, ms: 123, via: "节点", ip: { ip: "203.0.113.1", asn: 64500, organization: "Example Org", city: "Example City" } });
      expect(chainLocation(result.ip!)).toContain("Example City");
    }
    expect(chainLocation({ ip: "", asn: null, country: "美国", countryCode: "US", organization: "", region: "美国", city: "美国" })).toBe("美国");
    expect(chainIpDetailsUrl("2001:db8::1", "ipwho.is")).toBe("https://ipwho.is/2001%3Adb8%3A%3A1");
  });
  it("keeps latency errors separate from IP lookup and malformed-IP errors", () => {
    expect(chainTestResult({ ok: false, reason: "tls", error: "handshake", ip: { ok: false, error: "IP timeout" } })).toMatchObject({ ok: false, error: "TLS 握手失败:handshake", ipError: "IP timeout" });
    expect(chainTestResult({ ok: false, reason: "refused", error: "refused" }).error).toBe("端口拒绝连接");
    const result = chainTestResult({ ok: true, ms: 123, ip: { ok: true, body: "not JSON" } });
    expect(result.ok).toBe(true);
    expect(result.ipError).toBeTruthy();
    expect(result.ip).toBeUndefined();
  });
});
