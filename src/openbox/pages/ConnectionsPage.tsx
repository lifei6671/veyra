import { Fragment, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent, type PointerEvent as ReactPointerEvent } from "react";
import { createPortal } from "react-dom";
import {
  ArrowRightCircleIcon,
  Bars2Icon,
  ChevronDownIcon,
  ChevronUpIcon,
  CircleStackIcon,
  LinkIcon,
  LinkSlashIcon,
  MagnifyingGlassMinusIcon,
  MagnifyingGlassPlusIcon,
  MapPinIcon,
  PauseIcon,
  PlayIcon,
  PlusIcon,
  SparklesIcon,
  TagIcon,
  TrashIcon,
  WrenchScrewdriverIcon,
  XMarkIcon,
} from "@heroicons/react/24/outline";
import { api } from "../api/client";
import type { ConnectionsFrame, ControllerConnection, GeoIpInfo, ProxiesResponse, StorageResponse } from "../api/types";
import { formatBytes, formatRate, formatRelativeTime, matchesQuery } from "../lib/format";
import { EmptyState, IconButton, Modal } from "../components/shared";
import { SearchInputGroup, SegmentedGroup, SegmentedItem, SelectControl, SwitchControl } from "../ui/controls";
import groupGlobalIcon from "../assets/group-global.svg";
import groupOtherIcon from "../assets/group-other.svg";
import optionAutoIcon from "../assets/option-auto.svg";
import optionDirectIcon from "../assets/option-direct.svg";
import { mergeProxyGroupLatencyResult, mergeProxyLatency, ProxyGroupCard, type LatencyThresholds } from "./ProxiesPage";

export type ConnectionColumn = "close" | "type" | "process" | "host" | "rule" | "chains" | "outbound" | "dlSpeed" | "ulSpeed" | "dl" | "ul" | "connectTime" | "sourceIP" | "sourcePort" | "sniffHost" | "destination" | "destinationType" | "remoteAddress" | "inboundUser";
type SortType = Exclude<ConnectionColumn, "close">;
type SourceLabel = { cidr: string; name: string };
type ColumnList = "active" | "inactive";
type DraggedColumn = {
  column: ConnectionColumn;
  x: number;
  y: number;
  offsetX: number;
  offsetY: number;
  width: number;
  height: number;
};
type DraggedLabel = {
  index: number;
  x: number;
  y: number;
  offsetX: number;
  offsetY: number;
  width: number;
  height: number;
};

export const defaultConnectionColumns: ConnectionColumn[] = ["close", "sourceIP", "chains", "host", "dlSpeed", "ulSpeed", "dl", "ul", "connectTime"];

export const groupableConnectionColumns: ConnectionColumn[] = [
  "host", "sourceIP", "outbound", "process", "rule", "sourcePort", "chains", "sniffHost", "destination", "destinationType", "remoteAddress", "inboundUser", "type",
];

const columnLabels: Record<ConnectionColumn, string> = {
  close: "关闭", type: "类型", process: "进程", host: "主机", rule: "规则", chains: "代理链", outbound: "出站节点",
  dlSpeed: "进站速率", ulSpeed: "出站速率", dl: "进站", ul: "出站", connectTime: "连接时间", sourceIP: "源IP",
  sourcePort: "源端口", sniffHost: "嗅探主机", destination: "解析地址", destinationType: "解析类型", remoteAddress: "远端地址", inboundUser: "入站用户",
};

const allColumns = Object.keys(columnLabels) as ConnectionColumn[];

const defaultWidths: Record<ConnectionColumn, number> = {
  close: 50, host: 320, chains: 320, rule: 200, dl: 80, dlSpeed: 80, ul: 80, ulSpeed: 80, outbound: 80,
  type: 150, process: 150, sourceIP: 150, sourcePort: 100, sniffHost: 200, destination: 150, connectTime: 100,
  destinationType: 120, remoteAddress: 170, inboundUser: 130,
};

function parseJson<T>(value: string | undefined, fallback: T): T {
  if (!value) return fallback;
  try { return JSON.parse(value) as T; } catch { return fallback; }
}

function parseSourceLabels(value: string | undefined): SourceLabel[] {
  const parsed = parseJson<unknown>(value, []);
  if (!Array.isArray(parsed)) return [];
  return parsed.filter((item): item is SourceLabel => Boolean(item) && typeof item === "object" && typeof (item as SourceLabel).cidr === "string" && typeof (item as SourceLabel).name === "string");
}

export function parseConnectionColumns(value: string | undefined): ConnectionColumn[] {
  const parsed = parseJson<unknown>(value, defaultConnectionColumns);
  if (!Array.isArray(parsed)) return defaultConnectionColumns;
  const result = parsed.filter((column): column is ConnectionColumn => typeof column === "string" && allColumns.includes(column as ConnectionColumn));
  return result.length ? [...new Set(result)] : defaultConnectionColumns;
}

export function parseConnectionGrouping(value: string | undefined): ConnectionColumn[] {
  const parsed = parseJson<unknown>(value, []);
  if (!Array.isArray(parsed)) return [];
  return [...new Set(parsed.filter((column): column is ConnectionColumn => typeof column === "string" && groupableConnectionColumns.includes(column as ConnectionColumn)))];
}

export function connectionColumnsForView(columns: ConnectionColumn[], tab: "active" | "closed", groupColumns: ConnectionColumn[]) {
  const visible = columns.filter(column => tab === "active" || column !== "close");
  const activeGroups = new Set(groupColumns.filter(column => visible.includes(column)));
  if (!activeGroups.size) return visible;
  return [
    ...visible.filter(column => activeGroups.has(column)),
    ...visible.filter(column => column !== "close" && !activeGroups.has(column)),
  ];
}

export function isDnsHijackConnection(connection: ControllerConnection) {
  const { destinationIP, destinationPort, host } = connection.metadata;
  return !host && destinationIP === "127.0.0.1" && String(destinationPort) === "53";
}

function chainIcon(name: string) {
  if (name === "直连") return optionDirectIcon;
  if (name.includes("自动")) return optionAutoIcon;
  if (name === "其他") return groupOtherIcon;
  return groupGlobalIcon;
}

export function connectionPolicyName(connection: ControllerConnection) {
  return [...connection.chains].reverse().find(Boolean) ?? "";
}

function countryName(info: GeoIpInfo) {
  if (info.countryCode) {
    try { return new Intl.DisplayNames(["zh-CN"], { type: "region" }).of(info.countryCode) ?? info.country; }
    catch { return info.country; }
  }
  return info.country;
}

