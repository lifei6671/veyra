import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ProxyRoutingSnapshot } from "../lib/proxy-routing";

const useProxyRoutingMock = vi.fn();
vi.mock("../lib/proxy-routing", async (importOriginal) => ({
  ...await importOriginal<typeof import("../lib/proxy-routing")>(),
  useProxyRouting: () => useProxyRoutingMock(),
}));

import { ProxiesPage } from "./proxies/ProxiesPage";
import { RoutingPage } from "./routing/RoutingPage";

const snapshot: ProxyRoutingSnapshot = {
  revision: 7,
  desiredGeneration: 4,
  appliedGeneration: 3,
  runtimeState: "ready",
  applyState: { type: "savedPendingApply" },
  activeSubscriptionId: "sub-a",
  appliedSubscriptionId: "sub-a",
  providers: [{ id: "provider-a", subscriptionId: "sub-a", name: "订阅 A" }],
  nodes: [{ id: "node-a", providerId: "provider-a", name: "节点 A", protocol: "shadowsocks" }],
  defaultTarget: { type: "direct" },
  pools: [{ id: "pool-a", name: "出口组 A", kind: "custom", enabled: false, sources: [], selection: { type: "manual", selectedNodeId: "node-a" }, resolvedNodeIds: ["node-a"] }],
  routes: [{ id: "route-a", name: "规则 A", enabled: false, priority: 0, matcher: { type: "domain", values: ["example.test"] }, target: { type: "direct" } }],
  selectors: [{ poolId: "pool-a", desiredNodeId: "node-a", runtimeNodeId: null, state: "savedOnly" }],
};

const context = (patch: Record<string, unknown> = {}) => ({
  snapshot,
  loading: false,
  error: null,
  pending: false,
  notice: null,
  refresh: vi.fn(),
  mutate: vi.fn(),
  dismissNotice: vi.fn(),
  ...patch,
});

describe("proxy routing production pages", () => {
  beforeEach(() => useProxyRoutingMock.mockReset());

  it("ProxiesPage renders loading, empty and disabled snapshot states", () => {
    useProxyRoutingMock.mockReturnValue(context({ snapshot: null, loading: true }));
    expect(renderToStaticMarkup(createElement(ProxiesPage, { active: true }))).toContain("正在读取出口配置");

    useProxyRoutingMock.mockReturnValue(context({ snapshot: { ...snapshot, pools: [], nodes: [], selectors: [] } }));
    expect(renderToStaticMarkup(createElement(ProxiesPage, { active: true }))).toContain("暂无出口组");

    useProxyRoutingMock.mockReturnValue(context());
    const populated = renderToStaticMarkup(createElement(ProxiesPage, { active: true }));
    expect(populated).toContain("有未应用更改");
    expect(populated).toContain("proxy-group is-disabled");
    expect(populated).toContain("selector-savedOnly");
  });

  it("ProxiesPage renders authoritative error and Notice without patching the snapshot", () => {
    useProxyRoutingMock.mockReturnValue(context({ error: "runtimeUnavailable", notice: { kind: "error", message: "运行状态不可用" } }));
    const html = renderToStaticMarkup(createElement(ProxiesPage, { active: true }));
    expect(html).toContain("出口配置不可用");
    expect(html).toContain("运行状态不可用");
    expect(html).toContain("role=\"alert\"");
  });

  it("RoutingPage renders default, disabled route and global pending controls", () => {
    useProxyRoutingMock.mockReturnValue(context({ pending: true }));
    const html = renderToStaticMarkup(createElement(RoutingPage, { active: true }));
    expect(html).toContain("默认出口");
    expect(html).toContain("直连");
    expect(html).toContain("route-row is-disabled");
    expect(html).toContain("disabled=\"\"");

    useProxyRoutingMock.mockReturnValue(context({ snapshot: { ...snapshot, runtimeState: "transitioning", applyState: { type: "applying", operationId: "operation-test" } } }));
    const applying = renderToStaticMarkup(createElement(RoutingPage, { active: true }));
    expect(applying).toContain("应用中…");
    expect(applying).toContain("disabled=\"\"");
  });
});
