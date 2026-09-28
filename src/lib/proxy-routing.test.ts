import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { encodeProxyRoutingMutation, filterPoolNodes, getProxyRoutingSnapshot, mutateProxyRouting, nodeMatchesFilter, parseMutationResult, parseProxyRoutingSnapshot, parseSnapshotResult, presentMutationError, PROXY_PROTOCOLS, ProxyRoutingResponseOrder, retainAvailableNodeId, runtimeRoutingInvalidationKey, shouldDismissDialog, shouldPollProxyRouting, shouldRefreshAfterMutationError, shouldRefreshForRevision, validProbeUrl, wrappedDialogFocusIndex, type NodeFilter, type ProxyRoutingSnapshot } from "./proxy-routing";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const snapshot: ProxyRoutingSnapshot = {
  revision: 7, desiredGeneration: 4, appliedGeneration: 3, runtimeState: "ready", applyState: { type: "savedPendingApply" }, activeSubscriptionId: "sub-a", appliedSubscriptionId: "sub-a",
  providers: [{ id: "provider-a", subscriptionId: "sub-a", name: "订阅 A" }],
  nodes: [{ id: "node-a", providerId: "provider-a", name: "节点 A", protocol: "shadowsocks" }],
  defaultTarget: { type: "followActiveSubscription" },
  pools: [{ id: "pool-a", name: "出口组 A", kind: "custom", enabled: true, sources: [{ providerId: "provider-a", filter: { regions: [], protocols: ["shadowsocks"], includeKeywords: [], excludeKeywords: [], includeNodeIds: [], excludeNodeIds: [] } }], selection: { type: "manual", selectedNodeId: "node-a" }, resolvedNodeIds: ["node-a"] }],
  routes: [{ id: "route-a", name: "规则 A", enabled: true, priority: 0, matcher: { type: "domain", values: ["example.com"] }, target: { type: "pool", poolId: "pool-a" } }],
  selectors: [{ poolId: "pool-a", desiredNodeId: "node-a", runtimeNodeId: null, state: "savedOnly" }],
};

