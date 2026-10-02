import { afterEach, describe, expect, it, vi } from "vitest";
import { api, normalizeGeoIpInfo, parseResponseBody } from "./client";
import { buildAutoGroups, mergeAutoGroups } from "../pages/settings/GroupSettings.helpers";

afterEach(() => vi.unstubAllGlobals());

describe("parseResponseBody", () => {
  it("loads and saves panel settings with the original storage and background API payloads", async () => {
    const entries = { "config/global-radius": "16", "config/ipv6-test": "true", "config/test-sites": JSON.stringify([{ id: "google", icon: "brand:github", name: "GitHub", url: "https://github.com/" }]) };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ entries }), { headers: { "content-type": "text/plain" } }))
      .mockResolvedValueOnce(new Response("{}"))
      .mockResolvedValueOnce(new Response('{"image":"data:image/png;base64,fixture"}'))
      .mockResolvedValueOnce(new Response("{}"))
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(new Response("{}"));
    vi.stubGlobal("fetch", fetchMock);
    vi.stubGlobal("localStorage", { getItem: () => "zh-CN" });
    vi.stubGlobal("navigator", { language: "zh-CN" });

    await expect(api.storage()).resolves.toEqual({ entries });
    await api.patchStorage(entries);
    await expect(api.backgroundImage()).resolves.toBe("data:image/png;base64,fixture");
    await api.saveBackgroundImage("data:image/png;base64,fixture");
    await api.deleteBackgroundImage();
    await api.changePassword("old-test-password", "four");

    expect(fetchMock.mock.calls[0][0]).toBe("/api/storage");
    expect(fetchMock.mock.calls[1]).toMatchObject(["/api/storage", { method: "PATCH", body: JSON.stringify({ entries, removed: [] }) }]);
    expect(fetchMock.mock.calls[2][0]).toBe("/api/background-image");
    expect(fetchMock.mock.calls[3]).toMatchObject(["/api/background-image", { method: "PUT", body: JSON.stringify({ image: "data:image/png;base64,fixture" }) }]);
    expect(fetchMock.mock.calls[4]).toMatchObject(["/api/background-image", { method: "DELETE" }]);
    expect(fetchMock.mock.calls[5]).toMatchObject(["/api/auth/change-password", { method: "POST", body: JSON.stringify({ currentPassword: "old-test-password", newPassword: "four" }) }]);
  });

  it("parses JSON returned with the backend text/plain content type", () => {
    expect(parseResponseBody('{"proxies":{"DIRECT":{"name":"DIRECT"}}}', "text/plain")).toEqual({
      proxies: { DIRECT: { name: "DIRECT" } },
    });
  });

  it("keeps non-JSON text responses unchanged", () => {
    expect(parseResponseBody("service unavailable", "text/plain")).toBe("service unavailable");
  });

  it("normalizes the supported real IP information providers", () => {
    expect(normalizeGeoIpInfo("ip.sb", { ip: "198.41.192.67", asn: 13335, country_code: "US", organization: "Cloudflare" }, "fallback")).toEqual({
      ip: "198.41.192.67", asn: 13335, countryCode: "US", country: "", organization: "Cloudflare",
    });
    expect(normalizeGeoIpInfo("ipwho.is", { ip: "198.41.192.67", country: "United States", country_code: "US", connection: { asn: 13335, org: "Cloudflare, Inc." } }, "fallback")).toMatchObject({
      asn: 13335, countryCode: "US", country: "United States", organization: "Cloudflare, Inc.",
    });
    expect(normalizeGeoIpInfo("ipapi.is", { ip: "198.41.192.67", location: { country: "United States", country_code: "US" }, asn: { asn: 13335, org: "Cloudflare, Inc." } }, "fallback")).toMatchObject({
      asn: 13335, countryCode: "US", country: "United States", organization: "Cloudflare, Inc.",
    });
  });

  it("queries the configured IP information API instead of returning mock data", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response('{"ip":"198.41.192.67","asn":13335,"country_code":"US","organization":"Cloudflare"}', {
      status: 200,
      headers: { "content-type": "application/json" },
    }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(api.geoIp("198.41.192.67", "ip.sb")).resolves.toMatchObject({ ip: "198.41.192.67", asn: 13335 });
    expect(fetchMock.mock.calls[0][0]).toMatch(/^https:\/\/api\.ip\.sb\/geoip\/198\.41\.192\.67\?t=\d+$/);
  });

  it("wires proxy selection and single-node delay checks to controller endpoints", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(new Response('{"delay":188}', { status: 200, headers: { "content-type": "text/plain" } }));
    vi.stubGlobal("fetch", fetchMock);
    vi.stubGlobal("localStorage", { getItem: () => null });
    vi.stubGlobal("navigator", { language: "zh-CN" });

    await api.selectProxy("Speed", "订阅 | 美国-03");
    await expect(api.testProxy("订阅 | 美国-03", "http://example.com/204", 5000)).resolves.toEqual({ delay: 188 });

    expect(fetchMock.mock.calls[0][0]).toBe("/api/controller/proxies/Speed");
    expect(fetchMock.mock.calls[0][1]).toMatchObject({ method: "PUT", body: JSON.stringify({ name: "订阅 | 美国-03" }) });
    expect(fetchMock.mock.calls[1][0]).toBe("/api/controller/proxies/%E8%AE%A2%E9%98%85%20%7C%20%E7%BE%8E%E5%9B%BD-03/delay?url=http%3A%2F%2Fexample.com%2F204&timeout=5000");
  });

  it("uses the original outbound-group load, save, and defaults endpoints", async () => {
    const group = {
      id: "all-auto", name: "所有-自动", type: "urltest", mode: "dynamic", enabled: true,
      icon: "globe:earth-asia", iconScale: 0, keywords: [], members: [], interval: "300s", tolerance: 100,
    };
    const response = JSON.stringify({ groups: [group], types: ["urltest"], availableNodes: [], availableGroups: [] });
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(response, { status: 200, headers: { "content-type": "application/json" } }))
      .mockResolvedValueOnce(new Response(response, { status: 200, headers: { "content-type": "application/json" } }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ groups: [group] }), { status: 200, headers: { "content-type": "application/json" } }));
    vi.stubGlobal("fetch", fetchMock);
    vi.stubGlobal("localStorage", { getItem: () => null });
    vi.stubGlobal("navigator", { language: "zh-CN" });

    await expect(api.groups()).resolves.toMatchObject({ groups: [group] });
    await expect(api.saveGroups([group])).resolves.toMatchObject({ groups: [group] });
    await expect(api.defaultGroups()).resolves.toEqual([group]);

    expect(fetchMock.mock.calls[0][0]).toBe("/api/openbox/groups");
    expect(fetchMock.mock.calls[1]).toMatchObject(["/api/openbox/groups", { method: "PUT", body: JSON.stringify({ groups: [group] }) }]);
    expect(fetchMock.mock.calls[2][0]).toBe("/api/openbox/defaults/groups");
  });

  it("sends auto groups in country order and uses the saved groups and compiler warnings returned by the backend", async () => {
    const existing = { id: "manual", name: "美国-手动", type: "selector", mode: "dynamic", icon: "US", keywords: ["us"], members: [] };
    const auto = { id: "auto-us-urltest-123-0", name: "美国-自动", type: "urltest", mode: "dynamic", icon: "US", keywords: ["us", "united", "美国", "美國", "united states", "america", "洛杉矶", "洛杉磯", "硅谷", "圣何塞", "西雅图", "纽约"], members: [], interval: "300s", tolerance: 100 };
    const saved = { groups: [{ ...auto, enabled: true, iconScale: 0 }, existing], dropped: [{ name: "空组", reason: "empty" }], dangling: [{ name: "旧组", members: ["失效节点"] }] };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ groups: [existing], types: ["urltest", "selector"], availableNodes: [{ name: "🇺🇸 01", subscription: "A" }], availableGroups: [] })))
      .mockResolvedValueOnce(new Response(JSON.stringify(saved)));
    vi.stubGlobal("fetch", fetchMock);
    vi.stubGlobal("localStorage", { getItem: () => null });
    vi.stubGlobal("navigator", { language: "zh-CN" });
    const loaded = await api.groups();
    const generated = buildAutoGroups(loaded.groups, ["US"], ["urltest", "selector"], 123);
    expect(generated).toEqual([auto]);
    await expect(api.saveGroups(mergeAutoGroups(loaded.groups, generated))).resolves.toEqual(saved);
    expect(fetchMock.mock.calls[1]).toMatchObject(["/api/openbox/groups", { method: "PUT", body: JSON.stringify({ groups: [auto, existing] }) }]);
  });

  it("uses the original route lookup and diagnostic API contracts", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response("{}", { status: 200, headers: { "content-type": "application/json" } }));
    vi.stubGlobal("fetch", fetchMock);
    vi.stubGlobal("localStorage", { getItem: () => null });
    vi.stubGlobal("navigator", { language: "zh-CN" });

    await api.penetration("www.baidu.com");
    await api.terminalTestCapability();
    await api.terminalTest("www.baidu.com", 443, "TLS");
    await api.routeTest("www.baidu.com", 443);

    expect(fetchMock.mock.calls[0]).toMatchObject(["/api/openbox/penetration", { method: "POST", body: JSON.stringify({ target: "www.baidu.com" }) }]);
    expect(fetchMock.mock.calls[1][0]).toBe("/api/openbox/terminal-test/capability");
    expect(fetchMock.mock.calls[2]).toMatchObject(["/api/openbox/terminal-test", { method: "POST", body: JSON.stringify({ target: "www.baidu.com", port: 443, method: "TLS" }) }]);
    expect(fetchMock.mock.calls[3]).toMatchObject(["/api/openbox/route-test", { method: "POST", body: JSON.stringify({ target: "www.baidu.com", port: 443 }) }]);
  });

  it("keeps month, hour and drill filters on the real traffic endpoints", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response("{}", { status: 200, headers: { "content-type": "application/json" } }));
    vi.stubGlobal("fetch", fetchMock);
    vi.stubGlobal("localStorage", { getItem: () => null });
    vi.stubGlobal("navigator", { language: "zh-CN" });

    await api.trafficMonth("2026-09", false);
    await api.trafficDay("2026-09-28", 15, false);
    await api.trafficDrill("2026-09-28", "client", "192.168.1.10", "host", 15, false);

    expect(fetchMock.mock.calls[0][0]).toBe("/api/openbox/traffic/month?month=2026-09&direct=0");
    expect(fetchMock.mock.calls[1][0]).toBe("/api/openbox/traffic/day?day=2026-09-28&limit=500&hour=15&direct=0");
    expect(fetchMock.mock.calls[2][0]).toBe("/api/openbox/traffic/drill?day=2026-09-28&kind=client&key=192.168.1.10&by=host&limit=200&hour=15&direct=0");
  });
});

