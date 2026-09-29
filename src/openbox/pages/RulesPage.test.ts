import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { ControllerProxy, ControllerRule, PenetrationResult, RouteDiagnosticResult } from "../api/types";
import { buildActualStages, buildLoadingStages, buildPredictedStages, buildUnavailableStages, canExpandRule, normalizeRuleQuery, proxyDetailLabel, RouteComparison, RuleProxyDetails, ruleProxyGroup } from "./RulesPage";

const rule = (proxy: string): ControllerRule => ({ type: "default", payload: "rule_set=test", proxy });
const proxy = (name: string, all?: string[]): ControllerProxy => ({ name, type: all ? "Selector" : "Direct", all });

describe("RulesPage helpers", () => {
  it("extracts a route target without treating plain actions as groups", () => {
    expect(ruleProxyGroup("route(Speed)")).toBe("Speed");
    expect(ruleProxyGroup("reject")).toBe("");
    expect(ruleProxyGroup("sniff")).toBe("");
  });

  it("only expands rules backed by a proxy group with candidates", () => {
    const proxies = { Speed: proxy("Speed", ["直连", "所有-自动"]), "直连": proxy("直连") };
    expect(canExpandRule(rule("route(Speed)"), proxies)).toBe(true);
    expect(canExpandRule(rule("route(直连)"), proxies)).toBe(false);
    expect(canExpandRule(rule("reject"), proxies)).toBe(false);
  });

  it("formats HTTP URLs like the original search formatter", () => {
    expect(normalizeRuleQuery("https://www.google.com/path?q=1")).toBe("www.google.com");
    expect(normalizeRuleQuery("api.openai.com:443")).toBe("api.openai.com:443");
  });

  it("only advertises UDP when the proxy reports support", () => {
    expect(proxyDetailLabel({ ...proxy("auto"), type: "URLTest", udp: true })).toBe("urltest / udp");
    expect(proxyDetailLabel({ ...proxy("manual"), type: "Selector", udp: false })).toBe("selector");
    expect(proxyDetailLabel({ ...proxy("direct"), type: "Direct" })).toBe("direct");
  });

  it("maps the penetration response into the five original route stages", () => {
    const result = {
      matched: { index: 24, rule: { rule_set: ["geosite-google"], outbound: "Google" }, entries: [{ type: "domain_suffix", value: "google.com", source: "geosite-google" }] },
      chain: ["Google", "所有-自动", "订阅 | 美国-04"],
      finalOutbound: "Google",
      owner: { kind: "policy", name: "Google" },
      resolved: { addresses: ["198.19.0.52", "198.19.0.53"], fakeIp: true },
      dns: { viaProxy: true, server: { type: "tcp", tag: "dns-proxy", server: "1.1.1.1", detour: "国外" } },
      firstLayer: { nativeBypass: { enabled: true, sets: ["geoip-cn", "geoip-private"] }, entryMode: { mode: "blacklist", reason: "兜底「其他」走代理" } },
    } satisfies PenetrationResult;
    const stages = buildPredictedStages("www.google.com", result);
    expect(stages.map(stage => stage.number)).toEqual([5, 4, 3, 2, 1]);
    expect(stages[0]).toMatchObject({ badge: "代理", badgeTone: "proxy", chain: ["所有-自动", "订阅 | 美国-04"] });
    expect(stages[1]).toMatchObject({ badge: "第 25 条", headline: "站点集", meta: "Google", resultBadge: "域名后缀", value: "google.com", source: "geosite-google" });
    expect(stages[1].details).toEqual([{ text: "rule_set=geosite-google", mono: true }]);
    expect(stages[2].details?.map(detail => detail.text)).toEqual([
      "直连 IP 集合 geoip-cn, geoip-private 在入口旁路（nft）,不进内核",
      "按目标 IP 判:终端连的地址在旁路集合里才旁路,面板预知不了终端拿到的地址。",
      "入口没有默认放行:兜底「其他」走代理",
    ]);
    expect(stages[3]).toMatchObject({ headline: "代理 DNS", meta: "TCP 1.1.1.1 返回 FakeIP 占位地址" });
    expect(stages[3].details?.map(detail => detail.text)).toEqual([
      "解析器 dns-proxy",
      "内核 DNS 返回的是 FakeIP 占位地址:连接进内核后按域名找回目标,按 IP 判的规则不参与",
      "解析经由",
      "这里是内核 DNS 规则的推算;终端的查询是否进入内核,由上方的入口策略决定。",
    ]);
    expect(stages[3].details?.[2]).toEqual({ text: "解析经由", chain: ["国外"] });
  });

  it("renders a direct penetration result from the returned leaf outbound", () => {
    const result = {
      matched: { index: 36, rule: { rule_set: ["geosite-cn"], outbound: "国内" }, entries: [{ type: "domain_suffix", value: "baidu.com", source: "geosite-cn" }] },
      chain: ["国内", "直连"],
      finalOutbound: "国内",
      owner: { kind: "policy", name: "国内" },
      resolved: { addresses: ["110.242.69.21", "110.242.70.57"], fakeIp: false },
      dns: { viaProxy: false, server: { type: "udp", tag: "dns-direct", server: "114.114.114.114" } },
      firstLayer: { nativeBypass: { enabled: true, sets: ["geoip-cn", "geoip-private"] }, entryMode: { mode: "blacklist", reason: "兜底「其他」走代理" } },
    } satisfies PenetrationResult;

    const stages = buildPredictedStages("www.baidu.com", result);
    expect(stages[0]).toMatchObject({ badge: "直连", badgeTone: "success", chain: ["直连"] });
    expect(stages[3]).toMatchObject({ headline: "直连 DNS", meta: "UDP 114.114.114.114 解析到 2 个地址" });
  });

  it("does not show a stale penetration result after the current lookup fails", () => {
    const stale = {
      chain: ["国外", "所有-自动", "订阅 | 美国-04"],
      finalOutbound: "国外",
      owner: { kind: "policy", name: "国外" },
    } satisfies PenetrationResult;

    const markup = renderToStaticMarkup(createElement(RouteComparison, {
      target: "telegram.org",
      penetration: stale,
      penetrationLoading: false,
      penetrationError: "规则路由推算失败",
      mode: "kernel",
      method: "GET",
      capability: null,
      actual: null,
      actualLoading: false,
      actualError: "",
      onModeChange: () => undefined,
      onMethodChange: () => undefined,
      onRetry: () => undefined,
    }));

    expect(markup).toContain("推算失败");
    expect(markup).toContain("规则路由推算失败");
    expect(markup).not.toContain("订阅 | 美国-04");
  });

  it("renders the original five-stage busy state for a kernel diagnostic", () => {
    const stages = buildLoadingStages("telegram.org", "kernel");
    expect(stages.slice(0, 4).map(stage => stage.headline)).toEqual(["等待查询...", "等待查询...", "等待查询...", "等待查询..."]);
    expect(stages[4]).toMatchObject({ badge: "域名", headline: "面板自身测试", meta: "telegram.org" });
    expect(stages[4].details).toHaveLength(2);

    const markup = renderToStaticMarkup(createElement(RouteComparison, {
      target: "telegram.org",
      penetration: null,
      penetrationLoading: false,
      penetrationError: "",
      mode: "kernel",
      method: "GET",
      capability: null,
      actual: null,
      actualLoading: true,
      actualError: "",
      onModeChange: () => undefined,
      onMethodChange: () => undefined,
      onRetry: () => undefined,
    }));
    expect(markup).toContain('class="route-status muted loading"');
    expect(markup.match(/等待查询\.\.\./g)).toHaveLength(4);
    expect(markup).toContain("面板自身测试");
    expect(markup).toContain("telegram.org");
  });

  it("maps the kernel diagnostic response into observed DNS and exit results", () => {
    const result = {
      dns: { viaProxy: true, server: { type: "tcp", server: "1.1.1.1" } },
      resolve: { ok: true, answers: ["192.133.77.191"], ms: 13 },
      exit: { ok: false, ms: 5490, error: "TLS failed", chains: ["AI", "所有-自动"], owner: { kind: "policy", name: "AI" } },
    } satisfies RouteDiagnosticResult;
    const stages = buildActualStages("api.openai.com", "kernel", result);
    expect(stages[0]).toMatchObject({ badge: "5490 ms", headline: "所有-自动", tone: "warning" });
    expect(stages[2].headline).toBe("面板回环入站");
    expect(stages[3]).toMatchObject({ badge: "13 ms", headline: "代理 DNS" });
  });

  it("maps every expanded kernel diagnostic detail from the route-test response", () => {
    const result = {
      target: "www.baidu.com",
      dns: { ruleIndex: 40, viaProxy: false, server: { type: "udp", tag: "dns-direct", server: "114.114.114.114" } },
      resolve: { ok: true, status: 200, answers: ["110.242.69.21", "110.242.70.57"], ms: 15, ttl: 37 },
      exit: {
        url: "https://www.baidu.com/",
        connectTo: "110.242.69.21",
        ok: true,
        status: 200,
        ms: 67,
        chains: ["国内", "直连"],
        rule: "rule_set=geosite-cn => route(国内)",
        destinationIP: "110.242.69.21",
        viaProxy: false,
        owner: { kind: "policy", name: "国内" },
      },
    } satisfies RouteDiagnosticResult;
    const stages = buildActualStages("www.baidu.com", "kernel", result, false);
    expect(stages[0].details).toEqual([{ text: "目标 110.242.69.21", valueMono: true }]);
    expect(stages[1].details).toEqual([
      { text: "rule_set=geosite-cn => route(国内)", mono: true },
      { text: "本列是面板自己的测试连接,来源和入站路径与终端不同,不能用它补齐左侧缺失的终端条件。" },
    ]);
    expect(stages[2].details).toEqual([
      { text: "这次测试经过 Open-Box 内核（连接表已记录）" },
      { text: "这次访问是面板以域名进内核的回环入站发起的,和终端按域名访问一样:只按域名条件判,前置自定义分流和站点集里的 IP 段、GeoIP 都不参与,域名都没命中就走兜底。" },
    ]);
    expect(stages[3].details).toEqual([
      { text: "IPv6 未查询（档案未开启 IPv6）" },
      { label: "解析器", text: "dns-direct", valueMono: true },
      { text: "114.114.114.114 应答" },
      { label: "A", text: "A", values: ["110.242.69.21", "110.242.70.57"] },
    ]);
    expect(stages[4].details).toEqual([
      { text: "由面板经内核的回环入站发起,没有模拟任何终端的来源;结果不代表某台终端的实际访问。" },
      { text: "内核诊断走面板的回环入站,不经过 LAN 入口,只能看到内核内部的分流,看不到入口旁路。" },
    ]);
    const ipv6Stages = buildActualStages("www.baidu.com", "kernel", result, true);
    expect(ipv6Stages[3].details).not.toContainEqual({ text: "IPv6 未查询（档案未开启 IPv6）" });
    expect(ipv6Stages[3].details).toEqual([
      { label: "解析器", text: "dns-direct", valueMono: true },
      { text: "114.114.114.114 应答" },
      { label: "A", text: "A", values: ["110.242.69.21", "110.242.70.57"] },
    ]);
  });

  it("maps proxied cached OpenAI diagnostics into warning notes and observed chains", () => {
    const result = {
      target: "api.openai.com",
      dns: {
        ruleIndex: 9,
        server: { type: "tcp", tag: "dns-proxy", server: "1.1.1.1", detour: "国外" },
        viaProxy: true,
        fakeIpRule: 8,
        runtimeChain: ["国外", "所有-自动", "订阅 | 美国-04"],
        runtimeLeaf: "订阅 | 美国-04",
      },
      resolve: { ok: true, status: 200, answers: ["75.126.33.156"], ms: 13, ttl: 37, cached: true },
      exit: {
        url: "https://api.openai.com/",
        connectTo: "75.126.33.156",
        ok: false,
        ms: 5556,
        error: "Client network socket disconnected before secure TLS connection was established",
        chains: ["AI", "所有-自动", "订阅 | 美国-04"],
        rule: "rule_set=geosite-category-ai-!cn => route(AI)",
        destinationIP: "75.126.33.156",
        viaProxy: true,
        owner: { kind: "policy", name: "AI" },
      },
    } satisfies RouteDiagnosticResult;
    const stages = buildActualStages("api.openai.com", "kernel", result, false);

    expect(stages[0]).toMatchObject({
      badge: "5556 ms",
      badgeTone: "error",
      chain: ["所有-自动", "订阅 | 美国-04"],
      resultBadge: "访问失败",
      resultBadgeTone: "error",
      tone: "warning",
    });
    expect(stages[0].details).toEqual([
      { text: "访问失败:Client network socket disconnected before secure TLS connection was established", tone: "warning" },
      { text: "目标 75.126.33.156", valueMono: true },
      { text: "节点拿到的是 IP 75.126.33.156,按它直接连。这个地址来自内核缓存,是之前那次解析的答案,未必是当前线路就近的 CDN", tone: "warning" },
    ]);
    expect(stages[3].details).toEqual([
      { text: "IPv6 未查询（档案未开启 IPv6）" },
      { label: "解析器", text: "dns-proxy", valueMono: true },
      { text: "解析经由", chain: ["国外", "所有-自动", "订阅 | 美国-04"] },
      { text: "命中内核缓存（还剩 37 秒）。这份答案是之前解析的,这次没有重新经这条线路问", tone: "warning" },
      { label: "A", text: "A", values: ["75.126.33.156"] },
    ]);

    const markup = renderToStaticMarkup(createElement(RouteComparison, {
      target: "api.openai.com",
      penetration: null,
      penetrationLoading: false,
      penetrationError: "",
      mode: "kernel",
      method: "GET",
      capability: null,
      actual: result,
      actualLoading: false,
      actualError: "",
      onModeChange: () => undefined,
      onMethodChange: () => undefined,
      onRetry: () => undefined,
    }));
    expect(markup).toContain('class="route-status error"');
    expect(markup).toContain('class="route-stage-chain"');
    expect(markup).toContain('class="error">访问失败</em>');
  });

  it("does not present a partial diagnostic response as a successful exit", () => {
    const stages = buildActualStages("www.baidu.com", "kernel", { resolve: { ok: false, error: "DNS timeout" } });
    expect(stages[0]).toMatchObject({ headline: "—", meta: "未返回访问结果", tone: "warning" });
    expect(stages[1]).toMatchObject({ headline: "—", tone: "muted" });
    expect(stages[2]).toMatchObject({ headline: "—", tone: "muted" });
    expect(stages[3]).toMatchObject({ headline: "DNS 解析失败", tone: "warning", details: [{ text: "DNS timeout", mono: false }] });
  });

  it("renders the unavailable terminal state with a kernel diagnostic fallback", () => {
    const markup = renderToStaticMarkup(createElement(RouteComparison, {
      target: "www.baidu.com",
      penetration: null,
      penetrationLoading: false,
      penetrationError: "",
      mode: "terminal",
      method: "GET",
      capability: { ok: false, missing: ["lan", "dhcp"], lan: null },
      actual: null,
      actualLoading: false,
      actualError: "",
      onModeChange: () => undefined,
      onMethodChange: () => undefined,
      onRetry: () => undefined,
    }));
    expect(markup).toContain("真实路由");
    expect(markup).toContain("无法模拟");
    expect(markup).toContain("此设备不能模拟 LAN 终端");
    expect(markup).toContain("改用内核诊断");
    expect(markup).toContain('class="route-mode-toggle"');
    expect(markup).toContain('<select class="route-method-select" aria-label="探测方式"');
    expect(markup).toContain('<span>重新测试</span>');
    expect(markup).not.toContain(">0</div>");
  });

  it("maps terminal capability details without inventing a target field", () => {
    const stages = buildUnavailableStages("www.baidu.com", { ok: false, missing: ["lan", "dhcp"], lan: null });
    expect(stages[4]).toMatchObject({ headline: "此设备不能模拟 LAN 终端" });
    expect(stages[4]).not.toHaveProperty("meta");
    expect(stages[4].details).toEqual([
      { text: "缺少:找不到 LAN 网桥或它的 IPv4 地址、没有 udhcpc", tone: "warning" },
      { text: "不会自动退回内核回环测试:那条路径不经过 LAN 入口,看不到入口旁路。" },
    ]);
  });

  it("renders observed route icons and the HTTP result badge", () => {
    const actual = {
      resolve: { ok: true, answers: ["110.242.68.66"], ms: 14 },
      exit: { ok: true, status: 200, ms: 50, chains: ["国内", "直连"], owner: { kind: "policy", name: "国内" } },
    } satisfies RouteDiagnosticResult;
    const markup = renderToStaticMarkup(createElement(RouteComparison, {
      target: "www.baidu.com",
      penetration: null,
      penetrationLoading: false,
      penetrationError: "",
      mode: "kernel",
      method: "GET",
      capability: null,
      actual,
      actualLoading: false,
      actualError: "",
      onModeChange: () => undefined,
      onMethodChange: () => undefined,
      onRetry: () => undefined,
    }));
    expect(markup.match(/<img src="data:image\/svg\+xml/g)).toHaveLength(2);
    expect(markup).toContain("HTTP 200");
  });

  it("reuses the global proxy card with split latency actions and policy penetration", () => {
    const node = { name: "订阅 | 美国-04", type: "VLESS", udp: true } satisfies ControllerProxy;
    const nested = { name: "所有-手动", type: "Selector", now: node.name, all: [node.name] } satisfies ControllerProxy;
    const group = { name: "Speed", type: "Selector", now: nested.name, all: ["直连", nested.name] } satisfies ControllerProxy;
    const direct = { name: "直连", type: "Direct", udp: true } satisfies ControllerProxy;
    const markup = renderToStaticMarkup(createElement(RuleProxyDetails, {
      group,
      proxies: { [group.name]: group, [nested.name]: nested, [node.name]: node, [direct.name]: direct },
      busy: false,
      hideUnavailable: false,
      thresholds: { low: 100, medium: 200 },
      testingGroup: "",
      testingNodes: new Set([direct.name]),
      penetrationExpanded: true,
      onTogglePenetration: () => undefined,
      onSelect: async () => undefined,
      onTest: async () => undefined,
      onTestNode: async () => undefined,
    }));
    expect(markup).toContain('class="policy-card embedded"');
    expect(markup).toContain('class="policy-option-latency testing"');
    expect(markup).toContain('class="latency-testing-dots"');
    expect(markup).toContain('class="policy-option split-action disabled"');
    expect(markup).toContain('class="policy-option split-action active"');
    expect(markup).toContain('class="policy-footer open"');
    expect(markup).toContain(`${nested.name} 节点列表`);
    expect(markup).toContain(`aria-label="测试 ${node.name} 延迟"`);
  });
});