function JsonPrimitive({ value, path }: { value: unknown; path: string[] }) {
  if (typeof value === "string") return <span className="connection-json-string">
    {path[0] === "chains" && <img src={chainIcon(value)} alt="" />}
    {JSON.stringify(value)}
  </span>;
  if (typeof value === "number") return <span className="connection-json-number">{value}</span>;
  if (typeof value === "boolean") return <span className="connection-json-boolean">{String(value)}</span>;
  return <span className="connection-json-null">null</span>;
}

function JsonChildren({ value, depth, path }: { value: unknown[] | Record<string, unknown>; depth: number; path: string[] }) {
  if (Array.isArray(value)) return <>{value.map((item, index) => <JsonProperty
    key={`${path.join(".")}-${index}`}
    value={item}
    depth={depth}
    path={[...path, String(index)]}
    trailing={index < value.length - 1}
  />)}</>;
  const entries = Object.entries(value);
  return <>{entries.map(([key, item], index) => <JsonProperty
    key={`${path.join(".")}-${key}`}
    name={key}
    value={item}
    depth={depth}
    path={[...path, key]}
    trailing={index < entries.length - 1}
  />)}</>;
}

function JsonProperty({ name, value, depth, path, trailing }: { name?: string; value: unknown; depth: number; path: string[]; trailing: boolean }) {
  const complex = Array.isArray(value) || (Boolean(value) && typeof value === "object");
  const prefix = name === undefined ? null : <><span className="connection-json-key">{JSON.stringify(name)}</span><span>: </span></>;
  if (!complex) return <div className="connection-json-line" style={{ paddingLeft: depth * 18 }}>{prefix}<JsonPrimitive value={value} path={path} />{trailing && ","}</div>;
  const collection = value as unknown[] | Record<string, unknown>;
  const array = Array.isArray(collection);
  return <>
    <div className="connection-json-line" style={{ paddingLeft: depth * 18 }}>{prefix}{array ? "[" : "{"}</div>
    <JsonChildren value={collection} depth={depth + 1} path={path} />
    <div className="connection-json-line" style={{ paddingLeft: depth * 18 }}>{array ? "]" : "}"}{trailing && ","}</div>
  </>;
}

function ConnectionJson({ connection }: { connection: ControllerConnection }) {
  return <div className="connection-json" aria-label="连接原始数据">
    <div className="connection-json-line">{"{"}</div>
    <JsonChildren value={connection as unknown as Record<string, unknown>} depth={1} path={[]} />
    <div className="connection-json-line">{"}"}</div>
  </div>;
}

function sourceLabel(ip: string, labels: SourceLabel[]) {
  for (const item of labels) {
    if (!item.cidr || !item.name) continue;
    if (item.cidr === ip) return item.name;
    if (item.cidr.startsWith("/") && item.cidr.endsWith("/")) {
      try { if (new RegExp(item.cidr.slice(1, -1)).test(ip)) return item.name; } catch { /* invalid labels are ignored */ }
    }
    if (matchesIpv4Cidr(ip, item.cidr)) return item.name;
  }
  return ip;
}

function matchesIpv4Cidr(ip: string, cidr: string) {
  const [network, prefixText] = cidr.split("/");
  if (!network || prefixText === undefined) return false;
  const prefix = Number(prefixText);
  const toNumber = (value: string) => {
    const parts = value.split(".").map(Number);
    if (parts.length !== 4 || parts.some(part => !Number.isInteger(part) || part < 0 || part > 255)) return null;
    return parts.reduce((result, part) => (result * 256 + part) >>> 0, 0);
  };
  const address = toNumber(ip);
  const subnet = toNumber(network);
  if (address === null || subnet === null || !Number.isInteger(prefix) || prefix < 0 || prefix > 32) return false;
  const mask = prefix === 0 ? 0 : (0xffffffff << (32 - prefix)) >>> 0;
  return (address & mask) === (subnet & mask);
}

function sortValue(connection: ControllerConnection, type: SortType, labels: SourceLabel[]): string | number {
  const meta = connection.metadata;
  switch (type) {
    case "sourceIP": return sourceLabel(meta.sourceIP, labels);
    case "chains": return [...connection.chains].reverse().join(" ");
    case "host": return meta.host || meta.destinationIP;
    case "type": return `${meta.type} ${meta.network}`;
    case "process": return meta.process || meta.processPath || "";
    case "rule": return `${connection.rule} ${connection.rulePayload}`;
    case "outbound": return connection.chains[0] ?? "";
    case "dlSpeed": return connection.downloadSpeed ?? 0;
    case "ulSpeed": return connection.uploadSpeed ?? 0;
    case "dl": return connection.download;
    case "ul": return connection.upload;
    case "connectTime": return new Date(connection.start).getTime();
    case "sourcePort": return Number(meta.sourcePort) || meta.sourcePort;
    case "sniffHost": return meta.sniffHost || "";
    case "destination": return meta.destinationIP;
    case "destinationType": return meta.dnsMode || "";
    case "remoteAddress": return `${meta.destinationIP}:${meta.destinationPort}`;
    case "inboundUser": return meta.inboundUser || "";
  }
}

export function sortConnections(connections: ControllerConnection[], type: SortType, direction: "asc" | "desc", labels: SourceLabel[] = []) {
  return [...connections].sort((left, right) => {
    const a = sortValue(left, type, labels);
    const b = sortValue(right, type, labels);
    const result = typeof a === "number" && typeof b === "number" ? a - b : String(a).localeCompare(String(b), "zh-CN", { numeric: true });
    return direction === "asc" ? result : -result;
  });
}

function connectionGroupValue(connection: ControllerConnection, column: ConnectionColumn) {
  const meta = connection.metadata;
  switch (column) {
    case "host": return `${meta.host || meta.destinationIP}:${meta.destinationPort}`;
    case "sourceIP": return meta.sourceIP;
    case "outbound": return connection.chains[0] || "—";
    case "process": return meta.process || meta.processPath || "—";
    case "rule": return connection.rulePayload ? `${connection.rule} · ${connection.rulePayload}` : connection.rule;
    case "sourcePort": return meta.sourcePort;
    case "chains": return [...connection.chains].reverse().join(" → ") || "—";
    case "sniffHost": return meta.sniffHost || "—";
    case "destination": return meta.destinationIP;
    case "destinationType": return destinationType(meta.destinationIP);
    case "remoteAddress": return `${meta.destinationIP}:${meta.destinationPort}`;
    case "inboundUser": return meta.inboundUser || "—";
    case "type": return `${meta.type} | ${meta.network}`;
    default: return "";
  }
}

function connectionGroupKeyValue(connection: ControllerConnection, column: ConnectionColumn) {
  if (column === "host") return `${connection.metadata.host}\0${connection.metadata.destinationPort}`;
  return connectionGroupValue(connection, column);
}

