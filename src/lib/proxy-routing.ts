import { invoke } from "@tauri-apps/api/core";
import { createContext, createElement, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { subscribeRuntimeObservationDelta, type RuntimeObservation } from "./observability";
import { subscribeSubscriptionStateChanged } from "./subscriptions";

export const MAX_NAME_LENGTH = 80;
export const MAX_FILTER_VALUES = 64;
export const MAX_MATCHER_VALUES = 500;
export const PROXY_PROTOCOLS = ["socks", "http", "shadowsocks", "vmess", "vless", "trojan", "wireGuard", "hysteria", "hysteria2", "tuic", "shadowTls", "ssh", "naive", "anyTls", "snell"] as const;
export type ProxyProtocol = typeof PROXY_PROTOCOLS[number];
export type QueryError = "busy" | "stateUnavailable" | "runtimeUnavailable";
export type RuntimeState = "stopped" | "ready" | "transitioning" | "recoveryRequired";
export type ApplyError = "configurationFailed" | "stateChanged" | "stopFailed" | "startFailed" | "recoveryRequired";
export type ApplyState = { type: "applied" } | { type: "savedPendingApply" } |
  { type: "applying"; operationId: string } | { type: "savedApplyFailed"; operationId: string; error: ApplyError } |
  { type: "applyUnknown"; operationId: string };
export type NodeFilter = { regions: string[]; protocols: ProxyProtocol[]; includeKeywords: string[]; excludeKeywords: string[]; includeNodeIds: string[]; excludeNodeIds: string[] };
export type PoolSource = { providerId: string; filter: NodeFilter };
export type Selection = { type: "manual"; selectedNodeId: string | null } | { type: "urlTest"; probeUrl: string; intervalSeconds: number; toleranceMs: number };

export function nodeMatchesFilter(node: NodeSnapshot, filter: NodeFilter): boolean {
  const asciiLower = (value: string) => value.replace(/[A-Z]/g, (character) => character.toLowerCase());
  const name = asciiLower(node.name);
  const includes = (value: string) => name.includes(asciiLower(value));
  return (filter.regions.length === 0 || filter.regions.some(includes))
    && (filter.protocols.length === 0 || filter.protocols.includes(node.protocol))
    && (filter.includeKeywords.length === 0 || filter.includeKeywords.every(includes))
    && !filter.excludeKeywords.some(includes)
    && (filter.includeNodeIds.length === 0 || filter.includeNodeIds.includes(node.id))
    && !filter.excludeNodeIds.includes(node.id);
}

export function filterPoolNodes(nodes: NodeSnapshot[], sources: PoolSource[]): NodeSnapshot[] {
  const filters = new Map(sources.map((source) => [source.providerId, source.filter]));
  return nodes.filter((node) => {
    const filter = filters.get(node.providerId);
    return filter !== undefined && nodeMatchesFilter(node, filter);
  });
}

export function retainAvailableNodeId(selectedNodeId: string, availableNodes: NodeSnapshot[]): string {
  return selectedNodeId && !availableNodes.some((node) => node.id === selectedNodeId) ? "" : selectedNodeId;
}

export function validProbeUrl(value: string): boolean {
  try {
    const url = new URL(value);
    if (new TextEncoder().encode(value).length > 2048) return false;
    if (url.protocol === "https:") return true;
    return url.protocol === "http:"
      && !url.username
      && !url.password
      && !url.search
      && !url.hash
      && (url.hostname === "127.0.0.1" || url.hostname === "[::1]");
  } catch {
    return false;
  }
}
export type Matcher = { type: "domain" | "domainSuffix" | "application" | "ipCidr"; values: string[] } | { type: "port"; values: number[] } | { type: "protocol"; values: ("tcp" | "udp")[] };
export type DefaultTarget = { type: "followActiveSubscription" } | { type: "pool"; poolId: string } | { type: "direct" } | { type: "block" };
export type RouteTarget = Exclude<DefaultTarget, { type: "followActiveSubscription" }>;
export type ProviderSnapshot = { id: string; subscriptionId: string; name: string };
export type NodeSnapshot = { id: string; providerId: string; name: string; protocol: ProxyProtocol };
export type PoolSnapshot = { id: string; name: string; kind: "implicitProvider" | "custom"; enabled: boolean; sources: PoolSource[]; selection: Selection; resolvedNodeIds: string[] };
export type RouteSnapshot = { id: string; name: string; enabled: boolean; priority: number; matcher: Matcher; target: RouteTarget };
export type SelectorSnapshot = { poolId: string; desiredNodeId: string | null; runtimeNodeId: string | null; state: "inSync" | "savedOnly" | "notApplied" | "unknown" };
export type ProxyRoutingSnapshot = {
  revision: number; desiredGeneration: number; appliedGeneration: number | null; runtimeState: RuntimeState; applyState: ApplyState;
  activeSubscriptionId: string | null; appliedSubscriptionId: string | null; providers: ProviderSnapshot[]; nodes: NodeSnapshot[];
  defaultTarget: DefaultTarget; pools: PoolSnapshot[]; routes: RouteSnapshot[]; selectors: SelectorSnapshot[];
};
export type ProxyRoutingMutation =
  | { type: "createCustomPool"; name: string; enabled: boolean; sources: PoolSource[]; selection: Selection }
  | { type: "updateCustomPool"; id: string; name: string; enabled: boolean; sources: PoolSource[]; selection: Selection }
  | { type: "deleteCustomPool"; id: string }
  | { type: "setManualSelection"; poolId: string; nodeId: string }
  | { type: "setDefaultTarget"; target: DefaultTarget }
  | { type: "createRoute"; name: string; enabled: boolean; matcher: Matcher; target: RouteTarget; insertAt: number }
  | { type: "updateRoute"; id: string; name: string; enabled: boolean; matcher: Matcher; target: RouteTarget }
  | { type: "deleteRoute"; id: string }
  | { type: "reorderRoutes"; routeIds: string[] }
  | { type: "applyConfiguration" };
export type SelectorSavedOnlyReason = "runtimeStopped" | "runtimeNotReady" | "notInAppliedArtifact" | "dispatchUnavailable";
export type SelectorUnknownReason = "readBackUnavailable" | "unknownRuntimeNode" | "instanceChanged" | "superseded";
export type MutationOutcome = { type: "saved" } | { type: "selectorApplied"; poolId: string; nodeId: string } |
  { type: "selectorNotApplied"; poolId: string; nodeId: string; runtimeNodeId: string } |
  { type: "selectorSavedOnly"; poolId: string; nodeId: string; reason: SelectorSavedOnlyReason } |
  { type: "selectorApplyUnknown"; poolId: string; nodeId: string; reason: SelectorUnknownReason } |
  { type: "applyStarted"; operationId: string } | { type: "applyCompleted"; operationId: string };
export type MutationError = "invalidInput" | "busy" | "conflict" | "notFound" | "referenceConflict" | "validationFailed" | "saveFailed" | "stateUnavailable" | "runtimeUnavailable" | ApplyError;
export type SnapshotResult = { status: "ok"; snapshot: ProxyRoutingSnapshot } | { status: "error"; error: QueryError };
export type MutationResult = { status: "ok"; outcome: MutationOutcome; snapshot: ProxyRoutingSnapshot } | { status: "error"; error: MutationError; revision: number | null };

const queryErrors: QueryError[] = ["busy", "stateUnavailable", "runtimeUnavailable"];
const mutationErrors: MutationError[] = ["invalidInput", "busy", "conflict", "notFound", "referenceConflict", "validationFailed", "saveFailed", "stateUnavailable", "runtimeUnavailable", "configurationFailed", "stateChanged", "stopFailed", "startFailed", "recoveryRequired"];
const protocols = new Set<string>(PROXY_PROTOCOLS);
const exact = (value: Record<string, unknown>, keys: readonly string[]) => Object.keys(value).length === keys.length && keys.every((key) => key in value);
const record = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);
const text = (value: unknown): value is string => typeof value === "string" && value.length > 0;
const safeInteger = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const nullableText = (value: unknown): value is string | null => value === null || text(value);
function fail(): never { throw new Error("invalid proxy routing response"); }
const arrayOf = <T>(value: unknown, parser: (item: unknown) => T): T[] => { if (!Array.isArray(value)) fail(); return value.map(parser); };

