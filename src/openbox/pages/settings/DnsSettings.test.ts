import { afterEach, describe, expect, it, vi } from "vitest";
import { api } from "../../api/client";
import type { DnsRewriteRule, OpenBoxProfile } from "../../api/types";
import { defaultUpstreams, isDnsAddress, normalizeDomain, parseAddresses, parseAllowDomains, restoreRewrites, rewriteNeedsRestart, rewriteValidation, upstreamPatch, upstreamRows, upstreamValidation } from "./DnsSettings.helpers";

const dns: OpenBoxProfile["dns"] = { split: true, mode: "hijack", fakeIpForProxy: true, direct: "223.5.5.5", directProtocol: "udp", directPort: 53, directExtras: [{ server: "8.8.8.8", protocol: "tcp", port: 5353 }], proxy: "1.1.1.1", proxyProtocol: "udp", proxyPort: 53, proxyExtras: [] };
const rule: DnsRewriteRule = { id: "custom", enabled: false, source: "*.example.com", domain: "target.example.com", addresses: [], note: "保留备注" };
afterEach(() => vi.unstubAllGlobals());
describe("DNS settings user contracts", () => {
  it("round-trips the primary and backup upstreams without altering routing or other DNS fields", () => {
    const rows = upstreamRows(dns);
    expect(rows.map(row => row.protocol)).toEqual(["udp", "tcp", "udp"]);
    expect(upstreamPatch(rows)).toEqual({ direct: "223.5.5.5", directProtocol: "udp", directPort: 53, directExtras: dns.directExtras, proxy: "1.1.1.1", proxyProtocol: "udp", proxyPort: 53, proxyExtras: [] });
    expect(upstreamPatch(rows.filter(row => row.server !== "223.5.5.5"))).toMatchObject({ direct: "8.8.8.8", directProtocol: "tcp", directPort: 5353, directExtras: [] });
    expect(upstreamPatch(defaultUpstreams)).toMatchObject({ direct: "223.5.5.5", proxy: "1.1.1.1", proxyProtocol: "tcp" });
  });
  it("rejects hostnames, unspecified addresses, invalid ports and duplicate upstream addresses", () => {
    for (const value of ["dns.example.com", "999.0.0.1", "0.0.0.0", "::", "1.2.3"]) expect(isDnsAddress(value)).toBe(false);
    expect(isDnsAddress("2001:4860:4860::8888")).toBe(true);
    const rows = upstreamRows(dns);
    expect(upstreamValidation({ ...rows[0], port: 0 }, rows, rows[0])).toContain("1~65535");
    expect(upstreamValidation({ ...rows[0], server: "8.8.8.8" }, rows, rows[0])).toContain("已经用过");
    expect(upstreamValidation({ ...rows[0], server: "9.9.9.9" }, rows, rows[0])).toBe("");
  });
  it("caps each side at four and allows changing the side of its only upstream without clearing existing settings", () => {
    const rows = Array.from({ length: 4 }, (_, index) => ({ ...defaultUpstreams[0], index, server: `1.1.1.${index + 1}` }));
    expect(upstreamValidation({ ...rows[0], server: "8.8.8.8" }, rows)).toContain("最多 4");
    const moved = { ...defaultUpstreams[1], side: "direct" as const, server: "9.9.9.9" };
    expect(upstreamValidation(moved, defaultUpstreams, defaultUpstreams[1])).toBe("");
    expect(upstreamPatch([defaultUpstreams[0], moved])).toEqual({ direct: "223.5.5.5", directProtocol: "udp", directPort: 53, directExtras: [{ server: "9.9.9.9", protocol: "tcp", port: 53 }] });
    expect({ ...dns, ...upstreamPatch([defaultUpstreams[0], moved]) }.proxy).toBe(dns.proxy);
  });
  it("normalizes rewrite domains, validates wildcard sources and prevents duplicate/self targets", () => {
    expect(normalizeDomain(" EXAMPLE.COM. ")).toBe("example.com");
    expect(rewriteValidation(rule, [rule], "domain")).toBe("");
    expect(rewriteValidation({ ...rule, source: "foo.*.com" }, [], "domain")).toContain("查询域名不合法");
    expect(rewriteValidation({ ...rule, id: "other" }, [rule], "domain")).toContain("已经有");
    expect(rewriteValidation({ ...rule, source: "target.example.com" }, [], "domain")).toContain("不能和");
    expect(rewriteValidation({ ...rule, domain: "*.target.com" }, [], "domain")).toContain("目标域名不合法");
    expect(parseAddresses("1.2.3.4, 1.2.3.4\n2001:DB8::1; 8.8.8.8")).toEqual(["1.2.3.4", "2001:db8::1", "8.8.8.8"]);
    expect(rewriteValidation({ ...rule, addresses: ["999.1.1.1"] }, [], "ip")).toContain("合法");
  });
  it("applies target/note edits live, but marks source, additions and enablement for restart", () => {
    const before = { enabled: true, rules: [rule] };
    expect(rewriteNeedsRestart(before, { ...before, rules: [{ ...rule, domain: "new.com", note: "新备注" }] })).toBe(false);
    expect(rewriteNeedsRestart(before, { ...before, rules: [{ ...rule, source: "new.com" }] })).toBe(true);
    expect(rewriteNeedsRestart(before, { ...before, rules: [{ ...rule, enabled: true }] })).toBe(true);
    expect(rewriteNeedsRestart(before, { ...before, enabled: false })).toBe(true);
    expect(rewriteNeedsRestart(before, { ...before, rules: [] })).toBe(true);
  });
  it("restores supplied defaults while preserving custom rules and avoiding duplicate source domains", () => {
    const defaults = [{ ...rule, id: "google", source: "google.cn", domain: "google.com" }];
    expect(restoreRewrites([rule], defaults)).toEqual([rule, defaults[0]]);
    expect(restoreRewrites([{ ...defaults[0], domain: "wrong.com" }, rule], defaults)).toEqual([defaults[0], rule]);
    const custom = { ...rule, source: "google.cn" };
    expect(restoreRewrites([custom], defaults)).toEqual([custom]);
  });
  it("saves one allow domain per line, dropping blank lines and duplicates without changing wildcards", () => {
    expect(parseAllowDomains("example.com\n \n *.example.com \nexample.com")).toEqual(["example.com", "*.example.com"]);
  });
  it("uses the original DNS test, partial profile save, apply, filter and cache endpoints", async () => {
    const fetchMock = vi.fn().mockImplementation((url: string) => Promise.resolve(new Response(JSON.stringify(url.endsWith("upstream-test") ? { ok: false, error: "unreachable", note: "线路不可用" } : url.endsWith("profile") ? { profile: { dns } } : {}))));
    vi.stubGlobal("fetch", fetchMock); vi.stubGlobal("localStorage", { getItem: () => null }); vi.stubGlobal("navigator", { language: "zh-CN" });
    await expect(api.testDnsUpstream({ side: "proxy", protocol: "tcp", port: 53, server: "1.1.1.1" })).resolves.toMatchObject({ ok: false, error: "unreachable" });
    await api.saveProfile({ dns: { fakeIpForProxy: false } });
    await api.saveDnsFilter({ enabled: true, lists: [], allowDomains: ["*.example.com"], autoUpdate: { enabled: true, days: 7, hour: 4 } });
    await api.applyDnsFilter(true, "anti-ad"); await api.flushDns();
    expect(fetchMock.mock.calls[0]).toMatchObject(["/api/openbox/dns/upstream-test", { method: "POST", body: JSON.stringify({ side: "proxy", protocol: "tcp", port: 53, server: "1.1.1.1" }) }]);
    expect(fetchMock.mock.calls[1]).toMatchObject(["/api/openbox/profile", { method: "PUT", body: '{"dns":{"fakeIpForProxy":false}}' }]);
    expect(fetchMock.mock.calls[2][0]).toBe("/api/openbox/dns-filter");
    expect(fetchMock.mock.calls[3]).toMatchObject(["/api/openbox/dns-filter/apply", { method: "POST", body: '{"update":true,"listId":"anti-ad"}' }]);
    expect(fetchMock.mock.calls[4]).toMatchObject(["/api/openbox/dns/flush-cache", { method: "POST" }]);
  });
  it("encodes preview/record search, filters and pagination, and consumes the defaults response", async () => {
    const fetchMock = vi.fn().mockImplementation(() => Promise.resolve(new Response(JSON.stringify({ dnsRewriteDefaults: [rule], rows: [], page: 2, pageSize: 20, total: 100 }))));
    vi.stubGlobal("fetch", fetchMock); vi.stubGlobal("localStorage", { getItem: () => null }); vi.stubGlobal("navigator", { language: "zh-CN" });
    await expect(api.dnsRecords("a & b", "blocked", 2, 20)).resolves.toMatchObject({ page: 2, pageSize: 20, total: 100 });
    await api.dnsPreview("https://example.com/list?a=b&c=d", "测试", "all", 1, 20);
    await expect(api.dnsRewriteDefaults()).resolves.toEqual([rule]);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/openbox/dns-filter/records?search=a+%26+b&result=blocked&page=2&pageSize=20");
    expect(fetchMock.mock.calls[1][0]).toBe("/api/openbox/dns-filter/preview?url=https%3A%2F%2Fexample.com%2Flist%3Fa%3Db%26c%3Dd&search=%E6%B5%8B%E8%AF%95&action=all&page=1&pageSize=20");
    expect(fetchMock.mock.calls[2][0]).toBe("/api/openbox/profile/defaults?region=cn");
  });
});