function destinationType(address: string) {
  if (/^\d{1,3}(?:\.\d{1,3}){3}$/.test(address)) return "IPv4";
  if (address.includes(":")) return "IPv6";
  return "FQDN";
}

export type ConnectionGroup = {
  key: string;
  values: Partial<Record<ConnectionColumn, string>>;
  count: number;
  connections: ControllerConnection[];
};

export function groupConnections(connections: ControllerConnection[], columns: ConnectionColumn[]): ConnectionGroup[] {
  const groups = new Map<string, ConnectionGroup>();
  for (const connection of connections) {
    const values = Object.fromEntries(columns.map(column => [column, connectionGroupValue(connection, column)])) as Partial<Record<ConnectionColumn, string>>;
    const key = JSON.stringify(columns.map(column => connectionGroupKeyValue(connection, column)));
    const existing = groups.get(key);
    if (existing) {
      existing.count += 1;
      existing.connections.push(connection);
    } else groups.set(key, { key, values, count: 1, connections: [connection] });
  }
  return [...groups.values()];
}

export function moveConnectionColumn(active: ConnectionColumn[], inactive: ConnectionColumn[], column: ConnectionColumn, target: ColumnList, index: number) {
  const next = {
    active: active.filter(item => item !== column),
    inactive: inactive.filter(item => item !== column),
  };
  const list = next[target];
  list.splice(Math.max(0, Math.min(index, list.length)), 0, column);
  return next;
}

export function appendSourceLabel(labels: SourceLabel[], cidr: string, name: string) {
  const next = { cidr: cidr.trim(), name: name.trim() };
  return next.cidr && next.name ? [...labels, next] : labels;
}

export function moveSourceLabel(labels: SourceLabel[], sourceIndex: number, targetIndex: number) {
  const next = [...labels];
  const [item] = next.splice(sourceIndex, 1);
  if (!item) return labels;
  next.splice(Math.max(0, Math.min(targetIndex, next.length)), 0, item);
  return next;
}

export function formatConnectionQuery(value: string) {
  const input = value.trim();
  if (!input) return "";
  const url = /^[a-z][a-z0-9+.-]*:\/\//i.test(input) ? input : input.startsWith("//") ? `http:${input}` : `http://${input}`;
  try { return new URL(url).hostname.toLowerCase() || input; }
  catch { return input; }
}

