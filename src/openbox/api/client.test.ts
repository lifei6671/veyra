import { afterEach, describe, expect, it, vi } from "vitest";
import { api, parseResponseBody } from "./client";

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