function parseFilter(value: unknown): NodeFilter {
  const keys = ["regions", "protocols", "includeKeywords", "excludeKeywords", "includeNodeIds", "excludeNodeIds"];
  if (!record(value) || !exact(value, keys)) fail();
  const strings = (entry: unknown) => arrayOf(entry, (item) => text(item) ? item : fail());
  const parsedProtocols = arrayOf(value.protocols, (item) => typeof item === "string" && protocols.has(item) ? item as ProxyProtocol : fail());
  return { regions: strings(value.regions), protocols: parsedProtocols, includeKeywords: strings(value.includeKeywords), excludeKeywords: strings(value.excludeKeywords), includeNodeIds: strings(value.includeNodeIds), excludeNodeIds: strings(value.excludeNodeIds) };
}
function parseSource(value: unknown): PoolSource { if (!record(value) || !exact(value, ["providerId", "filter"]) || !text(value.providerId)) fail(); return { providerId: value.providerId, filter: parseFilter(value.filter) }; }
function parseSelection(value: unknown): Selection {
  if (!record(value) || typeof value.type !== "string") fail();
  if (value.type === "manual" && exact(value, ["type", "selectedNodeId"]) && nullableText(value.selectedNodeId)) return value as Selection;
  if (value.type === "urlTest" && exact(value, ["type", "probeUrl", "intervalSeconds", "toleranceMs"]) && text(value.probeUrl) && safeInteger(value.intervalSeconds) && value.intervalSeconds > 0 && safeInteger(value.toleranceMs)) return value as Selection;
  return fail();
}
function parseMatcher(value: unknown): Matcher {
  if (!record(value) || !exact(value, ["type", "values"]) || typeof value.type !== "string") fail();
  if (["domain", "domainSuffix", "application", "ipCidr"].includes(value.type)) return { type: value.type as "domain", values: arrayOf(value.values, (item) => text(item) ? item : fail()) };
  if (value.type === "port") return { type: "port", values: arrayOf(value.values, (item) => safeInteger(item) && item >= 1 && item <= 65535 ? item : fail()) };
  if (value.type === "protocol") return { type: "protocol", values: arrayOf(value.values, (item) => item === "tcp" || item === "udp" ? item : fail()) };
  return fail();
}
function parseTarget(value: unknown, allowFollow: true): DefaultTarget;
function parseTarget(value: unknown, allowFollow: false): RouteTarget;
function parseTarget(value: unknown, allowFollow: boolean): DefaultTarget | RouteTarget {
  if (!record(value) || typeof value.type !== "string") fail();
  if (allowFollow && value.type === "followActiveSubscription" && exact(value, ["type"])) return { type: value.type };
  if ((value.type === "direct" || value.type === "block") && exact(value, ["type"])) return { type: value.type };
  if (value.type === "pool" && exact(value, ["type", "poolId"]) && text(value.poolId)) return { type: "pool", poolId: value.poolId };
  return fail();
}
function parseApplyState(value: unknown): ApplyState {
  if (!record(value) || typeof value.type !== "string") fail();
  if ((value.type === "applied" || value.type === "savedPendingApply") && exact(value, ["type"])) return value as ApplyState;
  if ((value.type === "applying" || value.type === "applyUnknown") && exact(value, ["type", "operationId"]) && text(value.operationId)) return value as ApplyState;
  if (value.type === "savedApplyFailed" && exact(value, ["type", "operationId", "error"]) && text(value.operationId) && ["configurationFailed", "stateChanged", "stopFailed", "startFailed", "recoveryRequired"].includes(String(value.error))) return value as ApplyState;
  return fail();
}
function parseProvider(value: unknown): ProviderSnapshot { if (!record(value) || !exact(value, ["id", "subscriptionId", "name"]) || !text(value.id) || !text(value.subscriptionId) || !text(value.name)) fail(); return value as ProviderSnapshot; }
function parseNode(value: unknown): NodeSnapshot { if (!record(value) || !exact(value, ["id", "providerId", "name", "protocol"]) || !text(value.id) || !text(value.providerId) || !text(value.name) || typeof value.protocol !== "string" || !protocols.has(value.protocol)) fail(); return value as NodeSnapshot; }
function parsePool(value: unknown): PoolSnapshot { if (!record(value) || !exact(value, ["id", "name", "kind", "enabled", "sources", "selection", "resolvedNodeIds"]) || !text(value.id) || !text(value.name) || (value.kind !== "implicitProvider" && value.kind !== "custom") || typeof value.enabled !== "boolean") fail(); return { id: value.id, name: value.name, kind: value.kind, enabled: value.enabled, sources: arrayOf(value.sources, parseSource), selection: parseSelection(value.selection), resolvedNodeIds: arrayOf(value.resolvedNodeIds, (item) => text(item) ? item : fail()) }; }
function parseRoute(value: unknown): RouteSnapshot { if (!record(value) || !exact(value, ["id", "name", "enabled", "priority", "matcher", "target"]) || !text(value.id) || !text(value.name) || typeof value.enabled !== "boolean" || !safeInteger(value.priority)) fail(); return { id: value.id, name: value.name, enabled: value.enabled, priority: value.priority, matcher: parseMatcher(value.matcher), target: parseTarget(value.target, false) }; }
function parseSelector(value: unknown): SelectorSnapshot { if (!record(value) || !exact(value, ["poolId", "desiredNodeId", "runtimeNodeId", "state"]) || !text(value.poolId) || !nullableText(value.desiredNodeId) || !nullableText(value.runtimeNodeId) || !["inSync", "savedOnly", "notApplied", "unknown"].includes(String(value.state))) fail(); return value as SelectorSnapshot; }

