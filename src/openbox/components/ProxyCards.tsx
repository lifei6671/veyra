import { useId } from "react";
import { BoltIcon, ChevronDownIcon, ChevronUpIcon } from "@heroicons/react/24/outline";
import type { ControllerProxy, ProxiesResponse, ProxyHistory } from "../api/types";
import speedGroupIcon from "../assets/group-speed.svg";
import aiGroupIcon from "../assets/group-ai.svg";
import youtubeGroupIcon from "../assets/group-youtube.svg";
import tiktokGroupIcon from "../assets/group-tiktok.svg";
import netflixGroupIcon from "../assets/group-netflix.svg";
import githubGroupIcon from "../assets/group-github.svg";
import googleGroupIcon from "../assets/group-google.svg";
import microsoftGroupIcon from "../assets/group-microsoft.svg";
import appleGroupIcon from "../assets/group-apple.svg";
import gamesGroupIcon from "../assets/group-games.svg";
import globalGroupIcon from "../assets/group-global.svg";
import chinaGroupIcon from "../assets/group-china.svg";
import otherGroupIcon from "../assets/group-other.svg";
import directOptionIcon from "../assets/option-direct.svg";
import autoOptionIcon from "../assets/option-auto.svg";
import manualOptionIcon from "../assets/option-manual.svg";
import rejectOptionIcon from "../assets/option-reject.svg";
import { latestLatency, matchesQuery } from "../lib/format";

export type ProxyNodeSortMode = "default" | "nameAsc" | "nameDesc" | "latencyAsc" | "latencyDesc";
export type LatencyThresholds = { low: number; medium: number };

const groupIcons: Record<string, string> = {
  Speed: speedGroupIcon, AI: aiGroupIcon, Youtube: youtubeGroupIcon, TikTok: tiktokGroupIcon,
  Netflix: netflixGroupIcon, Github: githubGroupIcon, Google: googleGroupIcon, Microsoft: microsoftGroupIcon,
  Apple: appleGroupIcon, Games: gamesGroupIcon, "国外": globalGroupIcon, "国内": chinaGroupIcon, "其他": otherGroupIcon,
};

const optionIcons: Record<string, string> = {
  "直连": directOptionIcon,
  "所有-自动": autoOptionIcon,
  "所有-手动": manualOptionIcon,
  "拒绝": rejectOptionIcon,
};

export function proxyIcon(name: string) {
  return optionIcons[name] ?? groupIcons[name];
}

export function groupIcon(name: string) {
  if (name === "所有-自动") return autoOptionIcon;
  if (name === "所有-手动") return manualOptionIcon;
  return groupIcons[name] ?? otherGroupIcon;
}

