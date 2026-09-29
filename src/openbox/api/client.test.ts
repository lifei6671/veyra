import { afterEach, describe, expect, it, vi } from "vitest";
import { api, normalizeGeoIpInfo, parseResponseBody } from "./client";

afterEach(() => vi.unstubAllGlobals());

describe("parseResponseBody", () => {
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