export function parseProxyRoutingSnapshot(value: unknown): ProxyRoutingSnapshot {
  const keys = ["revision", "desiredGeneration", "appliedGeneration", "runtimeState", "applyState", "activeSubscriptionId", "appliedSubscriptionId", "providers", "nodes", "defaultTarget", "pools", "routes", "selectors"];
  if (!record(value) || !exact(value, keys) || !safeInteger(value.revision) || !safeInteger(value.desiredGeneration) || !(value.appliedGeneration === null || safeInteger(value.appliedGeneration)) || !["stopped", "ready", "transitioning", "recoveryRequired"].includes(String(value.runtimeState)) || !nullableText(value.activeSubscriptionId) || !nullableText(value.appliedSubscriptionId)) fail();
  return { revision: value.revision, desiredGeneration: value.desiredGeneration, appliedGeneration: value.appliedGeneration, runtimeState: value.runtimeState as RuntimeState, applyState: parseApplyState(value.applyState), activeSubscriptionId: value.activeSubscriptionId, appliedSubscriptionId: value.appliedSubscriptionId, providers: arrayOf(value.providers, parseProvider), nodes: arrayOf(value.nodes, parseNode), defaultTarget: parseTarget(value.defaultTarget, true), pools: arrayOf(value.pools, parsePool), routes: arrayOf(value.routes, parseRoute), selectors: arrayOf(value.selectors, parseSelector) };
}
function parseOutcome(value: unknown): MutationOutcome {
  if (!record(value) || typeof value.type !== "string") fail();
  if (value.type === "saved" && exact(value, ["type"])) return value as MutationOutcome;
  if ((value.type === "selectorApplied") && exact(value, ["type", "poolId", "nodeId"]) && text(value.poolId) && text(value.nodeId)) return value as MutationOutcome;
  if (value.type === "selectorNotApplied" && exact(value, ["type", "poolId", "nodeId", "runtimeNodeId"]) && text(value.poolId) && text(value.nodeId) && text(value.runtimeNodeId)) return value as MutationOutcome;
  if (value.type === "selectorSavedOnly" && exact(value, ["type", "poolId", "nodeId", "reason"]) && text(value.poolId) && text(value.nodeId) && ["runtimeStopped", "runtimeNotReady", "notInAppliedArtifact", "dispatchUnavailable"].includes(String(value.reason))) return value as MutationOutcome;
  if (value.type === "selectorApplyUnknown" && exact(value, ["type", "poolId", "nodeId", "reason"]) && text(value.poolId) && text(value.nodeId) && ["readBackUnavailable", "unknownRuntimeNode", "instanceChanged", "superseded"].includes(String(value.reason))) return value as MutationOutcome;
  if ((value.type === "applyStarted" || value.type === "applyCompleted") && exact(value, ["type", "operationId"]) && text(value.operationId)) return value as MutationOutcome;
  return fail();
}
export function parseSnapshotResult(value: unknown): SnapshotResult { if (!record(value) || typeof value.status !== "string") fail(); if (value.status === "ok" && exact(value, ["status", "snapshot"])) return { status: "ok", snapshot: parseProxyRoutingSnapshot(value.snapshot) }; if (value.status === "error" && exact(value, ["status", "error"]) && queryErrors.includes(value.error as QueryError)) return value as SnapshotResult; return fail(); }
export function parseMutationResult(value: unknown): MutationResult { if (!record(value) || typeof value.status !== "string") fail(); if (value.status === "ok" && exact(value, ["status", "outcome", "snapshot"])) return { status: "ok", outcome: parseOutcome(value.outcome), snapshot: parseProxyRoutingSnapshot(value.snapshot) }; if (value.status === "error" && exact(value, ["status", "error", "revision"]) && mutationErrors.includes(value.error as MutationError) && (value.revision === null || safeInteger(value.revision))) return value as MutationResult; return fail(); }
export async function getProxyRoutingSnapshot(): Promise<SnapshotResult> { return parseSnapshotResult(await invoke<unknown>("get_proxy_routing_snapshot")); }
const encodeFilter = (filter: NodeFilter): NodeFilter => ({ regions: [...filter.regions], protocols: [...filter.protocols], includeKeywords: [...filter.includeKeywords], excludeKeywords: [...filter.excludeKeywords], includeNodeIds: [...filter.includeNodeIds], excludeNodeIds: [...filter.excludeNodeIds] });
const encodeSources = (sources: PoolSource[]) => sources.map((source) => ({ providerId: source.providerId, filter: encodeFilter(source.filter) }));
const encodeSelection = (selection: Selection): Selection => selection.type === "manual" ? { type: "manual", selectedNodeId: selection.selectedNodeId } : { type: "urlTest", probeUrl: selection.probeUrl, intervalSeconds: selection.intervalSeconds, toleranceMs: selection.toleranceMs };
const encodeMatcher = (matcher: Matcher): Matcher => matcher.type === "port" ? { type: "port", values: [...matcher.values] } : matcher.type === "protocol" ? { type: "protocol", values: [...matcher.values] } : { type: matcher.type, values: [...matcher.values] };
const encodeDefaultTarget = (target: DefaultTarget): DefaultTarget => target.type === "pool" ? { type: "pool", poolId: target.poolId } : { type: target.type };
const encodeRouteTarget = (target: RouteTarget): RouteTarget => target.type === "pool" ? { type: "pool", poolId: target.poolId } : { type: target.type };
export function encodeProxyRoutingMutation(mutation: ProxyRoutingMutation): ProxyRoutingMutation {
  switch (mutation.type) {
    case "createCustomPool": return { type: mutation.type, name: mutation.name, enabled: mutation.enabled, sources: encodeSources(mutation.sources), selection: encodeSelection(mutation.selection) };
    case "updateCustomPool": return { type: mutation.type, id: mutation.id, name: mutation.name, enabled: mutation.enabled, sources: encodeSources(mutation.sources), selection: encodeSelection(mutation.selection) };
    case "deleteCustomPool": return { type: mutation.type, id: mutation.id };
    case "setManualSelection": return { type: mutation.type, poolId: mutation.poolId, nodeId: mutation.nodeId };
    case "setDefaultTarget": return { type: mutation.type, target: encodeDefaultTarget(mutation.target) };
    case "createRoute": return { type: mutation.type, name: mutation.name, enabled: mutation.enabled, matcher: encodeMatcher(mutation.matcher), target: encodeRouteTarget(mutation.target), insertAt: mutation.insertAt };
    case "updateRoute": return { type: mutation.type, id: mutation.id, name: mutation.name, enabled: mutation.enabled, matcher: encodeMatcher(mutation.matcher), target: encodeRouteTarget(mutation.target) };
    case "deleteRoute": return { type: mutation.type, id: mutation.id };
    case "reorderRoutes": return { type: mutation.type, routeIds: [...mutation.routeIds] };
    case "applyConfiguration": return { type: mutation.type };
  }
}
export async function mutateProxyRouting(expectedRevision: number, mutation: ProxyRoutingMutation): Promise<MutationResult> { return parseMutationResult(await invoke<unknown>("mutate_proxy_routing", { request: { expectedRevision, mutation: encodeProxyRoutingMutation(mutation) } })); }
export const shouldRefreshForRevision = (currentRevision: number, revision: number | null) => revision === null || revision !== currentRevision;
export const shouldRefreshAfterMutationError = (mutation: ProxyRoutingMutation, currentRevision: number, revision: number | null) => mutation.type === "applyConfiguration" || shouldRefreshForRevision(currentRevision, revision);
export const shouldPollProxyRouting = (snapshot: ProxyRoutingSnapshot | null) => snapshot?.applyState.type === "applying" || snapshot?.applyState.type === "applyUnknown";
export const runtimeRoutingInvalidationKey = (observation: RuntimeObservation) => [
  observation.sidecarLifecycle,
  observation.appliedSubscriptionId ?? "",
  observation.appliedConfigurationGeneration ?? "",
  observation.subscriptionSwitch?.operationId ?? "",
  observation.subscriptionSwitch?.status ?? "",
  observation.subscriptionSwitch?.errorCode ?? "",
].join("|");
export const shouldDismissDialog = (key: string, busy: boolean) => key === "Escape" && !busy;
export function wrappedDialogFocusIndex(currentIndex: number, controlCount: number, backwards: boolean): number | null {
  if (controlCount <= 0) return null;
  if (backwards && currentIndex <= 0) return controlCount - 1;
  if (!backwards && (currentIndex < 0 || currentIndex >= controlCount - 1)) return 0;
  return null;
}