export function ConnectionsPage({ frame, storage, onPatchStorage, onToast }: {
  frame: ConnectionsFrame;
  storage: StorageResponse;
  onPatchStorage: (entries: Record<string, string>, removed?: string[]) => Promise<void>;
  onToast: (message: string) => void;
}) {
  const [paused, setPaused] = useState(false);
  const [snapshot, setSnapshot] = useState<ConnectionsFrame>(frame);
  const [tab, setTab] = useState<"active" | "closed">("active");
  const [query, setQuery] = useState("");
  const [sourceIP, setSourceIP] = useState("all");
  const [expandedGroupKeys, setExpandedGroupKeys] = useState<Set<string>>(() => new Set());
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [detailConnection, setDetailConnection] = useState<ControllerConnection | null>(null);
  const [detailProxies, setDetailProxies] = useState<ProxiesResponse | null>(null);
  const [detailGeo, setDetailGeo] = useState<GeoIpInfo | null>(null);
  const [detailError, setDetailError] = useState("");
  const [detailBusy, setDetailBusy] = useState("");
  const [detailTestingNodes, setDetailTestingNodes] = useState<Set<string>>(() => new Set());
  const detailTestingNodesRef = useRef(new Set<string>());
  const [detailPolicyCollapsed, setDetailPolicyCollapsed] = useState(true);
  const [detailPenetrationExpanded, setDetailPenetrationExpanded] = useState(false);

  useEffect(() => { if (!paused) setSnapshot(frame); }, [frame, paused]);
  useEffect(() => {
    setDetailConnection(current => current
      ? [...frame.connections, ...(frame.closedConnections ?? [])].find(item => item.id === current.id) ?? current
      : null);
  }, [frame]);

  const entries = storage.entries;
  const storedColumns = entries["config/connection-table-columns"];
  const storedSourceLabels = entries["config/source-ip-label-list"];
  const columns = useMemo(() => parseConnectionColumns(storedColumns), [storedColumns]);
  const groupColumns = useMemo(() => parseConnectionGrouping(entries["config/table-grouping"]), [entries]);
  const hideDns = entries["config/hide-dns-connections"] !== "false";
  const connectionsHidden = entries["config/quick-filter-enabled"] === "true";
  const configuredSort = entries["config/connection-sort-type"] as ConnectionColumn | undefined;
  const sortType: SortType = configuredSort && configuredSort !== "close" && allColumns.includes(configuredSort) ? configuredSort : "host";
  const sortDirection = entries["config/connection-sort-direction"] === "desc" ? "desc" : "asc";
  const sourceLabels = useMemo(() => parseSourceLabels(storedSourceLabels), [storedSourceLabels]);
  const widths = { ...defaultWidths, ...parseJson<Partial<Record<ConnectionColumn, number>>>(entries["config/table-column-width"], {}) };
  const testUrl = entries["config/speedtest-url"] ?? "http://www.gstatic.com/generate_204";
  const testTimeout = Number(entries["config/speedtest-timeout"] ?? 5000);
  const detailThresholds: LatencyThresholds = {
    low: Number(entries["config/low-latency"] ?? 500),
    medium: Number(entries["config/medium-latency"] ?? 1000),
  };
  const detailGroupName = detailConnection ? connectionPolicyName(detailConnection) : "";
  const detailIp = detailConnection?.metadata.destinationIP ?? "";
  const geoProvider = entries["config/geoip-info-api"] ?? "ip.sb";

  useEffect(() => {
    let current = true;
    setDetailError("");
    void api.proxies()
      .then(value => { if (current) setDetailProxies(value); })
      .catch(reason => { if (current) setDetailError(reason instanceof Error ? reason.message : "策略数据加载失败"); });
    return () => { current = false; };
  }, []);

  useEffect(() => {
    if (!detailConnection) return;
    let current = true;
    setDetailGeo(null);
    if (detailIp) void api.geoIp(detailIp, geoProvider)
      .then(value => { if (current) setDetailGeo(value); })
      .catch(() => undefined);
    return () => { current = false; };
  }, [detailConnection?.id, detailIp, geoProvider]);

  const active = snapshot.connections.filter(item => !hideDns || !isDnsHijackConnection(item));
  const closed = (snapshot.closedConnections ?? []).filter(item => !hideDns || !isDnsHijackConnection(item));
  const sourceIPs = useMemo(() => [...new Set([...active, ...closed].map(item => item.metadata.sourceIP))], [active, closed]);
  const source = tab === "active" ? active : closed;
  const effectiveGroupColumns = groupColumns.filter(column => columns.includes(column));
  const visibleColumns = connectionColumnsForView(columns, tab, effectiveGroupColumns);
  const visible = sortConnections(source.filter(item => (sourceIP === "all" || item.metadata.sourceIP === sourceIP) && matchesQuery([
    item.metadata.sourceIP, sourceLabel(item.metadata.sourceIP, sourceLabels), item.metadata.sourcePort, item.metadata.host,
    item.metadata.destinationIP, item.metadata.destinationPort, item.metadata.network, item.metadata.type, item.metadata.process,
    item.metadata.processPath, item.metadata.sniffHost, item.metadata.inboundUser, item.rule, item.rulePayload, ...item.chains,
  ], query)), sortType, sortDirection, sourceLabels);
  const connectionGroups = effectiveGroupColumns.length ? groupConnections(visible, effectiveGroupColumns) : [];
  const columnWidths = visibleColumns.map(column => Math.max(50, Number(widths[column]) || defaultWidths[column]));
  const gridTemplateColumns = columnWidths.map(width => `${width + 120}fr`).join(" ");
  const tableMinWidth = columnWidths.reduce((total, width) => total + width, 0);

  const persist = async (next: Record<string, string>, success?: string) => {
    try { await onPatchStorage(next); if (success) onToast(success); }
    catch (reason) { onToast(reason instanceof Error ? reason.message : "连接设置保存失败"); }
  };

  const closeOne = async (connection: ControllerConnection) => {
    setBusy(true);
    try {
      await api.closeConnection(connection.id);
      setSnapshot(current => ({ ...current, connections: current.connections.filter(item => item.id !== connection.id), closedConnections: [connection, ...(current.closedConnections ?? []).filter(item => item.id !== connection.id)].slice(0, 100) }));
      onToast("连接已关闭");
    } catch (reason) { onToast(reason instanceof Error ? reason.message : "关闭连接失败"); }
    finally { setBusy(false); }
  };

  const closeAll = async () => {
    setBusy(true);
    try {
      await api.closeAllConnections();
      setSnapshot(current => ({ ...current, connections: [], closedConnections: [...current.connections, ...(current.closedConnections ?? [])].slice(0, 100) }));
      onToast("全部连接已关闭");
    } catch (reason) { onToast(reason instanceof Error ? reason.message : "关闭全部连接失败"); }
    finally { setBusy(false); }
  };

  const changeSort = (type: SortType) => {
    const direction = sortType === type && sortDirection === "asc" ? "desc" : "asc";
    void persist({ "config/connection-sort-type": type, "config/connection-sort-direction": direction });
  };

  const toggleGrouping = (column: ConnectionColumn) => {
    setExpandedGroupKeys(new Set());
    const next = groupColumns.includes(column) ? groupColumns.filter(item => item !== column) : [...groupColumns, column];
    void persist({ "config/table-grouping": JSON.stringify(next) });
  };

  const toggleGroupExpanded = (key: string) => setExpandedGroupKeys(current => {
    const next = new Set(current);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    return next;
  });

  const openDetail = (connection: ControllerConnection) => {
    setDetailPolicyCollapsed(true);
    setDetailPenetrationExpanded(false);
    setDetailConnection(connection);
  };

  const selectDetailProxy = async (group: string, name: string) => {
    setDetailBusy(`${group}:${name}`);
    try {
      await api.selectProxy(group, name);
      setDetailProxies(current => current ? { proxies: { ...current.proxies, [group]: { ...current.proxies[group], now: name } } } : current);
      onToast(`${group} 已切换到 ${name}`);
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "切换失败");
    } finally {
      setDetailBusy("");
    }
  };

  const testDetailGroup = async (group: string) => {
    setDetailBusy(`test:${group}`);
    try {
      const result = await api.testProxyGroup(group, testUrl, testTimeout);
      setDetailProxies(current => current ? { proxies: mergeProxyGroupLatencyResult(current.proxies, group, result) } : current);
      onToast(`${group} 延迟测试完成`);
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "延迟测试失败");
    } finally {
      setDetailBusy("");
    }
  };

  const testDetailNode = async (name: string) => {
    if (detailTestingNodesRef.current.has(name)) return;
    detailTestingNodesRef.current.add(name);
    setDetailTestingNodes(new Set(detailTestingNodesRef.current));
    try {
      const { delay } = await api.testProxy(name, testUrl, testTimeout);
      setDetailProxies(current => current ? { proxies: mergeProxyLatency(current.proxies, name, delay) } : current);
    } catch {
      // The card keeps its previous latency when a probe fails.
    } finally {
      detailTestingNodesRef.current.delete(name);
      setDetailTestingNodes(new Set(detailTestingNodesRef.current));
    }
  };

  return <main className="page connections-page">
    <div className="route-controls connection-controls">
      <SegmentedGroup className="proxy-tabs connection-tabs" value={tab} onValueChange={value => setTab(value as "active" | "closed")} aria-label="连接状态">
        <SegmentedItem value="active">活跃{tab === "active" ? ` (${active.length})` : ""}</SegmentedItem>
        <SegmentedItem value="closed">已关闭{tab === "closed" ? ` (${closed.length})` : ""}</SegmentedItem>
      </SegmentedGroup>
      <SelectControl label="源IP" value={sourceIP} onValueChange={setSourceIP} options={[{ value: "all", label: "全部" }, ...sourceIPs.map(value => ({ value, label: sourceLabel(value, sourceLabels) }))]} />
      <SearchInputGroup label="搜索连接" value={query} onChange={setQuery} onClear={() => setQuery("")} placeholder="搜索 | 多个关键词用空格分隔" />
      <IconButton label="格式化查询" onClick={() => setQuery(formatConnectionQuery)}><SparklesIcon /></IconButton>
      <div className="route-tools">
        <IconButton label="连接设置" onClick={() => setSettingsOpen(true)}><WrenchScrewdriverIcon /></IconButton>
        <IconButton label={connectionsHidden ? "显示连接" : "隐藏连接"} onClick={() => void persist({ "config/quick-filter-enabled": String(!connectionsHidden) })}>
          {connectionsHidden ? <LinkSlashIcon /> : <LinkIcon />}
        </IconButton>
        <IconButton label={paused ? "继续刷新" : "暂停刷新"} active={paused} onClick={() => setPaused(value => !value)}>{paused ? <PlayIcon /> : <PauseIcon />}</IconButton>
        <IconButton label="关闭全部连接" disabled={busy || !active.length} onClick={() => void closeAll()}><XMarkIcon /></IconButton>
      </div>
    </div>

    <section className="connection-table-scroll">
      <div className="connection-table" role="table" style={{ "--connection-columns": gridTemplateColumns, minWidth: tableMinWidth } as CSSProperties}>
        <div className="connection-table-row connection-table-head" role="row">
          {visibleColumns.map(column => <ConnectionHeader key={column} column={column} activeSort={sortType} direction={sortDirection} grouped={groupColumns.includes(column)} onSort={changeSort} onToggleGrouping={toggleGrouping} />)}
        </div>
        {effectiveGroupColumns.length ? connectionGroups.map(group => <Fragment key={group.key}>
          <div
            className={`connection-table-row connection-table-group${expandedGroupKeys.has(group.key) ? " expanded" : ""}`}
            role="row"
            tabIndex={0}
            aria-expanded={expandedGroupKeys.has(group.key)}
            onClick={() => toggleGroupExpanded(group.key)}
            onKeyDown={event => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); toggleGroupExpanded(group.key); } }}
          >
            {visibleColumns.map(column => <ConnectionGroupCell key={column} column={column} group={group} labels={sourceLabels} />)}
          </div>
          {expandedGroupKeys.has(group.key) && group.connections.map(item => <div
            className="connection-table-row connection-table-child"
            role="row"
            tabIndex={0}
            key={item.id}
            onClick={() => openDetail(item)}
            onKeyDown={event => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); openDetail(item); } }}
          >
            {visibleColumns.map(column => effectiveGroupColumns.includes(column)
              ? <span role="cell" key={column} />
              : <ConnectionCell key={column} column={column} connection={item} label={sourceLabel(item.metadata.sourceIP, sourceLabels)} busy={busy} onClose={() => void closeOne(item)} />)}
          </div>)}
        </Fragment>) : visible.map(item => <div
          className="connection-table-row"
          role="row"
          tabIndex={0}
          key={item.id}
          onClick={() => openDetail(item)}
          onKeyDown={event => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); openDetail(item); } }}
        >
          {visibleColumns.map(column => <ConnectionCell key={column} column={column} connection={item} label={sourceLabel(item.metadata.sourceIP, sourceLabels)} busy={busy} onClose={() => void closeOne(item)} />)}
        </div>)}
      </div>
    </section>
    {!visible.length && <EmptyState icon={<LinkIcon />} title={tab === "closed" ? "暂无已关闭连接" : "当前没有匹配连接"} text={tab === "closed" ? "连接结束后会自动保留在这里。" : "调整筛选条件，或等待新的网络连接。"} compact />}

    {settingsOpen && <ConnectionSettings columns={columns} labels={sourceLabels} hideDns={hideDns} onPersist={persist} onClose={() => setSettingsOpen(false)} />}
    {detailConnection && <ConnectionDetail
      connection={detailConnection}
      data={detailProxies}
      geo={detailGeo}
      groupName={detailGroupName}
      error={detailError}
      busy={detailBusy}
      testingNodes={detailTestingNodes}
      collapsed={detailPolicyCollapsed}
      penetrationExpanded={detailPenetrationExpanded}
      thresholds={detailThresholds}
      hideUnavailable={entries["config/hide-unavailable-proxies"] === "true"}
      onClose={() => setDetailConnection(null)}
      onToggle={() => setDetailPolicyCollapsed(value => !value)}
      onTogglePenetration={() => setDetailPenetrationExpanded(value => !value)}
      onSelect={selectDetailProxy}
      onTest={testDetailGroup}
      onTestNode={testDetailNode}
    />}
  </main>;
}