export function ProxyGroupCard({ group, data, collapsed, busy, onToggle, onSelect, onTest, onTestNode, variant, hideUnavailable, thresholds, history, latencyHistory, testingGroup, testingNodes = EMPTY_TESTING_NODES, query = "", nodeSort = "latencyAsc", groupByProvider = true, penetrationExpanded = false, onTogglePenetration, embedded = false }: {
  group: ControllerProxy;
  data: ProxiesResponse | null;
  collapsed: boolean;
  busy: boolean;
  onToggle: () => void;
  onSelect: (group: string, name: string) => Promise<void>;
  onTest: (group: string) => Promise<void>;
  onTestNode: (name: string) => Promise<void>;
  variant: "policy" | "nodes";
  hideUnavailable: boolean;
  thresholds: LatencyThresholds;
  history: ProxyHistory[];
  latencyHistory: Record<string, ProxyHistory[]>;
  testingGroup: string;
  testingNodes?: ReadonlySet<string>;
  query?: string;
  nodeSort?: ProxyNodeSortMode;
  groupByProvider?: boolean;
  penetrationExpanded?: boolean;
  onTogglePenetration?: () => void;
  embedded?: boolean;
}) {
  const visibleOptions = ((group.all ?? []).map(name => data?.proxies[name]).filter(Boolean) as ControllerProxy[])
    .filter(option => isVisibleProxy(option, hideUnavailable));
  const allOptions = sortProxyNodes(visibleOptions, nodeSort);
  const options = allOptions.filter(option => proxyMatchesQuery(option, query));
  const tested = allOptions.filter(option => latestLatency(option.history)).length;
  const current = group.now ? data?.proxies[group.now] : undefined;
  const penetration = variant === "policy" ? getPenetrationGroup(group, data) : undefined;
  const penetrationOpen = Boolean(penetration && penetrationExpanded);
  const nodesCollapsed = variant === "nodes" && collapsed;
  const nodeClass = variant === "nodes" ? " node-group-card" : "";
  const collapsedClass = collapsed ? `${nodesCollapsed ? " node-group-collapsed" : ""} policy-card-collapsed` : "";
  const selectedDelay = selectedProxyDelay(group, data);
  const testing = testingGroup === "all" || testingGroup === group.name;
  const headerContent = <>
    <img className="policy-icon" src={groupIcon(group.name)} alt="" />
    <div className="policy-summary">
      <div><h2>{group.name}</h2>{variant === "policy" && <span>域名穿透</span>}<small>{group.type} ({tested}/{allOptions.length})</small></div>
      <p>{variant === "policy" && <img src={optionIcons[group.now ?? ""] ?? otherGroupIcon} alt="" />}{group.now ?? "未选择"}{current?.now && <><b>›</b>{current.now}</>}</p>
    </div>
  </>;
  return <article className={`${embedded ? "" : "surface "}policy-card${embedded ? " embedded" : ""}${nodeClass}${collapsedClass}`}>
    <div className="policy-head">
      {embedded
        ? <div className="node-group-header-toggle static">{headerContent}</div>
        : <button type="button" className="node-group-header-toggle" aria-label={`${collapsed ? "展开" : "收起"} ${group.name}`} aria-expanded={!collapsed} onClick={onToggle}>{headerContent}</button>}
      <PolicyLatency groupName={group.name} history={history.length ? history : current?.history ?? []} delay={selectedDelay} thresholds={thresholds} busy={busy} testing={testing} onTest={onTest} />
      {variant === "nodes" && group.name === "所有-自动" && <small className="group-meta">检测间隔 300 秒 · 容差 100 毫秒</small>}
    </div>
    <div className={`node-health-summary${collapsed ? " visible" : ""}`} aria-hidden={!collapsed}>
      <div className="node-health-summary-inner"><NodeHealthDots options={allOptions} selectedName={group.now} thresholds={thresholds} /></div>
    </div>
    {variant === "policy" && <section className={`policy-card-collapse${collapsed ? " collapsed" : ""}`} aria-hidden={collapsed} inert={collapsed || undefined}>
      <div className="policy-card-collapse-inner">
        <div className="policy-options">{options.map(option => <ProxyOption
          key={option.name}
          option={option}
          active={option.name === group.now}
          disabled={busy || testingNodes.has(option.name)}
          thresholds={thresholds}
          onClick={() => void onSelect(group.name, option.name)}
          onTest={() => void onTestNode(option.name)}
          testing={testingGroup === "all" || testingNodes.has(option.name)}
        />)}</div>
        {penetration && <>
          <button type="button" className={`policy-footer${penetrationOpen ? " open" : ""}`} aria-expanded={penetrationOpen} onClick={onTogglePenetration}>{penetrationOpen ? "收起穿透" : "策略穿透"}{penetrationOpen ? <ChevronUpIcon /> : <ChevronDownIcon />}</button>
          {penetrationOpen && <PenetrationGroup
            group={penetration}
            data={data}
            busy={busy}
            onSelect={onSelect}
            onTest={onTest}
            onTestNode={onTestNode}
            hideUnavailable={hideUnavailable}
            thresholds={thresholds}
            history={latencyHistory[penetration.name] ?? penetration.history ?? []}
            testing={testingGroup === "all" || testingGroup === penetration.name}
            testingNode={testingGroup}
            testingNodes={testingNodes}
            query={query}
            nodeSort={nodeSort}
          />}
        </>}
      </div>
    </section>}
    {variant === "nodes" && <section className={`node-provider-collapse${nodesCollapsed ? " collapsed" : ""}`} aria-hidden={nodesCollapsed} inert={nodesCollapsed || undefined}>
      <div className="node-provider-section">
        {groupByProvider && <div className="node-provider-head">
          <h3 className="node-provider-title">订阅</h3>
          <button type="button" className="penetration-test" aria-label={`测试 ${group.name} 全部节点`} aria-busy={testing || undefined} disabled={busy} onClick={() => void onTest(group.name)}><BoltIcon className={testing ? "testing-bolt" : undefined} /></button>
        </div>}
        <div className="policy-options node-provider-options">{options.map(option => <ProxyOption
          key={option.name}
          option={option}
          active={option.name === group.now}
          disabled={busy || testingNodes.has(option.name)}
          thresholds={thresholds}
          showIcon={false}
          onClick={proxyOptionAction(group.type) === "select" ? () => void onSelect(group.name, option.name) : undefined}
          onTest={() => void onTestNode(option.name)}
          testing={testingGroup === "all" || testingNodes.has(option.name)}
        />)}</div>
      </div>
    </section>}
  </article>;
}