describe("proxy routing wire contract", () => {
  beforeEach(() => vi.clearAllMocks());

  it("接受 exact snapshot 并拒绝顶层和嵌套额外字段", () => {
    expect(parseProxyRoutingSnapshot(snapshot)).toEqual(snapshot);
    expect(() => parseProxyRoutingSnapshot({ ...snapshot, secret: "hidden" })).toThrow("invalid proxy routing response");
    expect(() => parseProxyRoutingSnapshot({ ...snapshot, nodes: [{ ...snapshot.nodes[0], server: "127.0.0.1" }] })).toThrow("invalid proxy routing response");
    expect(() => parseProxyRoutingSnapshot({ ...snapshot, applyState: { type: "applied", operationId: "extra" } })).toThrow("invalid proxy routing response");
  });

  it("协议闭集与 Rust DTO 的十五种 camelCase 值一致", () => {
    expect(PROXY_PROTOCOLS).toEqual(["socks", "http", "shadowsocks", "vmess", "vless", "trojan", "wireGuard", "hysteria", "hysteria2", "tuic", "shadowTls", "ssh", "naive", "anyTls", "snell"]);
    expect(() => parseProxyRoutingSnapshot({ ...snapshot, nodes: [{ ...snapshot.nodes[0], protocol: "mieru" }] })).toThrow();
  });

  it("QueryError 只接受冻结闭集，Stopped 和 RecoveryRequired 仍是成功 Snapshot", () => {
    for (const error of ["busy", "stateUnavailable", "runtimeUnavailable"] as const) expect(parseSnapshotResult({ status: "error", error })).toEqual({ status: "error", error });
    expect(() => parseSnapshotResult({ status: "error", error: "stopped" })).toThrow();
    expect(parseSnapshotResult({ status: "ok", snapshot: { ...snapshot, runtimeState: "stopped" } }).status).toBe("ok");
    expect(parseSnapshotResult({ status: "ok", snapshot: { ...snapshot, runtimeState: "recoveryRequired" } }).status).toBe("ok");
  });

  it("穷举解析冻结 MutationOutcome 并拒绝额外字段和非法 reason", () => {
    const outcomes = [
      { type: "saved" }, { type: "selectorApplied", poolId: "pool-a", nodeId: "node-a" },
      { type: "selectorNotApplied", poolId: "pool-a", nodeId: "node-a", runtimeNodeId: "node-b" },
      ...["runtimeStopped", "runtimeNotReady", "notInAppliedArtifact", "dispatchUnavailable"].map((reason) => ({ type: "selectorSavedOnly", poolId: "pool-a", nodeId: "node-a", reason })),
      ...["readBackUnavailable", "unknownRuntimeNode", "instanceChanged", "superseded"].map((reason) => ({ type: "selectorApplyUnknown", poolId: "pool-a", nodeId: "node-a", reason })),
      { type: "applyStarted", operationId: "op-a" }, { type: "applyCompleted", operationId: "op-a" },
    ];
    for (const outcome of outcomes) expect(parseMutationResult({ status: "ok", outcome, snapshot }).status).toBe("ok");
    expect(() => parseMutationResult({ status: "ok", outcome: { type: "saved", payload: {} }, snapshot })).toThrow();
    expect(() => parseMutationResult({ status: "ok", outcome: { type: "selectorSavedOnly", poolId: "pool-a", nodeId: "node-a", reason: "future" }, snapshot })).toThrow();
  });

  it("解析 error revision，并按 null 或不同 revision 触发重新查询", () => {
    expect(parseMutationResult({ status: "error", error: "conflict", revision: 8 })).toEqual({ status: "error", error: "conflict", revision: 8 });
    expect(shouldRefreshForRevision(7, 7)).toBe(false);
    expect(shouldRefreshForRevision(7, 8)).toBe(true);
    expect(shouldRefreshForRevision(7, null)).toBe(true);
    expect(shouldRefreshAfterMutationError({ type: "applyConfiguration" }, 7, 7)).toBe(true);
    expect(shouldRefreshAfterMutationError({ type: "deleteRoute", id: "route-a" }, 7, 7)).toBe(false);
    expect(() => parseMutationResult({ status: "error", error: "unknown", revision: 7 })).toThrow();
  });

  it("encoder 重建封闭 mutation，不透传运行时额外字段", () => {
    const unsafe = { type: "setManualSelection", poolId: "pool-a", nodeId: "node-a", arbitrary: { secret: true } } as never;
    expect(encodeProxyRoutingMutation(unsafe)).toEqual({ type: "setManualSelection", poolId: "pool-a", nodeId: "node-a" });
  });

  it("使用唯一两个 command 并以 request 包装 exact mutation", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ status: "ok", snapshot });
    await expect(getProxyRoutingSnapshot()).resolves.toMatchObject({ status: "ok" });
    expect(invoke).toHaveBeenNthCalledWith(1, "get_proxy_routing_snapshot");
    vi.mocked(invoke).mockResolvedValueOnce({ status: "ok", outcome: { type: "saved" }, snapshot: { ...snapshot, revision: 8 } });
    await mutateProxyRouting(7, { type: "setDefaultTarget", target: { type: "direct" } });
    expect(invoke).toHaveBeenNthCalledWith(2, "mutate_proxy_routing", { request: { expectedRevision: 7, mutation: { type: "setDefaultTarget", target: { type: "direct" } } } });
  });

  it("mutation authoritative snapshot 使旧 query 失效且 revision 不倒退", () => {
    const order = new ProxyRoutingResponseOrder();
    const oldQuery = order.beginQuery();
    order.acceptMutation({ ...snapshot, revision: 8 });
    expect(order.acceptQuery(oldQuery, snapshot)).toBe(false);
    const nextQuery = order.beginQuery();
    expect(order.acceptQuery(nextQuery, { ...snapshot, revision: 7 })).toBe(false);
    const newestQuery = order.beginQuery();
    expect(order.acceptQuery(newestQuery, { ...snapshot, revision: 9 })).toBe(true);
  });

  it("后发 query epoch 唯一生效，旧响应不能覆盖新响应", () => {
    const order = new ProxyRoutingResponseOrder();
    const first = order.beginQuery();
    const second = order.beginQuery();
    expect(order.acceptQuery(second, { ...snapshot, revision: 8 })).toBe(true);
    expect(order.acceptQuery(first, { ...snapshot, revision: 9 })).toBe(false);
  });

  it("只对 applying/applyUnknown 持续轮询，终态停止", () => {
    expect(shouldPollProxyRouting({ ...snapshot, applyState: { type: "applying", operationId: "op-a" } })).toBe(true);
    expect(shouldPollProxyRouting({ ...snapshot, applyState: { type: "applyUnknown", operationId: "op-a" } })).toBe(true);
    expect(shouldPollProxyRouting({ ...snapshot, applyState: { type: "savedApplyFailed", operationId: "op-a", error: "startFailed" } })).toBe(false);
    expect(shouldPollProxyRouting({ ...snapshot, applyState: { type: "applied" } })).toBe(false);
  });

  it("Dialog busy 时 Escape 不关闭，空闲时恢复关闭与循环焦点", () => {
    expect(shouldDismissDialog("Escape", true)).toBe(false);
    expect(shouldDismissDialog("Escape", false)).toBe(true);
    expect(shouldDismissDialog("Enter", false)).toBe(false);
    expect(wrappedDialogFocusIndex(0, 3, true)).toBe(2);
    expect(wrappedDialogFocusIndex(2, 3, false)).toBe(0);
    expect(wrappedDialogFocusIndex(1, 3, false)).toBeNull();
  });

  it("Pool 草稿成员与后端 NodeFilter 语义一致", () => {
    const node = { id: "node-a", providerId: "provider-a", name: "香港 Premium", protocol: "shadowsocks" as const };
    const filter = (patch: Partial<NodeFilter>): NodeFilter => ({ regions: [], protocols: [], includeKeywords: [], excludeKeywords: [], includeNodeIds: [], excludeNodeIds: [], ...patch });
    expect(nodeMatchesFilter(node, filter({ regions: ["香港"], protocols: ["shadowsocks"], includeKeywords: ["港", "premium"], includeNodeIds: ["node-a"] }))).toBe(true);
    expect(nodeMatchesFilter(node, filter({ regions: ["日本"] }))).toBe(false);
    expect(nodeMatchesFilter(node, filter({ protocols: ["vmess"] }))).toBe(false);
    expect(nodeMatchesFilter(node, filter({ includeKeywords: ["香港", "missing"] }))).toBe(false);
    expect(nodeMatchesFilter(node, filter({ excludeKeywords: ["premium"] }))).toBe(false);
    expect(nodeMatchesFilter(node, filter({ includeNodeIds: ["node-b"] }))).toBe(false);
    expect(nodeMatchesFilter(node, filter({ excludeNodeIds: ["node-a"] }))).toBe(false);
    expect(nodeMatchesFilter({ ...node, name: "ÜBER US" }, filter({ includeKeywords: ["über", "us"] }))).toBe(false);
    const available = filterPoolNodes([node, { ...node, id: "node-b", providerId: "provider-b" }], [{ providerId: "provider-a", filter: filter({ regions: ["香港"] }) }]);
    expect(available.map((item) => item.id)).toEqual(["node-a"]);
    expect(retainAvailableNodeId("node-a", available)).toBe("node-a");
    expect(retainAvailableNodeId("node-b", available)).toBe("");
  });

  it("Probe URL 与后端仅允许 HTTPS 或无附加字段的 loopback HTTP", () => {
    for (const value of ["https://example.com/probe?x=1#ok", "http://127.0.0.1/probe", "http://[::1]/probe"]) expect(validProbeUrl(value)).toBe(true);
    for (const value of ["http://localhost/probe", "http://example.com/probe", "http://user@127.0.0.1/", "http://127.0.0.1/?x=1", "http://127.0.0.1/#x", "ftp://127.0.0.1/"]) expect(validProbeUrl(value)).toBe(false);
  });

  it("runtime routing invalidation 只响应 lifecycle、applied identity 和 operation 终态", () => {
    const base = { source: "runtime", revision: 1, observedAtMs: 1, trafficHistory: [], captureMode: "off", sidecarLifecycle: "ready", uploadRateBps: 0, downloadRateBps: 0, uploadTotalBytes: 0, downloadTotalBytes: 0, connectionCount: 0, logSummary: [], appliedSubscriptionId: "sub-a", appliedConfigurationGeneration: 3, subscriptionSwitch: null, managedProxyAvailable: true, coreMemoryBytes: 1 } as const;
    expect(runtimeRoutingInvalidationKey({ ...base, observedAtMs: 2, uploadRateBps: 20 })).toBe(runtimeRoutingInvalidationKey(base));
    expect(runtimeRoutingInvalidationKey({ ...base, sidecarLifecycle: "stopped" })).not.toBe(runtimeRoutingInvalidationKey(base));
    expect(runtimeRoutingInvalidationKey({ ...base, appliedConfigurationGeneration: 4 })).not.toBe(runtimeRoutingInvalidationKey(base));
    expect(runtimeRoutingInvalidationKey({ ...base, subscriptionSwitch: { operationId: "op-a", status: "failed", errorCode: "startFailed" } })).not.toBe(runtimeRoutingInvalidationKey(base));
  });
});