describe("target routing API contracts", () => {
  it("tests unsaved chain content and saves only chainProxies, returning backend node metadata", async () => {
    const request = { link: "socks5://example.com:1080", upstream: "所有-自动", testUrl: "https://example.com/204", timeoutMs: 5000, ipUrls: ["https://api.ip.sb/geoip"] };
    const latency = { ok: true, ms: 123, via: "节点", ip: { ok: false, error: "timeout" } };
    const chainProxies = [{ id: "chain", name: "住宅", enabled: true, link: request.link, upstream: request.upstream }];
    const profile = { ipv6: true, chainProxies: [{ ...chainProxies[0], node: { type: "socks", server: "example.com", port: 1080 } }] };
    const fetchMock = vi.fn().mockResolvedValueOnce(new Response(JSON.stringify(latency))).mockResolvedValueOnce(new Response(JSON.stringify({ profile })));
    vi.stubGlobal("fetch", fetchMock); vi.stubGlobal("localStorage", { getItem: () => null }); vi.stubGlobal("navigator", { language: "zh-CN" });
    await expect(api.testChainProxy(request)).resolves.toEqual(latency);
    await expect(api.saveProfile({ chainProxies })).resolves.toEqual(profile);
    expect(fetchMock.mock.calls[0]).toMatchObject(["/api/openbox/chain-proxies/latency", { method: "POST", body: JSON.stringify(request) }]);
    expect(fetchMock.mock.calls[1]).toMatchObject(["/api/openbox/profile", { method: "PUT", body: JSON.stringify({ chainProxies }) }]);
  });
  it("reads known terminals and saves only clientRoutes, consuming the complete backend profile", async () => {
    const clients = [{ ip: "192.168.1.10", name: "电视", mac: "aa:bb:cc:dd:ee:ff" }];
    const clientRoutes = [{ id: "tv", name: "电视", match: "mac" as const, sources: [], macs: [clients[0].mac], outbound: "直连" }];
    const profile = { clientRoutes, ipv6: true, serverNormalized: true };
    const fetchMock = vi.fn().mockResolvedValueOnce(new Response(JSON.stringify({ clients }))).mockResolvedValueOnce(new Response(JSON.stringify({ profile })));
    vi.stubGlobal("fetch", fetchMock); vi.stubGlobal("localStorage", { getItem: () => null }); vi.stubGlobal("navigator", { language: "zh-CN" });
    await expect(api.clients()).resolves.toEqual({ clients });
    await expect(api.saveProfile({ clientRoutes })).resolves.toEqual(profile);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/openbox/clients");
    expect(fetchMock.mock.calls[1]).toMatchObject(["/api/openbox/profile", { method: "PUT", body: JSON.stringify({ clientRoutes }) }]);
  });
  it("saves only the changed routing portion and consumes the returned profile and installation defaults", async () => {
    const routing = { policies: [{ id: "ai", name: "AI", rulesets: ["geosite-category-ai-!cn"] }], fallbackDefault: "直连", fallbackName: "其他", fallbackIcon: "globe:generic" };
    const profile = { routing, ipv6: true, serverNormalized: true };
    const fetchMock = vi.fn().mockResolvedValueOnce(new Response(JSON.stringify({ profile }))).mockResolvedValueOnce(new Response(JSON.stringify({ routing })));
    vi.stubGlobal("fetch", fetchMock); vi.stubGlobal("localStorage", { getItem: () => null }); vi.stubGlobal("navigator", { language: "zh-CN" });
    await expect(api.saveProfile({ routing: { policies: routing.policies } })).resolves.toEqual(profile);
    await expect(api.defaultRouting()).resolves.toEqual(routing);
    expect(fetchMock.mock.calls[0]).toMatchObject(["/api/openbox/profile", { method: "PUT", body: JSON.stringify({ routing: { policies: routing.policies } }) }]);
    expect(fetchMock.mock.calls[1][0]).toBe("/api/openbox/defaults/routing");
  });
  it("uses the source endpoints and pagination for geosite, remote rule previews, imports and refreshes", async () => {
    const result = { entries: [{ type: "domain", value: "example.com" }], total: 182, matched: 1 };
    const fetchMock = vi.fn().mockImplementation(async () => new Response(JSON.stringify(result)));
    vi.stubGlobal("fetch", fetchMock); vi.stubGlobal("localStorage", { getItem: () => null }); vi.stubGlobal("navigator", { language: "zh-CN" });
    await expect(api.ruleSetEntries({ tag: "geosite-category-ai-!cn" }, "example", 50)).resolves.toEqual(result);
    await api.ruleSetEntries({ url: "https://example.com/list?a=1&b=2" }, "", 0, 1);
    await api.importRuleSet("https://example.com/list?a=1&b=2");
    await api.refreshRuleSet("https://example.com/list");
    expect(fetchMock.mock.calls[0][0]).toBe("/api/openbox/rulesets/entries?tag=geosite-category-ai-%21cn&offset=50&limit=50&q=example");
    expect(fetchMock.mock.calls[1][0]).toBe("/api/openbox/rulesets/preview?url=https%3A%2F%2Fexample.com%2Flist%3Fa%3D1%26b%3D2&offset=0&limit=1");
    expect(fetchMock.mock.calls[2][0]).toBe("/api/openbox/rulesets/import?url=https%3A%2F%2Fexample.com%2Flist%3Fa%3D1%26b%3D2");
    expect(fetchMock.mock.calls[3]).toMatchObject(["/api/openbox/rulesets/refresh", { method: "POST", body: JSON.stringify({ url: "https://example.com/list" }) }]);
  });
});