export function NodeHealthDots({ options, selectedName, thresholds }: { options: ControllerProxy[]; selectedName?: string; thresholds: LatencyThresholds }) {
  const tested = options.filter(option => latestLatency(option.history) != null).length;
  return <div className="node-health-dots" aria-label={`节点状态：${tested} 已测速，${options.length - tested} 未测速`}>
    {options.map(option => {
      const delay = latestLatency(option.history);
      const selected = option.name === selectedName ? " selected" : "";
      return <span className={`${delay == null ? "untested" : latencyClass(delay, thresholds)}${selected}`} aria-hidden="true" key={option.name} />;
    })}
  </div>;
}

export function PenetrationGroup({ group, data, busy, onSelect, onTest, onTestNode, hideUnavailable, thresholds, history, testing, testingNode = "", testingNodes = EMPTY_TESTING_NODES, query = "", nodeSort = "latencyAsc" }: {
  group: ControllerProxy;
  data: ProxiesResponse | null;
  busy: boolean;
  onSelect: (group: string, name: string) => Promise<void>;
  onTest: (group: string) => Promise<void>;
  onTestNode: (name: string) => Promise<void>;
  hideUnavailable: boolean;
  thresholds: LatencyThresholds;
  history: ProxyHistory[];
  testing: boolean;
  testingNode?: string;
  testingNodes?: ReadonlySet<string>;
  query?: string;
  nodeSort?: ProxyNodeSortMode;
}) {
  const allOptions = sortProxyNodes(((group.all ?? []).map(name => data?.proxies[name]).filter(Boolean) as ControllerProxy[])
    .filter(option => isVisibleProxy(option, hideUnavailable)), nodeSort);
  const options = allOptions.filter(option => proxyMatchesQuery(option, query));
  const tested = allOptions.filter(option => latestLatency(option.history)).length;
  const selectedDelay = selectedProxyDelay(group, data);
  return <section className="policy-penetration" aria-label={`${group.name} 节点列表`}>
    <div className="penetration-head">
      <img className="penetration-icon" src={groupIcon(group.name)} alt="" />
      <div className="penetration-summary">
        <div><h3>{group.name}</h3><small>{group.type} ({tested}/{allOptions.length})</small></div>
        <p>{group.now ?? "未选择"}</p>
      </div>
      <PolicyLatency groupName={group.name} history={history} delay={selectedDelay} thresholds={thresholds} busy={busy} testing={testing} onTest={onTest} />
    </div>
    <div className="penetration-provider-head">
      <h4>订阅</h4>
      <button type="button" className="penetration-test" aria-label={`测试 ${group.name} 全部节点`} aria-busy={testing || undefined} disabled={busy} onClick={() => void onTest(group.name)}><BoltIcon className={testing ? "testing-bolt" : undefined} /></button>
    </div>
    <div className="policy-options penetration-options">{options.map(option => <ProxyOption
      key={option.name}
      option={option}
      active={option.name === group.now}
      disabled={busy || testingNodes.has(option.name)}
      thresholds={thresholds}
      showIcon={false}
      onClick={proxyOptionAction(group.type) === "select" ? () => void onSelect(group.name, option.name) : undefined}
      onTest={() => void onTestNode(option.name)}
      testing={testingNode === "all" || testingNodes.has(option.name)}
    />)}</div>
  </section>;
}