describe("proxy routing presentation adapter", () => {
  it("routes each mutation error to exactly one local or global destination", () => {
    const global = vi.fn();
    const local = vi.fn();

    presentMutationError("输入无效，请检查后重试", global, { onPresentationError: local });
    expect(local).toHaveBeenCalledOnce();
    expect(local).toHaveBeenCalledWith("输入无效，请检查后重试");
    expect(global).not.toHaveBeenCalled();

    presentMutationError("保存失败，原有配置未改变", global);
    expect(global).toHaveBeenCalledOnce();
    expect(global).toHaveBeenCalledWith({ kind: "error", message: "保存失败，原有配置未改变" });
  });

  async function providerHarness() {
    type ProviderContext = { mutate: (...args: unknown[]) => Promise<unknown> };
    vi.resetModules();
    const global = vi.fn();
    const stateSetters = Array.from({ length: 5 }, () => vi.fn());
    const providerMarker = Symbol("ProxyRoutingContext.Provider");
    let stateIndex = 0;
    let contextValue: ProviderContext | null = null;

    vi.doMock("react", async (importOriginal) => ({
      ...await importOriginal<typeof import("react")>(),
      createContext: () => ({ Provider: providerMarker }),
      createElement: (type: unknown, props: { value?: typeof contextValue } | null) => {
        if (type === providerMarker) contextValue = props?.value ?? null;
        return null;
      },
      useCallback: <T,>(callback: T) => callback,
      useContext: () => contextValue,
      useEffect: () => undefined,
      useMemo: <T,>(factory: () => T) => factory(),
      useRef: <T,>(initial: T) => ({ current: initial }),
      useState: () => {
        const values = [snapshot, false, null, false, null] as const;
        const index = stateIndex++;
        return [values[index], stateSetters[index]];
      },
    }));
    vi.doMock("../components/ToastHost", () => ({ useToastOwner: () => global }));

    const module = await import("./proxy-routing");
    const tauri = await import("@tauri-apps/api/core");
    module.ProxyRoutingProvider({ children: null });
    const captured = contextValue as unknown as ProviderContext | null;
    if (captured === null) throw new Error("provider context was not captured");
    return { context: captured, global, invoke: vi.mocked(tauri.invoke) };
  }

  it("keeps business and response failures invocation-local when a Dialog sink is supplied", async () => {
    const business = await providerHarness();
    const businessLocal = vi.fn();
    business.invoke.mockResolvedValueOnce({ status: "error", error: "validationFailed", revision: 7 });
    await business.context.mutate({ type: "deleteRoute", id: "route-a" }, { onPresentationError: businessLocal });
    expect(businessLocal).toHaveBeenCalledExactlyOnceWith("更改未通过完整配置校验");
    expect(business.global.mock.calls.filter(([notice]) => notice !== null)).toHaveLength(0);
    expect(business.invoke).toHaveBeenCalledWith("mutate_proxy_routing", {
      request: { expectedRevision: 7, mutation: { type: "deleteRoute", id: "route-a" } },
    });

    const invalid = await providerHarness();
    const invalidLocal = vi.fn();
    invalid.invoke.mockResolvedValueOnce({ status: "future" });
    await invalid.context.mutate({ type: "deleteRoute", id: "route-a" }, { onPresentationError: invalidLocal });
    expect(invalidLocal).toHaveBeenCalledExactlyOnceWith("响应格式不可用，请刷新后重试");
    expect(invalid.global.mock.calls.filter(([notice]) => notice !== null)).toHaveLength(0);

    const rejected = await providerHarness();
    const rejectedLocal = vi.fn();
    rejected.invoke.mockRejectedValue(new Error("invoke failed"));
    await rejected.context.mutate({ type: "deleteRoute", id: "route-a" }, { onPresentationError: rejectedLocal });
    expect(rejectedLocal).toHaveBeenCalledExactlyOnceWith("响应格式不可用，请刷新后重试");
    expect(rejected.global.mock.calls.filter(([notice]) => notice !== null)).toHaveLength(0);
  });

  it("keeps non-Dialog errors and successful or warning outcomes global", async () => {
    const failed = await providerHarness();
    failed.invoke.mockResolvedValueOnce({ status: "error", error: "saveFailed", revision: 7 });
    await failed.context.mutate({ type: "deleteRoute", id: "route-a" });
    expect(failed.global.mock.calls.filter(([notice]) => notice !== null)).toEqual([
      [{ kind: "error", message: "保存失败，原有配置未改变" }],
    ]);

    const succeeded = await providerHarness();
    const successLocal = vi.fn();
    succeeded.invoke.mockResolvedValueOnce({ status: "ok", outcome: { type: "saved" }, snapshot: { ...snapshot, revision: 8 } });
    await succeeded.context.mutate({ type: "deleteRoute", id: "route-a" }, { onPresentationError: successLocal });
    expect(successLocal).not.toHaveBeenCalled();
    expect(succeeded.global.mock.calls.filter(([notice]) => notice !== null)).toEqual([
      [{ kind: "success", message: "更改已保存，等待应用" }],
    ]);

    const warned = await providerHarness();
    warned.invoke.mockResolvedValueOnce({ status: "ok", outcome: { type: "selectorNotApplied", poolId: "pool-a", nodeId: "node-a", runtimeNodeId: "node-b" }, snapshot: { ...snapshot, revision: 8 } });
    await warned.context.mutate({ type: "setManualSelection", poolId: "pool-a", nodeId: "node-a" });
    expect(warned.global.mock.calls.filter(([notice]) => notice !== null)).toEqual([
      [{ kind: "warning", message: "选择已保存，运行时仍使用原节点" }],
    ]);
  });
});