export class ProxyRoutingResponseOrder {
  private requestEpoch = 0;
  private acceptedRevision = -1;
  beginQuery(): number { this.requestEpoch += 1; return this.requestEpoch; }
  invalidateQueries(): void { this.requestEpoch += 1; }
  isCurrent(epoch: number): boolean { return epoch === this.requestEpoch; }
  acceptQuery(epoch: number, snapshot: ProxyRoutingSnapshot): boolean {
    if (!this.isCurrent(epoch) || snapshot.revision < this.acceptedRevision) return false;
    this.acceptedRevision = snapshot.revision;
    return true;
  }
  acceptMutation(snapshot: ProxyRoutingSnapshot): void {
    this.invalidateQueries();
    this.acceptedRevision = Math.max(this.acceptedRevision, snapshot.revision);
  }
}

type Notice = { kind: "success" | "error" | "warning"; message: string };
type ContextValue = { snapshot: ProxyRoutingSnapshot | null; loading: boolean; error: QueryError | "invalidResponse" | null; pending: boolean; notice: Notice | null; refresh(): Promise<void>; mutate(mutation: ProxyRoutingMutation): Promise<MutationResult | null>; dismissNotice(): void };
const ProxyRoutingContext = createContext<ContextValue | null>(null);
const outcomeMessages: Record<MutationOutcome["type"], string> = { saved: "更改已保存，等待应用", selectorApplied: "节点已切换并经运行时确认", selectorNotApplied: "选择已保存，运行时仍使用原节点", selectorSavedOnly: "选择已保存，将在下次完整应用时生效", selectorApplyUnknown: "选择已保存，运行时结果暂时无法确认", applyStarted: "正在应用配置", applyCompleted: "配置已应用" };
export function ProxyRoutingProvider({ children }: { children: ReactNode }) {
  const [snapshot, setSnapshot] = useState<ProxyRoutingSnapshot | null>(null); const [loading, setLoading] = useState(true); const [error, setError] = useState<ContextValue["error"]>(null); const [pending, setPending] = useState(false); const pendingRef = useRef(false); const [notice, setNotice] = useState<Notice | null>(null); const snapshotRef = useRef(snapshot); snapshotRef.current = snapshot;
  const mountedRef = useRef(true); const queryInFlightRef = useRef(false); const queryQueuedRef = useRef(false); const responseOrderRef = useRef(new ProxyRoutingResponseOrder());
  const refresh = useCallback(async () => {
    if (queryInFlightRef.current) { queryQueuedRef.current = true; responseOrderRef.current.invalidateQueries(); return; }
    queryInFlightRef.current = true; if (mountedRef.current) setLoading(true);
    try {
      do {
        queryQueuedRef.current = false;
        const epoch = responseOrderRef.current.beginQuery();
        try {
          const result = await getProxyRoutingSnapshot();
          if (!mountedRef.current || !responseOrderRef.current.isCurrent(epoch)) continue;
          if (result.status === "ok") {
            if (responseOrderRef.current.acceptQuery(epoch, result.snapshot)) { setSnapshot(result.snapshot); setError(null); }
          } else setError(result.error);
        } catch { if (mountedRef.current && responseOrderRef.current.isCurrent(epoch)) setError("invalidResponse"); }
      } while (mountedRef.current && queryQueuedRef.current);
    } finally { queryInFlightRef.current = false; if (mountedRef.current) setLoading(false); }
  }, []);
  useEffect(() => { mountedRef.current = true; void refresh(); return () => { mountedRef.current = false; responseOrderRef.current.invalidateQueries(); }; }, [refresh]);
  useEffect(() => { let stop: (() => void) | undefined; let active = true; void subscribeSubscriptionStateChanged(() => { if (active) void refresh(); }).then((unlisten) => { if (active) stop = unlisten; else unlisten(); }).catch(() => undefined); return () => { active = false; stop?.(); }; }, [refresh]);
  useEffect(() => {
    let stop: (() => void) | undefined; let active = true; let previousKey: string | null = null;
    void subscribeRuntimeObservationDelta((observation) => {
      const nextKey = runtimeRoutingInvalidationKey(observation);
      if (!active || nextKey === previousKey) return;
      previousKey = nextKey;
      void refresh();
    }).then((unlisten) => { if (active) stop = unlisten; else unlisten(); }).catch(() => undefined);
    return () => { active = false; stop?.(); };
  }, [refresh]);
  const pollOperationId = snapshot?.applyState.type === "applying" || snapshot?.applyState.type === "applyUnknown" ? snapshot.applyState.operationId : null;
  useEffect(() => {
    if (!shouldPollProxyRouting(snapshot)) return;
    let stopped = false; let timer: number | undefined;
    const poll = async () => { await refresh(); if (!stopped) timer = window.setTimeout(() => void poll(), 800); };
    timer = window.setTimeout(() => void poll(), 800);
    return () => { stopped = true; if (timer !== undefined) window.clearTimeout(timer); };
  }, [pollOperationId, refresh]);
  const mutate = useCallback(async (mutation: ProxyRoutingMutation) => { const current = snapshotRef.current; if (!current || pendingRef.current) return null; pendingRef.current = true; setPending(true); setNotice(null); try { const result = await mutateProxyRouting(current.revision, mutation); if (result.status === "ok") { responseOrderRef.current.acceptMutation(result.snapshot); setSnapshot(result.snapshot); setError(null); setNotice({ kind: result.outcome.type === "selectorApplyUnknown" || result.outcome.type === "selectorNotApplied" ? "warning" : "success", message: outcomeMessages[result.outcome.type] }); } else { setNotice({ kind: "error", message: mutationErrorMessage(result.error) }); if (shouldRefreshAfterMutationError(mutation, current.revision, result.revision)) void refresh(); } return result; } catch { setNotice({ kind: "error", message: "响应格式不可用，请刷新后重试" }); void refresh(); return null; } finally { pendingRef.current = false; if (mountedRef.current) setPending(false); } }, [refresh]);
  const value = useMemo<ContextValue>(() => ({ snapshot, loading, error, pending, notice, refresh, mutate, dismissNotice: () => setNotice(null) }), [snapshot, loading, error, pending, notice, refresh, mutate]);
  return createElement(ProxyRoutingContext.Provider, { value }, children);
}
export function useProxyRouting(): ContextValue { const value = useContext(ProxyRoutingContext); if (!value) throw new Error("ProxyRoutingProvider is required"); return value; }
export function mutationErrorMessage(error: MutationError | QueryError | "invalidResponse") { const messages: Record<string, string> = { invalidInput: "输入无效，请检查后重试", busy: "另一项操作正在进行，请稍后重试", conflict: "页面数据已更新，请基于最新状态重试", notFound: "目标已不存在，列表已刷新", referenceConflict: "该出口组仍被默认出口或分流引用", validationFailed: "更改未通过完整配置校验", saveFailed: "保存失败，原有配置未改变", stateUnavailable: "配置状态不可用", runtimeUnavailable: "运行状态不可用", configurationFailed: "配置检查失败，旧运行配置保持不变", stateChanged: "应用前状态已变化，请重新确认", stopFailed: "旧内核停止未确认，需要恢复", startFailed: "新内核启动失败，当前已停止", recoveryRequired: "运行状态需要恢复", invalidResponse: "响应格式不可用" }; return messages[error]; }