export function PolicyLatency({ groupName, history, delay, thresholds, busy, testing = false, onTest }: {
  groupName: string;
  history: ProxyHistory[];
  delay: number | null;
  thresholds: LatencyThresholds;
  busy: boolean;
  testing?: boolean;
  onTest: (group: string) => Promise<void>;
}) {
  const tooltipId = useId();
  const points = recentLatencyHistory(history);
  return <span className="policy-latency-wrap">
    <button
      type="button"
      className={`policy-latency${delay == null ? "" : ` ${latencyClass(delay, thresholds)}`}`}
      aria-label={`测试 ${groupName}`}
      aria-describedby={points.length ? tooltipId : undefined}
      aria-busy={testing || undefined}
      disabled={busy}
      onClick={event => { event.stopPropagation(); void onTest(groupName); }}
    >{testing ? <BoltIcon className="testing-bolt" /> : delay ?? <BoltIcon />}</button>
    {points.length > 0 && <span id={tooltipId} className="policy-history-tooltip" role="tooltip">
      <span className="policy-history-list">
        {points.map((point, index) => <span className="policy-history-item" key={`${point.time}-${point.node ?? groupName}-${index}`}>
          <span className="policy-history-meta">
            <small>{point.node ?? groupName}</small>
            <time dateTime={point.time}>{formatLatencyTime(point.time)}</time>
          </span>
          <span className={`policy-history-track${point.delay > 0 ? " success" : ""}${index === 0 ? " first" : ""}`} aria-hidden="true"><i /></span>
          <strong className={point.delay > 0 ? "success" : ""}>{point.delay > 0 ? `${point.delay}ms` : "超时"}</strong>
        </span>)}
      </span>
    </span>}
  </span>;
}

export function ProxyOption({ option, active, disabled, testDisabled = false, thresholds, showIcon = true, testing = false, onClick, onTest }: {
  option: ControllerProxy;
  active: boolean;
  disabled: boolean;
  testDisabled?: boolean;
  thresholds: LatencyThresholds;
  showIcon?: boolean;
  testing?: boolean;
  onClick?: () => void;
  onTest?: () => void;
}) {
  const delay = latestLatency(option.history);
  const content = <>
    <span className="option-title">{showIcon && <img src={optionIcons[option.name] ?? groupIcons[option.name] ?? otherGroupIcon} alt="" />}<b>{option.name}</b></span>
    <small>{proxyType(option.type)}{option.udp ? " / udp" : ""}</small>
  </>;
  if (onTest) return <div className={`policy-option split-action${active ? " active" : ""}${disabled ? " disabled" : ""}`}>
    {onClick
      ? <button type="button" className="policy-option-main" aria-label={`选择 ${option.name}`} disabled={disabled} onClick={onClick}>{content}</button>
      : <div className="policy-option-main static" aria-disabled="true">{content}</div>}
    <button
      type="button"
      className={`policy-option-latency${delay ? ` ${latencyClass(delay, thresholds)}` : ""}${testing ? " testing" : ""}`}
      aria-label={`测试 ${option.name} 延迟`}
      title={`测试 ${option.name} 延迟`}
      aria-busy={testing || undefined}
      disabled={disabled || testDisabled}
      onClick={onTest}
    >{testing
      ? <span className="latency-testing-dots" aria-hidden="true"><i /><i /><i /></span>
      : delay ?? <BoltIcon />}</button>
  </div>;
  return <button type="button" className={`policy-option${active ? " active" : ""}`} disabled={disabled || !onClick} onClick={onClick}>
    {content}
    <strong className={delay ? latencyClass(delay, thresholds) : ""}>{delay ?? <BoltIcon />}</strong>
  </button>;
}

export function proxyMatchesQuery(option: ControllerProxy, query: string) {
  return matchesQuery([option.name, option.type, proxyType(option.type), option.udp ? "udp" : undefined], query);
}

export function proxyType(value: string) {
  const normalized = value.toLocaleLowerCase();
  if (normalized === "hysteria2") return "hy2";
  if (normalized === "urltest") return "urltest";
  return normalized;
}

export function latencyClass(delay: number, thresholds: LatencyThresholds) {
  return delay > thresholds.medium ? "danger" : delay > thresholds.low ? "warning" : "success";
}

export function isVisibleProxy(proxy: ControllerProxy, hideUnavailable: boolean) {
  return !hideUnavailable || proxy.alive !== false;
}

export function proxyOptionAction(groupType: string) {
  return groupType.toLocaleLowerCase() === "selector" ? "select" : "test";
}