function ConnectionDetail({ connection, data, geo, groupName, error, busy, testingNodes, collapsed, penetrationExpanded, thresholds, hideUnavailable, onClose, onToggle, onTogglePenetration, onSelect, onTest, onTestNode }: {
  connection: ControllerConnection;
  data: ProxiesResponse | null;
  geo: GeoIpInfo | null;
  groupName: string;
  error: string;
  busy: string;
  testingNodes: ReadonlySet<string>;
  collapsed: boolean;
  penetrationExpanded: boolean;
  thresholds: LatencyThresholds;
  hideUnavailable: boolean;
  onClose: () => void;
  onToggle: () => void;
  onTogglePenetration: () => void;
  onSelect: (group: string, name: string) => Promise<void>;
  onTest: (group: string) => Promise<void>;
  onTestNode: (name: string) => Promise<void>;
}) {
  const group = groupName ? data?.proxies[groupName] : undefined;
  const location = geo ? countryName(geo) : "";
  return <Modal title="连接详情" onClose={onClose} className="connection-detail-modal">
    <div className="connection-detail-layout">
      <section className="connection-detail-data">
        <div className="connection-detail-json-scroll"><ConnectionJson connection={connection} /></div>
        {geo && <div className="connection-detail-ip">
          <span><ArrowRightCircleIcon />{geo.ip}{geo.asn ? ` ( AS${geo.asn} )` : ""}</span>
          <span>{location && <><MapPinIcon />{location}</>}{geo.organization && <><CircleStackIcon />{geo.organization}</>}</span>
        </div>}
      </section>
      <div className="connection-detail-divider" aria-hidden="true" />
      <section className="connection-detail-policy" aria-label="对应策略">
        {group && <ProxyGroupCard
          group={group}
          data={data}
          collapsed={collapsed}
          busy={Boolean(busy)}
          onToggle={onToggle}
          onSelect={onSelect}
          onTest={onTest}
          onTestNode={onTestNode}
          variant="policy"
          hideUnavailable={hideUnavailable}
          thresholds={thresholds}
          history={group.history ?? []}
          latencyHistory={{}}
          testingGroup={busy.startsWith("test:") ? busy.slice(5) : ""}
          testingNodes={testingNodes}
          penetrationExpanded={penetrationExpanded}
          onTogglePenetration={onTogglePenetration}
        />}
        {!group && <div className={`connection-detail-policy-state${error ? " error" : ""}`}>
          {error || (data ? `未找到对应策略：${groupName || "无"}` : "正在读取策略…")}
        </div>}
      </section>
    </div>
  </Modal>;
}

