import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { ControllerProxy, ProxiesResponse, Subscription } from "../api/types";
import { clampNodeCardWidth, completeProxyViewOrder, formatLatencyTime, getPenetrationGroup, getProxyViewGroups, isVisibleProxy, latencyClass, moveProxyViewId, NodeHealthDots, PenetrationGroup, PolicyLatency, ProxyGroupCard, ProxyOption, proxyMatchesQuery, proxyOptionAction, proxyViewSettings, proxyViewTab, recentLatencyHistory, reorderProxyViewIds, selectedProxyDelay, sortPenetrationOptions, sortProxyNodes, sortProxyViewItems, summarizeGroupLatencyTest, toggleProxyCardsCollapsed, uniqueSubscriptionNodes } from "./ProxiesPage";

describe("proxy view grouping", () => {
  it("restores a valid proxy view tab and falls back to strategy groups", () => {
    expect(proxyViewTab("nodes")).toBe("nodes");
    expect(proxyViewTab("subscriptions")).toBe("subscriptions");
    expect(proxyViewTab("unknown")).toBe("groups");
    expect(proxyViewTab(null)).toBe("groups");
  });

  it("restores one shared strategy settings model and migrates the old strategy order", () => {
    expect(proxyViewSettings(JSON.stringify({ nodeSort: "nameDesc", groupByProvider: false, nodeCardMinWidth: 180, strategyOrder: ["AI", "Speed"] })))
      .toEqual({ nodeSort: "nameDesc", groupByProvider: false, nodeCardMinWidth: 180, strategyOrder: ["AI", "Speed"] });
    expect(proxyViewSettings(JSON.stringify({ groups: { mode: "custom", order: ["AI", 3, "Speed"] } })).strategyOrder)
      .toEqual(["AI", "Speed"]);
    expect(proxyViewSettings("not-json"))
      .toEqual({ nodeSort: "latencyAsc", groupByProvider: true, nodeCardMinWidth: 145, strategyOrder: [] });
    expect(clampNodeCardWidth(20)).toBe(100);
    expect(clampNodeCardWidth(500)).toBe(320);
  });

  it("sorts cards by latency, name, and saved custom order without dropping new items", () => {
    const items = [
      { id: "slow", name: "Charlie", delay: 300 },
      { id: "new", name: "Bravo", delay: null },
      { id: "fast", name: "Alpha", delay: 80 },
    ];
    const sort = (mode: "latency" | "name" | "custom", order: string[] = []) => sortProxyViewItems(items, { mode, order }, item => item.id, item => item.name, item => item.delay).map(item => item.id);
    expect(sort("latency")).toEqual(["fast", "slow", "new"]);
    expect(sort("name")).toEqual(["fast", "new", "slow"]);
    expect(sort("custom", ["fast", "slow"])).toEqual(["fast", "slow", "new"]);
  });

  it("completes, drags, and keyboard-moves custom display order", () => {
    expect(completeProxyViewOrder(["b", "removed", "b"], ["a", "b", "c"]))
      .toEqual(["b", "a", "c"]);
    expect(reorderProxyViewIds(["a", "b", "c"], "c", "a")).toEqual(["c", "a", "b"]);
    expect(moveProxyViewId(["a", "b", "c"], 0, 1)).toEqual(["b", "a", "c"]);
  });

  it("toggles every card in only the active proxy tab", () => {
    const collapsed = { "nodes:auto": true, "subscriptions:main": true };
    const closed = toggleProxyCardsCollapsed(collapsed, "groups", ["Speed", "AI"]);
    expect(closed).toEqual({ ...collapsed, "groups:Speed": true, "groups:AI": true });
    expect(toggleProxyCardsCollapsed(closed, "groups", ["Speed", "AI"]))
      .toEqual({ ...collapsed, "groups:Speed": false, "groups:AI": false });
  });

  it("keeps the live UI contract of 13 strategy groups and 2 node groups", () => {
    const proxies: Record<string, ControllerProxy> = {};
    for (const name of ["Speed", "AI", "Youtube", "TikTok", "Netflix", "Github", "Google", "Microsoft", "Apple", "Games", "国外", "国内", "其他"]) {
      proxies[name] = { name, type: "Selector", all: ["直连", "所有-自动", "所有-手动", "拒绝"] };
    }
    proxies.GLOBAL = { name: "GLOBAL", type: "Selector", all: [] };
    proxies["所有-自动"] = { name: "所有-自动", type: "URLTest", all: ["订阅 | 美国-01"] };
    proxies["所有-手动"] = { name: "所有-手动", type: "Selector", all: ["订阅 | 美国-01"] };
    proxies["订阅 | 美国-01"] = { name: "订阅 | 美国-01", type: "VLESS" };

    const result = getProxyViewGroups({ proxies } satisfies ProxiesResponse);
    expect(result.customGroups).toHaveLength(13);
    expect(result.nodeGroups.map(group => group.name)).toEqual(["所有-自动", "所有-手动"]);
  });

  it("distinguishes automatic and manual node groups", () => {
    expect(proxyOptionAction("URLTest")).toBe("test");
    expect(proxyOptionAction("Selector")).toBe("select");
  });

  it("keeps automatic penetration nodes read-only except for their latency badge", () => {
    const node = { name: "订阅 | 美国-01", type: "VLESS", udp: true, history: [{ time: "now", delay: 80 }] } satisfies ControllerProxy;
    const group = { name: "所有-自动", type: "URLTest", now: node.name, all: [node.name] } satisfies ControllerProxy;
    const markup = renderToStaticMarkup(createElement(PenetrationGroup, {
      group,
      data: { proxies: { [group.name]: group, [node.name]: node } },
      busy: false,
      onSelect: async () => undefined,
      onTest: async () => undefined,
      onTestNode: async () => undefined,
      hideUnavailable: false,
      thresholds: { low: 100, medium: 200 },
      history: [],
      testing: false,
    }));
    expect(markup).not.toContain(`aria-label="选择 ${node.name}"`);
    expect(markup).toContain(`aria-label="测试 ${node.name} 延迟"`);
    expect(markup).toContain("policy-option-latency success");
  });

  it("keeps a subscription node card static and animates only its latency button", () => {
    const node = { name: "订阅 | 美国-03", type: "Hysteria2", udp: true, history: [{ time: "now", delay: 193 }] } satisfies ControllerProxy;
    const markup = renderToStaticMarkup(createElement(ProxyOption, {
      option: node,
      active: false,
      disabled: true,
      thresholds: { low: 500, medium: 1000 },
      testing: true,
      onTest: () => undefined,
    }));
    expect(markup).toContain('class="policy-option-main static"');
    expect(markup.match(/<button/g)).toHaveLength(1);
    expect(markup).toContain(`aria-label="测试 ${node.name} 延迟"`);
    expect(markup).toContain('aria-busy="true"');
    expect(markup).toContain('class="latency-testing-dots"');
    expect(markup.match(/<i><\/i>/g)).toHaveLength(3);
  });

  it("lets manual penetration nodes select from the card body and test from the badge", () => {
    const node = { name: "订阅 | 美国-04", type: "VLESS", udp: true, history: [{ time: "now", delay: 252 }] } satisfies ControllerProxy;
    const group = { name: "所有-手动", type: "Selector", now: node.name, all: [node.name] } satisfies ControllerProxy;
    const markup = renderToStaticMarkup(createElement(PenetrationGroup, {
      group,
      data: { proxies: { [group.name]: group, [node.name]: node } },
      busy: false,
      onSelect: async () => undefined,
      onTest: async () => undefined,
      onTestNode: async () => undefined,
      hideUnavailable: false,
      thresholds: { low: 100, medium: 200 },
      history: [],
      testing: false,
    }));
    expect(markup).toContain(`aria-label="选择 ${node.name}"`);
    expect(markup).toContain(`aria-label="测试 ${node.name} 延迟"`);
    expect(markup).toContain("policy-option-latency danger");
  });

  it("only hides nodes explicitly reported unavailable by the controller", () => {
    expect(isVisibleProxy({ name: "untested", type: "VLESS" }, true)).toBe(true);
    expect(isVisibleProxy({ name: "down", type: "VLESS", alive: false, history: [{ time: "now", delay: 188 }] }, true)).toBe(false);
  });

  it("uses configured latency thresholds", () => {
    const thresholds = { low: 100, medium: 200 };
    expect(latencyClass(80, thresholds)).toBe("success");
    expect(latencyClass(150, thresholds)).toBe("warning");
    expect(latencyClass(250, thresholds)).toBe("danger");
  });

  it("matches every space-separated search term against node display fields", () => {
    const node = { name: "订阅 | 美国-03", type: "Hysteria2", udp: true } satisfies ControllerProxy;
    expect(proxyMatchesQuery(node, "美国-03 hy2")).toBe(true);
    expect(proxyMatchesQuery(node, "美国 udp")).toBe(true);
    expect(proxyMatchesQuery(node, "美国 vless")).toBe(false);
  });

  it("shows the newest ten latency history records first and keeps timeout records", () => {
    const history = Array.from({ length: 12 }, (_, index) => ({ time: `2026-09-28T10:00:${String(index).padStart(2, "0")}+08:00`, delay: index === 11 ? 0 : 100 + index }));
    const recent = recentLatencyHistory(history);
    expect(recent).toHaveLength(10);
    expect(recent[0]).toEqual(history[11]);
    expect(recent[9]).toEqual(history[2]);
  });

  it("formats latency timestamps like the live history tooltip", () => {
    expect(formatLatencyTime("2026-09-28T19:30:20")).toBe("2026-09-28 19:30:20");
    expect(formatLatencyTime("not-a-date")).toBe("not-a-date");
  });

  it("shows the selected node delay on a measured strategy and keeps its history tooltip", () => {
    const markup = renderToStaticMarkup(createElement(PolicyLatency, {
      groupName: "AI",
      history: [{ time: "2026-09-28T19:30:20", delay: 217, node: "订阅 | 美国-05" }],
      delay: 217,
      thresholds: { low: 100, medium: 250 },
      busy: false,
      onTest: async () => undefined,
    }));
    expect(markup).toContain('class="policy-latency warning"');
    expect(markup).toContain(">217</button>");
    expect(markup).toContain("217ms");
  });

  it("keeps the lightning icon when the selected node has not been measured", () => {
    const markup = renderToStaticMarkup(createElement(PolicyLatency, {
      groupName: "AI",
      history: [],
      delay: null,
      thresholds: { low: 100, medium: 250 },
      busy: false,
      onTest: async () => undefined,
    }));
    expect(markup).toContain("<svg");
  });

  it("resolves the selected nested strategy for policy penetration", () => {
    const nested = { name: "所有-手动", type: "Selector", all: ["订阅 | 美国-01"] } satisfies ControllerProxy;
    const data = { proxies: { Speed: { name: "Speed", type: "Selector", now: nested.name }, [nested.name]: nested } };
    expect(getPenetrationGroup(data.proxies.Speed, data)).toBe(nested);
  });

  it("does not offer policy penetration for a terminal node", () => {
    const group = { name: "Apple", type: "Selector", now: "直连" } satisfies ControllerProxy;
    const data = { proxies: { Apple: group, "直连": { name: "直连", type: "Direct" } } };
    expect(getPenetrationGroup(group, data)).toBeUndefined();
  });

  it("resolves latency from the selected terminal node through nested strategies", () => {
    const node = { name: "订阅 | 美国-04", type: "VLESS", history: [{ time: "now", delay: 252 }] } satisfies ControllerProxy;
    const nested = { name: "所有-手动", type: "Selector", now: node.name, all: [node.name] } satisfies ControllerProxy;
    const strategy = { name: "Speed", type: "Selector", now: nested.name, all: [nested.name] } satisfies ControllerProxy;
    const data = { proxies: { [strategy.name]: strategy, [nested.name]: nested, [node.name]: node } } satisfies ProxiesResponse;
    expect(selectedProxyDelay(strategy, data)).toBe(252);
    expect(selectedProxyDelay(nested, data)).toBe(252);
  });

  it("keeps unmeasured and cyclic selections in the lightning state", () => {
    const untested = { name: "untested", type: "VLESS" } satisfies ControllerProxy;
    const group = { name: "group", type: "Selector", now: untested.name } satisfies ControllerProxy;
    expect(selectedProxyDelay(group, { proxies: { group, untested } })).toBeNull();

    const cyclic = { name: "cyclic", type: "Selector", now: "cyclic", history: [{ time: "now", delay: 88 }] } satisfies ControllerProxy;
    expect(selectedProxyDelay(cyclic, { proxies: { cyclic } })).toBeNull();

    const unselected = { name: "unselected", type: "Selector", history: [{ time: "now", delay: 91 }] } satisfies ControllerProxy;
    expect(selectedProxyDelay(unselected, { proxies: { unselected } })).toBeNull();
  });

  it("orders penetration nodes by measured latency and keeps untested nodes last", () => {
    const options = [
      { name: "untested-a", type: "VLESS" },
      { name: "slow", type: "VLESS", history: [{ time: "now", delay: 405 }] },
      { name: "fast", type: "VLESS", history: [{ time: "now", delay: 193 }] },
      { name: "untested-b", type: "VLESS" },
    ];
    expect(sortPenetrationOptions(options).map(option => option.name)).toEqual(["fast", "slow", "untested-a", "untested-b"]);
    expect(sortProxyNodes(options, "latencyDesc").map(option => option.name)).toEqual(["slow", "fast", "untested-a", "untested-b"]);
    expect(sortProxyNodes(options, "nameAsc").map(option => option.name)).toEqual(["fast", "slow", "untested-a", "untested-b"]);
    expect(sortProxyNodes(options, "default")).toEqual(options);
  });

  it("summarizes missing or timed-out group results as failed tests", () => {
    expect(summarizeGroupLatencyTest({ "node-a": 193, "node-b": 0, "node-c": 229 }, 5))
      .toEqual({ success: 2, failed: 3 });
  });

  it("renders the testing icon state without removing the history tooltip", () => {
    const markup = renderToStaticMarkup(createElement(PolicyLatency, {
      groupName: "所有-手动",
      history: [{ time: "2026-09-28T19:30:20", delay: 217, node: "订阅 | 美国-05" }],
      delay: 217,
      thresholds: { low: 100, medium: 250 },
      busy: true,
      testing: true,
      onTest: async () => undefined,
    }));
    expect(markup).toContain('aria-busy="true"');
    expect(markup).toContain('class="testing-bolt"');
    expect(markup).toContain("217ms");
  });

  it("collects every subscription node once for a global latency test", () => {
    const subscription = (id: string, name: string) => ({
      id, name, format: "clash", nodeCount: 1, createdAt: "now", updatedAt: "now", kernelStale: false,
    }) satisfies Subscription;
    const entries = [
      { name: "订阅 A | 美国-01", type: "VLESS" },
      { name: "订阅 B | 日本-01", type: "VLESS" },
      { name: "所有-自动", type: "URLTest", all: ["订阅 A | 美国-01", "订阅 B | 日本-01"] },
    ] satisfies ControllerProxy[];

    expect(uniqueSubscriptionNodes([subscription("a", "订阅 A"), subscription("b", "订阅 B"), subscription("a-copy", "订阅 A")], entries).map(node => node.name))
      .toEqual(["订阅 A | 美国-01", "订阅 B | 日本-01"]);
  });

  it("only collapses node cards from the top header and marks the selected node as a hollow dot", () => {
    const fast = { name: "订阅 | 美国-01", type: "VLESS", history: [{ time: "now", delay: 80 }] } satisfies ControllerProxy;
    const untested = { name: "订阅 | 美国-02", type: "VLESS" } satisfies ControllerProxy;
    const group = { name: "所有-自动", type: "URLTest", now: fast.name, all: [untested.name, fast.name] } satisfies ControllerProxy;
    const data = { proxies: { [group.name]: group, [fast.name]: fast, [untested.name]: untested } } satisfies ProxiesResponse;
    const renderCard = (collapsed: boolean) => renderToStaticMarkup(createElement(ProxyGroupCard, {
      group,
      data,
      collapsed,
      busy: false,
      onToggle: () => undefined,
      onSelect: async () => undefined,
      onTest: async () => undefined,
      onTestNode: async () => undefined,
      variant: "nodes",
      hideUnavailable: false,
      thresholds: { low: 100, medium: 200 },
      history: [],
      latencyHistory: {},
      testingGroup: "",
    }));

    const expanded = renderCard(false);
    expect(expanded).toContain('aria-label="收起 所有-自动"');
    expect(expanded).toContain('<h3 class="node-provider-title">订阅</h3>');
    expect(expanded).toContain('class="node-provider-collapse" aria-hidden="false"');
    expect(expanded).toContain('class="node-health-summary" aria-hidden="true"');
    expect(expanded).not.toContain('aria-label="收起 所有-自动 订阅"');
    expect(expanded).not.toContain('aria-label="展开 所有-自动 订阅"');
    expect(expanded).toContain(`aria-label="测试 ${fast.name} 延迟"`);
    expect(expanded).not.toContain("收起节点");
    expect(expanded).not.toContain("展开节点");
    expect(expanded.indexOf(`aria-label="测试 ${fast.name} 延迟"`))
      .toBeLessThan(expanded.indexOf(`aria-label="测试 ${untested.name} 延迟"`));

    const cardClosed = renderCard(true);
    expect(cardClosed).toContain('aria-label="展开 所有-自动"');
    expect(cardClosed).toContain("节点状态：1 已测速，1 未测速");
    expect(cardClosed).toContain('class="success selected"');
    expect(cardClosed).toContain('class="node-provider-collapse collapsed" aria-hidden="true" inert=""');
    expect(cardClosed).toContain('class="node-health-summary visible" aria-hidden="false"');
  });

  it("keeps strategy content mounted for animation and shows latency dots while collapsed", () => {
    const fast = { name: "直连", type: "Direct", history: [{ time: "now", delay: 80 }] } satisfies ControllerProxy;
    const group = { name: "Speed", type: "Selector", now: fast.name, all: [fast.name] } satisfies ControllerProxy;
    const data = { proxies: { [group.name]: group, [fast.name]: fast } } satisfies ProxiesResponse;
    const markup = renderToStaticMarkup(createElement(ProxyGroupCard, {
      group,
      data,
      collapsed: true,
      busy: false,
      onToggle: () => undefined,
      onSelect: async () => undefined,
      onTest: async () => undefined,
      onTestNode: async () => undefined,
      variant: "policy",
      hideUnavailable: false,
      thresholds: { low: 100, medium: 200 },
      history: [],
      latencyHistory: {},
      testingGroup: "",
    }));
    expect(markup).toContain('class="node-health-summary visible" aria-hidden="false"');
    expect(markup).toContain('class="policy-card-collapse collapsed" aria-hidden="true" inert=""');
    expect(markup).toContain("节点状态：1 已测速，0 未测速");
    expect(markup).toContain("直连");
  });

  it("filters child node cards without hiding the parent group or changing its total", () => {
    const fast = { name: "订阅 | 美国-01", type: "VLESS", history: [{ time: "now", delay: 80 }] } satisfies ControllerProxy;
    const other = { name: "订阅 | 日本-01", type: "Hysteria2" } satisfies ControllerProxy;
    const group = { name: "所有-自动", type: "URLTest", now: fast.name, all: [fast.name, other.name] } satisfies ControllerProxy;
    const data = { proxies: { [group.name]: group, [fast.name]: fast, [other.name]: other } } satisfies ProxiesResponse;
    const markup = renderToStaticMarkup(createElement(ProxyGroupCard, {
      group,
      data,
      collapsed: false,
      busy: false,
      onToggle: () => undefined,
      onSelect: async () => undefined,
      onTest: async () => undefined,
      onTestNode: async () => undefined,
      variant: "nodes",
      hideUnavailable: false,
      thresholds: { low: 100, medium: 200 },
      history: [],
      latencyHistory: {},
      testingGroup: "",
      query: "美国 vless",
    }));
    expect(markup).toContain("URLTest (1/2)");
    expect(markup).toContain(fast.name);
    expect(markup).not.toContain(other.name);
  });

  it("renders node status dots with latency colors and an untested state", () => {
    const markup = renderToStaticMarkup(createElement(NodeHealthDots, {
      options: [
        { name: "fast", type: "VLESS", history: [{ time: "now", delay: 80 }] },
        { name: "medium", type: "VLESS", history: [{ time: "now", delay: 150 }] },
        { name: "slow", type: "VLESS", history: [{ time: "now", delay: 250 }] },
        { name: "untested", type: "VLESS" },
      ],
      selectedName: "medium",
      thresholds: { low: 100, medium: 200 },
    }));
    expect(markup).toContain('class="success"');
    expect(markup).toContain('class="warning selected"');
    expect(markup).toContain('class="danger"');
    expect(markup).toContain('class="untested"');
  });
});