export function recentLatencyHistory(history: ProxyHistory[], limit = 10) {
  return history.slice(-limit).reverse();
}

export function formatLatencyTime(value: string) {
  const date = new Date(value);
  if (!Number.isFinite(date.getTime())) return value.replace("T", " ").replace(/\.\d{3}Z$/, "");
  const part = (item: number) => String(item).padStart(2, "0");
  return `${date.getFullYear()}-${part(date.getMonth() + 1)}-${part(date.getDate())} ${part(date.getHours())}:${part(date.getMinutes())}:${part(date.getSeconds())}`;
}

export function getPenetrationGroup(group: ControllerProxy, data: ProxiesResponse | null) {
  const selected = group.now ? data?.proxies[group.now] : undefined;
  return selected?.all?.length ? selected : undefined;
}

export function selectedProxyDelay(group: ControllerProxy, data: ProxiesResponse | null) {
  const visited = new Set<string>();
  let selected: ControllerProxy | undefined = group;
  while (selected?.now) {
    if (visited.has(selected.name)) return null;
    visited.add(selected.name);
    selected = data?.proxies[selected.now];
    if (!selected) return null;
  }
  return selected === group ? null : latestLatency(selected.history);
}

export function sortPenetrationOptions(options: ControllerProxy[]) {
  return sortProxyNodes(options, "latencyAsc");
}

export function sortProxyNodes(options: ControllerProxy[], mode: ProxyNodeSortMode) {
  const indexed = options.map((option, index) => ({ option, index, delay: latestLatency(option.history) }));
  if (mode === "default") return options;
  if (mode === "nameAsc" || mode === "nameDesc") return indexed
    .sort((left, right) => (mode === "nameAsc" ? 1 : -1) * left.option.name.localeCompare(right.option.name, "zh-CN") || left.index - right.index)
    .map(item => item.option);
  return indexed.sort((left, right) => {
    if (left.delay == null && right.delay == null) return left.index - right.index;
    if (left.delay == null) return 1;
    if (right.delay == null) return -1;
    return (mode === "latencyAsc" ? left.delay - right.delay : right.delay - left.delay) || left.index - right.index;
  }).map(item => item.option);
}

export function summarizeGroupLatencyTest(result: Record<string, number>, total: number) {
  const success = Object.values(result).filter(delay => Number.isFinite(delay) && delay > 0).length;
  return { success, failed: Math.max(0, total - success) };
}

export function mergeProxyLatency(proxies: Record<string, ControllerProxy>, name: string, delay: number, time = new Date().toISOString()) {
  const proxy = proxies[name];
  if (!proxy) return proxies;
  return {
    ...proxies,
    [name]: {
      ...proxy,
      history: [...(proxy.history ?? []), { time, delay }].slice(-10),
    },
  };
}

export function mergeProxyLatencies(proxies: Record<string, ControllerProxy>, delays: Record<string, number>, time = new Date().toISOString()) {
  return Object.entries(delays).reduce((current, [name, delay]) => mergeProxyLatency(current, name, delay, time), proxies);
}

export function appendProxyGroupLatencyHistory(history: ProxyHistory[], delays: Record<string, number>, time = new Date().toISOString()) {
  const entries = Object.entries(delays).map(([node, delay]) => ({ time, delay, node }));
  return [...history, ...entries].slice(-10);
}

export function mergeProxyGroupLatencyResult(proxies: Record<string, ControllerProxy>, groupName: string, delays: Record<string, number>, time = new Date().toISOString()) {
  const next = mergeProxyLatencies(proxies, delays, time);
  const group = next[groupName];
  if (!group) return next;
  return {
    ...next,
    [groupName]: {
      ...group,
      history: appendProxyGroupLatencyHistory(group.history ?? [], delays, time),
    },
  };
}

export function preserveProxyLatencies(snapshot: Record<string, ControllerProxy>, current: Record<string, ControllerProxy>, names: Iterable<string>) {
  let next = snapshot;
  for (const name of names) {
    const snapshotProxy = snapshot[name];
    const currentProxy = current[name];
    if (!snapshotProxy || !currentProxy) continue;
    if (next === snapshot) next = { ...snapshot };
    next[name] = { ...snapshotProxy, history: currentProxy.history };
  }
  return next;
}

const EMPTY_TESTING_NODES: ReadonlySet<string> = new Set();