function ConnectionHeader({ column, activeSort, direction, grouped, onSort, onToggleGrouping }: { column: ConnectionColumn; activeSort: SortType; direction: "asc" | "desc"; grouped: boolean; onSort: (type: SortType) => void; onToggleGrouping: (column: ConnectionColumn) => void }) {
  if (column === "close") return <span role="columnheader">{columnLabels[column]}</span>;
  const active = activeSort === column;
  return <span className="connection-header-cell" role="columnheader">
    <button type="button" className={`connection-sort-button${active ? " active" : ""}`} onClick={() => onSort(column)}>{columnLabels[column]}{active && (direction === "asc" ? <ChevronUpIcon /> : <ChevronDownIcon />)}</button>
    {groupableConnectionColumns.includes(column) && <button
      type="button"
      className={`connection-group-button${grouped ? " active" : ""}`}
      aria-label={grouped ? `取消按${columnLabels[column]}聚合` : `按${columnLabels[column]}聚合`}
      title={grouped ? `取消按${columnLabels[column]}聚合` : `按${columnLabels[column]}聚合`}
      onClick={() => onToggleGrouping(column)}
    >{grouped ? <MagnifyingGlassMinusIcon /> : <MagnifyingGlassPlusIcon />}</button>}
    {column === "host" && <MapPinIcon className="connection-pin" aria-label="固定主机列" />}
  </span>;
}

function ConnectionGroupCell({ column, group, labels }: { column: ConnectionColumn; group: ConnectionGroup; labels: SourceLabel[] }) {
  const value = group.values[column];
  if (value === undefined) return <span role="cell" />;
  const label = column === "sourceIP" ? sourceLabel(value, labels) : value;
  return <span role="cell" className="connection-group-cell" title={label}>
    <MagnifyingGlassMinusIcon />
    <span>{label}</span>
    <small>({group.count})</small>
  </span>;
}

function ConnectionCell({ column, connection, label, busy, onClose }: { column: ConnectionColumn; connection: ControllerConnection; label: string; busy: boolean; onClose: () => void }) {
  const meta = connection.metadata;
  const host = meta.host || meta.destinationIP;
  switch (column) {
    case "close": return <span role="cell"><button type="button" className="connection-close" aria-label={`关闭 ${host}`} title={`关闭 ${host}`} disabled={busy} onClick={event => { event.stopPropagation(); onClose(); }}><XMarkIcon /></button></span>;
    case "sourceIP": return <span role="cell" title={meta.sourceIP}>{label}</span>;
    case "chains": return <span role="cell" className="connection-chain" title={[...connection.chains].reverse().join(" → ")}>{[...connection.chains].reverse().map((name, index) => <span key={`${name}-${index}`}>{index > 0 && <ArrowRightCircleIcon className="connection-chain-arrow" />}<img src={chainIcon(name)} alt="" />{name}</span>)}</span>;
    case "host": return <span role="cell" title={`${host}:${meta.destinationPort}`}>{host}:{meta.destinationPort}</span>;
    case "dlSpeed": return <span role="cell">{formatRate(connection.downloadSpeed)}</span>;
    case "ulSpeed": return <span role="cell">{formatRate(connection.uploadSpeed)}</span>;
    case "dl": return <span role="cell">{formatBytes(connection.download)}</span>;
    case "ul": return <span role="cell">{formatBytes(connection.upload)}</span>;
    case "connectTime": return <time role="cell" dateTime={connection.start}>{formatRelativeTime(connection.start)}</time>;
    case "type": return <span role="cell">{meta.type} | {meta.network}</span>;
    case "process": return <span role="cell" title={meta.processPath}>{meta.process || meta.processPath || "—"}</span>;
    case "rule": return <span role="cell">{connection.rule}{connection.rulePayload ? ` · ${connection.rulePayload}` : ""}</span>;
    case "outbound": return <span role="cell">{connection.chains[0] || "—"}</span>;
    case "sourcePort": return <span role="cell">{meta.sourcePort}</span>;
    case "sniffHost": return <span role="cell">{meta.sniffHost || "—"}</span>;
    case "destination": return <span role="cell">{meta.destinationIP}</span>;
    case "destinationType": return <span role="cell">{destinationType(meta.destinationIP)}</span>;
    case "remoteAddress": return <span role="cell">{meta.destinationIP}:{meta.destinationPort}</span>;
    case "inboundUser": return <span role="cell">{meta.inboundUser || "—"}</span>;
  }
}

function ConnectionSettings({ columns, labels, hideDns, onPersist, onClose }: {
  columns: ConnectionColumn[];
  labels: SourceLabel[];
  hideDns: boolean;
  onPersist: (entries: Record<string, string>, success?: string) => Promise<void>;
  onClose: () => void;
}) {
  const [cidr, setCidr] = useState("");
  const [name, setName] = useState("");
  const [labelsExpanded, setLabelsExpanded] = useState(true);
  const [columnDraft, setColumnDraft] = useState(() => ({ active: columns, inactive: allColumns.filter(column => !columns.includes(column)) }));
  const [draggedColumn, setDraggedColumn] = useState<DraggedColumn | null>(null);
  const [columnDropTarget, setColumnDropTarget] = useState<{ list: ColumnList; index: number } | null>(null);
  const [draftLabels, setDraftLabels] = useState(labels);
  const [draggedLabel, setDraggedLabel] = useState<DraggedLabel | null>(null);
  const [labelDropIndex, setLabelDropIndex] = useState<number | null>(null);
  const columnDraftRef = useRef(columnDraft);
  const columnDropTargetRef = useRef(columnDropTarget);
  const draftLabelsRef = useRef(draftLabels);
  const labelDropIndexRef = useRef(labelDropIndex);
  const columnNodesRef = useRef(new Map<ConnectionColumn, HTMLButtonElement>());
  const previousColumnPositionsRef = useRef(new Map<ConnectionColumn, DOMRect>());
  const labelNodesRef = useRef(new Map<number, HTMLDivElement>());
  const previousLabelPositionsRef = useRef(new Map<number, DOMRect>());

  useEffect(() => {
    const next = { active: columns, inactive: allColumns.filter(column => !columns.includes(column)) };
    columnDraftRef.current = next;
    setColumnDraft(next);
  }, [columns]);
  useEffect(() => { draftLabelsRef.current = labels; setDraftLabels(labels); }, [labels]);

  useLayoutEffect(() => {
    const previous = previousColumnPositionsRef.current;
    previousColumnPositionsRef.current = new Map();
    previous.forEach((rect, column) => {
      if (column === draggedColumn?.column) return;
      const node = columnNodesRef.current.get(column);
      if (!node) return;
      const next = node.getBoundingClientRect();
      const x = rect.left - next.left;
      const y = rect.top - next.top;
      if (!x && !y) return;
      node.animate(
        [{ transform: `translate3d(${x}px, ${y}px, 0)` }, { transform: "translate3d(0, 0, 0)" }],
        { duration: 180, easing: "cubic-bezier(.2,.8,.2,1)" },
      );
    });
  }, [columnDropTarget, draggedColumn?.column]);

  useLayoutEffect(() => {
    const previous = previousLabelPositionsRef.current;
    previousLabelPositionsRef.current = new Map();
    previous.forEach((rect, index) => {
      if (index === draggedLabel?.index) return;
      const node = labelNodesRef.current.get(index);
      if (!node) return;
      const next = node.getBoundingClientRect();
      const x = rect.left - next.left;
      const y = rect.top - next.top;
      if (!x && !y) return;
      node.animate(
        [{ transform: `translate3d(${x}px, ${y}px, 0)` }, { transform: "translate3d(0, 0, 0)" }],
        { duration: 180, easing: "cubic-bezier(.2,.8,.2,1)" },
      );
    });
  }, [draggedLabel?.index, labelDropIndex]);

  const captureColumnPositions = () => {
    previousColumnPositionsRef.current = new Map(
      [...columnNodesRef.current].map(([column, node]) => [column, node.getBoundingClientRect()]),
    );
  };

  const captureLabelPositions = () => {
    previousLabelPositionsRef.current = new Map(
      [...labelNodesRef.current].map(([index, node]) => [index, node.getBoundingClientRect()]),
    );
  };

  const beginColumnDrag = (event: ReactPointerEvent<HTMLButtonElement>, column: ConnectionColumn, list: ColumnList) => {
    event.preventDefault();
    const picker = event.currentTarget.closest<HTMLElement>(".connection-column-picker");
    picker?.setPointerCapture(event.pointerId);
    const rect = event.currentTarget.getBoundingClientRect();
    captureColumnPositions();
    setDraggedColumn({
      column,
      x: event.clientX,
      y: event.clientY,
      offsetX: event.clientX - rect.left,
      offsetY: event.clientY - rect.top,
      width: rect.width,
      height: rect.height,
    });
    const nextTarget = { list, index: columnDraftRef.current[list].indexOf(column) };
    columnDropTargetRef.current = nextTarget;
    setColumnDropTarget(nextTarget);
  };
  const moveColumnFromPointer = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!draggedColumn) return;
    setDraggedColumn(current => current ? { ...current, x: event.clientX, y: event.clientY } : current);
    const target = document.elementFromPoint(event.clientX, event.clientY) as HTMLElement | null;
    const listElement = target?.closest<HTMLElement>("[data-column-list]");
    if (!listElement) return;
    const list = listElement.dataset.columnList as ColumnList;
    const items = [...listElement.querySelectorAll<HTMLElement>("[data-column]")];
    const index = items.findIndex(item => {
      const rect = item.getBoundingClientRect();
      return event.clientY < rect.top + rect.height / 2;
    });
    const nextTarget = { list, index: index === -1 ? items.length : index };
    if (columnDropTargetRef.current?.list === nextTarget.list && columnDropTargetRef.current.index === nextTarget.index) return;
    captureColumnPositions();
    columnDropTargetRef.current = nextTarget;
    setColumnDropTarget(nextTarget);
  };
  const dropColumn = (event: ReactPointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    const target = columnDropTargetRef.current;
    if (draggedColumn && target) {
      const next = moveConnectionColumn(columnDraftRef.current.active, columnDraftRef.current.inactive, draggedColumn.column, target.list, target.index);
      const changed = next.active.join("\0") !== columnDraftRef.current.active.join("\0") || next.inactive.join("\0") !== columnDraftRef.current.inactive.join("\0");
      columnDraftRef.current = next;
      setColumnDraft(next);
      if (changed) void onPersist({ "config/connection-table-columns": JSON.stringify(next.active) });
    }
    setDraggedColumn(null);
    columnDropTargetRef.current = null;
    setColumnDropTarget(null);
  };
  const cancelColumnDrag = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    setDraggedColumn(null);
    columnDropTargetRef.current = null;
    setColumnDropTarget(null);
  };

  const persistLabels = (next: SourceLabel[]) => void onPersist({ "config/source-ip-label-list": JSON.stringify(next) });
  const addLabel = () => {
    const next = appendSourceLabel(draftLabelsRef.current, cidr, name);
    if (next === draftLabelsRef.current) return;
    draftLabelsRef.current = next;
    setDraftLabels(next);
    persistLabels(next);
    setCidr(""); setName("");
  };
  const updateLabel = (index: number, key: keyof SourceLabel, value: string) => setDraftLabels(current => {
    const next = current.map((item, itemIndex) => itemIndex === index ? { ...item, [key]: value } : item);
    draftLabelsRef.current = next;
    return next;
  });
  const saveEditedLabels = () => persistLabels(draftLabelsRef.current.filter(item => item.cidr.trim() && item.name.trim()).map(item => ({ cidr: item.cidr.trim(), name: item.name.trim() })));
  const removeLabel = (index: number) => {
    const next = draftLabelsRef.current.filter((_, itemIndex) => itemIndex !== index);
    draftLabelsRef.current = next;
    setDraftLabels(next);
    persistLabels(next);
  };
  const beginLabelDrag = (event: ReactPointerEvent<HTMLButtonElement>, index: number) => {
    event.preventDefault();
    const container = event.currentTarget.closest<HTMLElement>(".source-labels");
    container?.setPointerCapture(event.pointerId);
    const row = event.currentTarget.closest<HTMLElement>(".source-label-row");
    if (!row) return;
    const rect = row.getBoundingClientRect();
    captureLabelPositions();
    setDraggedLabel({
      index,
      x: event.clientX,
      y: event.clientY,
      offsetX: event.clientX - rect.left,
      offsetY: event.clientY - rect.top,
      width: rect.width,
      height: rect.height,
    });
    labelDropIndexRef.current = index;
    setLabelDropIndex(index);
  };
  const moveLabelFromPointer = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!draggedLabel) return;
    setDraggedLabel(current => current ? { ...current, x: event.clientX, y: event.clientY } : current);
    const bounds = event.currentTarget.getBoundingClientRect();
    if (event.clientX < bounds.left || event.clientX > bounds.right || event.clientY < bounds.top || event.clientY > bounds.bottom) return;
    const items = [...event.currentTarget.querySelectorAll<HTMLElement>("[data-label-index]")];
    const index = items.findIndex(item => {
      const rect = item.getBoundingClientRect();
      return event.clientY < rect.top + rect.height / 2;
    });
    const nextIndex = index === -1 ? items.length : index;
    if (labelDropIndexRef.current === nextIndex) return;
    captureLabelPositions();
    labelDropIndexRef.current = nextIndex;
    setLabelDropIndex(nextIndex);
  };
  const dropLabel = (event: ReactPointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    const targetIndex = labelDropIndexRef.current;
    if (draggedLabel && targetIndex !== null) {
      const next = moveSourceLabel(draftLabelsRef.current, draggedLabel.index, targetIndex);
      const changed = JSON.stringify(next) !== JSON.stringify(draftLabelsRef.current);
      draftLabelsRef.current = next;
      setDraftLabels(next);
      if (changed) persistLabels(next);
    }
    setDraggedLabel(null);
    labelDropIndexRef.current = null;
    setLabelDropIndex(null);
  };
  const cancelLabelDrag = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    setDraggedLabel(null);
    labelDropIndexRef.current = null;
    setLabelDropIndex(null);
  };
  const submitOnEnter = (event: KeyboardEvent<HTMLInputElement>) => { if (event.key === "Enter") addLabel(); };

  const renderColumnList = (list: ColumnList) => {
    const items = columnDraft[list].filter(column => column !== draggedColumn?.column);
    const targetIndex = columnDropTarget?.list === list ? Math.min(columnDropTarget.index, items.length) : -1;
    return items.map((column, index) => <Fragment key={column}>
      {index === targetIndex && <div className="connection-column-placeholder">{columnLabels[draggedColumn!.column]}</div>}
      <button
        type="button"
        className={list === "inactive" ? "inactive" : ""}
        data-column={column}
        aria-label={`拖动${columnLabels[column]}`}
        ref={node => { if (node) columnNodesRef.current.set(column, node); else columnNodesRef.current.delete(column); }}
        onPointerDown={event => beginColumnDrag(event, column, list)}
      >{columnLabels[column]}</button>
    </Fragment>).concat(targetIndex === items.length && draggedColumn ? [<div className="connection-column-placeholder" key="drop-placeholder">{columnLabels[draggedColumn.column]}</div>] : []);
  };

  const labelEntries = draftLabels.map((item, index) => ({ item, index })).filter(({ index }) => index !== draggedLabel?.index);
  const targetLabelIndex = labelDropIndex === null ? -1 : Math.min(labelDropIndex, labelEntries.length);

  return <Modal title="连接设置" onClose={onClose} className="connection-settings-modal">
    <div className="connection-settings">
      <label className="connection-setting-switch"><span>隐藏 DNS 劫持连接</span><SwitchControl label="隐藏 DNS 劫持连接" checked={hideDns} onCheckedChange={checked => void onPersist({ "config/hide-dns-connections": String(checked) })} /></label>
      <p>终端的 DNS 查询被劫持进内核后交给 dnsmasq / dns-out 的那些 127.0.0.1:53 行：没有域名，每开一个 App 就刷几十行。藏掉不影响统计数。</p>
      <hr />
      <h3>自定义表格列</h3>
      <div className="connection-column-picker" onPointerMove={moveColumnFromPointer} onPointerUp={dropColumn} onPointerCancel={cancelColumnDrag}>
        {(["active", "inactive"] as const).map(list => <div
          className={`connection-column-list ${list}`}
          key={list}
          data-column-list={list}
        >{renderColumnList(list)}</div>)}
      </div>
      {draggedColumn && createPortal(<div
        className="connection-column-drag-preview"
        style={{
          width: draggedColumn.width,
          height: draggedColumn.height,
          transform: `translate3d(${draggedColumn.x - draggedColumn.offsetX}px, ${draggedColumn.y - draggedColumn.offsetY}px, 0) scale(1.025) rotate(.35deg)`,
        }}
      >{columnLabels[draggedColumn.column]}</div>, document.body)}
      <hr />
      <div className="source-label-heading"><h3>终端名称 ({draftLabels.length})</h3><button type="button" aria-label={labelsExpanded ? "收起终端名称" : "展开终端名称"} onClick={() => setLabelsExpanded(value => !value)}>{labelsExpanded ? <ChevronUpIcon /> : <ChevronDownIcon />}</button></div>
      {labelsExpanded && <div className="source-labels" onPointerMove={moveLabelFromPointer} onPointerUp={dropLabel} onPointerCancel={cancelLabelDrag}>
        {labelEntries.map(({ item, index }, visibleIndex) => <Fragment key={index}>
          {visibleIndex === targetLabelIndex && <div className="source-label-placeholder" />}
          <div className="source-label-row" data-label-index={index} ref={node => { if (node) labelNodesRef.current.set(index, node); else labelNodesRef.current.delete(index); }}>
            <button type="button" className="source-label-handle" aria-label={`拖动终端名称 ${item.name}`} onPointerDown={event => beginLabelDrag(event, index)}><Bars2Icon /></button>
            <input aria-label={`终端地址 ${index + 1}`} value={item.cidr} onChange={event => updateLabel(index, "cidr", event.target.value)} onBlur={saveEditedLabels} />
            <ArrowRightCircleIcon className="source-label-arrow" aria-hidden="true" />
            <input aria-label={`终端名称 ${index + 1}`} value={item.name} onChange={event => updateLabel(index, "name", event.target.value)} onBlur={saveEditedLabels} />
            <button type="button" className="source-label-action" aria-label={`删除终端名称 ${item.name}`} onClick={() => removeLabel(index)}><TrashIcon /></button>
          </div>
        </Fragment>)}
        {targetLabelIndex === labelEntries.length && draggedLabel && <div className="source-label-placeholder" />}
        <div className="source-label-row source-label-editor">
          <TagIcon className="source-label-tag" aria-hidden="true" />
          <input aria-label="新终端地址" value={cidr} onChange={event => setCidr(event.target.value)} onKeyDown={submitOnEnter} placeholder="IP/CIDR | eui64 | /Regex" />
          <ArrowRightCircleIcon className="source-label-arrow" aria-hidden="true" />
          <input aria-label="新终端名称" value={name} onChange={event => setName(event.target.value)} onKeyDown={submitOnEnter} placeholder="名称" />
          <button type="button" className="source-label-action" aria-label="添加终端名称" disabled={!cidr.trim() || !name.trim()} onClick={addLabel}><PlusIcon /></button>
        </div>
      </div>}
      {draggedLabel && createPortal(<div
        className="source-label-drag-preview"
        style={{
          width: draggedLabel.width,
          height: draggedLabel.height,
          transform: `translate3d(${draggedLabel.x - draggedLabel.offsetX}px, ${draggedLabel.y - draggedLabel.offsetY}px, 0) scale(1.012)`,
        }}
      >
        <Bars2Icon />
        <span>{draftLabels[draggedLabel.index]?.cidr}</span>
        <ArrowRightCircleIcon className="source-label-arrow" aria-hidden="true" />
        <span>{draftLabels[draggedLabel.index]?.name}</span>
        <TrashIcon />
      </div>, document.body)}
    </div>
  </Modal>;
}
