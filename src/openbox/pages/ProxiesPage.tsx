import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { ArrowPathIcon, Bars3Icon, BoltIcon, ChevronDownIcon, ChevronUpIcon, MapPinIcon, PencilSquareIcon, WrenchScrewdriverIcon } from "@heroicons/react/24/outline";
import { toast } from "sonner";
import { api } from "../api/client";
import type { ControllerProxy, ProxiesResponse, ProxyHistory, StorageResponse, Subscription } from "../api/types";
import { latestLatency } from "../lib/format";
import { SearchInputGroup, SelectControl, SegmentedGroup, SegmentedItem } from "../ui/controls";
import { ErrorState, IconButton, Modal } from "../components/shared";
import { appendProxyGroupLatencyHistory, NodeHealthDots, ProxyGroupCard, ProxyOption, groupIcon, isVisibleProxy, mergeProxyGroupLatencyResult, mergeProxyLatencies, preserveProxyLatencies, proxyMatchesQuery, selectedProxyDelay, sortProxyNodes, summarizeGroupLatencyTest } from "../components/ProxyCards";
import type { ProxyNodeSortMode } from "../components/ProxyCards";
export { appendProxyGroupLatencyHistory, NodeHealthDots, PenetrationGroup, PolicyLatency, ProxyGroupCard, ProxyOption, formatLatencyTime, getPenetrationGroup, isVisibleProxy, latencyClass, mergeProxyGroupLatencyResult, mergeProxyLatencies, mergeProxyLatency, preserveProxyLatencies, proxyMatchesQuery, proxyOptionAction, recentLatencyHistory, selectedProxyDelay, sortPenetrationOptions, sortProxyNodes, summarizeGroupLatencyTest } from "../components/ProxyCards";
export type { LatencyThresholds, ProxyNodeSortMode } from "../components/ProxyCards";

type ViewTab = "groups" | "nodes" | "subscriptions";
export type ProxySortMode = "default" | "latency" | "name" | "custom";
type ProxyViewSettings = {
  nodeSort: ProxyNodeSortMode;
  groupByProvider: boolean;
  nodeCardMinWidth: number;
  strategyOrder: string[];
};
const PROXY_VIEW_TAB_KEY = "openbox:proxy-view-tab";
export const PROXY_VIEW_SETTINGS_KEY = "openbox:proxy-view-settings";
const defaultViewSettings = (): ProxyViewSettings => ({ nodeSort: "latencyAsc", groupByProvider: true, nodeCardMinWidth: 145, strategyOrder: [] });

export function ProxiesPage({ storage, onToast, onNodeCardMinWidthChange }: { storage: StorageResponse | null; onToast: (message: string) => void; onNodeCardMinWidthChange: (width: number) => void }) {
  const [data, setData] = useState<ProxiesResponse | null>(null);
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [latencyHistory, setLatencyHistory] = useState<Record<string, ProxyHistory[]>>({});
  const [tab, setTab] = useState<ViewTab>(() => proxyViewTab(typeof window === "undefined" ? null : window.sessionStorage.getItem(PROXY_VIEW_TAB_KEY)));
  const [query, setQuery] = useState("");
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState("");
  const [testingNodes, setTestingNodes] = useState<Set<string>>(() => new Set());
  const testingNodesRef = useRef(new Set<string>());
  const latencyRevisionRef = useRef(0);
  const latencyRevisionByNodeRef = useRef(new Map<string, number>());
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const [penetrationExpanded, setPenetrationExpanded] = useState<Record<string, boolean>>({});
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOrder, setDragOrder] = useState<string[] | null>(null);
  const orderItemRefs = useRef(new Map<string, HTMLDivElement>());
  const orderItemRects = useRef<Map<string, DOMRect> | null>(null);
  const [viewSettings, setViewSettings] = useState<ProxyViewSettings>(() => proxyViewSettings(typeof window === "undefined" ? null : window.localStorage.getItem(PROXY_VIEW_SETTINGS_KEY)));

  const load = async () => {
    const startedAtRevision = latencyRevisionRef.current;
    setError(null);
    try {
      const [proxies, subscriptionItems, latency] = await Promise.all([api.proxies(), api.subscriptions(), api.proxyLatencyHistory()]);
      setData(current => {
        if (!current || latencyRevisionRef.current === startedAtRevision) return proxies;
        const newerNodes = Array.from(latencyRevisionByNodeRef.current)
          .filter(([, revision]) => revision > startedAtRevision)
          .map(([name]) => name);
        return { proxies: preserveProxyLatencies(proxies.proxies, current.proxies, newerNodes) };
      });
      setSubscriptions(subscriptionItems);
      setLatencyHistory(latency.history);
    } catch (reason) {
      setError(reason);
    }
  };
  useEffect(() => { void load(); }, []);

  const entries = useMemo(() => Object.values(data?.proxies ?? {}), [data]);
  const { customGroups, nodeGroups } = useMemo(() => getProxyViewGroups(data), [data]);
  const timeout = Number(storage?.entries["config/speedtest-timeout"] ?? 5000);
  const testUrl = storage?.entries["config/speedtest-url"] ?? "http://www.gstatic.com/generate_204";
  const columns = Math.min(3, Math.max(1, Number(storage?.entries["config/proxy-group-columns"] ?? 1)));
  const hideUnavailable = storage?.entries["config/hide-unavailable-proxies"] === "true";
  const thresholds = {
    low: finiteSetting(storage?.entries["config/low-latency"], 500),
    medium: finiteSetting(storage?.entries["config/medium-latency"], 1000),
  };
  const listStyle = { gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` };
  const testingGroup = busy.startsWith("test:") ? busy.slice(5) : "";
  const visibleGroups = useMemo(() => sortProxyViewItems(customGroups, { mode: "custom", order: viewSettings.strategyOrder }, group => group.name, group => group.name, group => selectedProxyDelay(group, data)), [customGroups, data, viewSettings.strategyOrder]);
  const visibleNodeGroups = nodeGroups;
  const visibleSubscriptions = subscriptions;
  const allSubscriptionNodes = useMemo(() => uniqueSubscriptionNodes(subscriptions, entries), [entries, subscriptions]);
  const changeTab = (value: string) => {
    const next = proxyViewTab(value);
    setTab(next);
    setSettingsOpen(false);
    window.sessionStorage.setItem(PROXY_VIEW_TAB_KEY, next);
  };

  const saveViewSettings = (next: ProxyViewSettings) => {
    setViewSettings(next);
    window.localStorage.setItem(PROXY_VIEW_SETTINGS_KEY, JSON.stringify(next));
    onNodeCardMinWidthChange(next.nodeCardMinWidth);
  };

  const updateOrder = (order: string[]) => {
    saveViewSettings({ ...viewSettings, strategyOrder: order });
  };

  const applyLatencyResults = (results: Record<string, number>, groupName?: string) => {
    const names = Object.keys(results);
    if (!names.length) return;
    const time = new Date().toISOString();
    const revision = ++latencyRevisionRef.current;
    for (const name of names) latencyRevisionByNodeRef.current.set(name, revision);
    setData(current => current ? {
      proxies: groupName
        ? mergeProxyGroupLatencyResult(current.proxies, groupName, results, time)
        : mergeProxyLatencies(current.proxies, results, time),
    } : current);
    if (groupName) setLatencyHistory(current => ({
      ...current,
      [groupName]: appendProxyGroupLatencyHistory(current[groupName] ?? [], results, time),
    }));
  };

  const select = async (group: string, name: string) => {
    setBusy(`${group}:${name}`);
    try {
      await api.selectProxy(group, name);
      setData(current => current ? { proxies: { ...current.proxies, [group]: { ...current.proxies[group], now: name } } } : current);
      onToast(`${group} 已切换到 ${name}`);
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "切换失败");
    } finally {
      setBusy("");
    }
  };

  const testGroup = async (group: string) => {
    const total = data?.proxies[group]?.all?.length ?? 0;
    const toastId = toast.loading(group, {
      description: `0/${total} 测试完成`,
      duration: Infinity,
    });
    setBusy(`test:${group}`);
    try {
      const result = await api.testProxyGroup(group, testUrl, timeout);
      const summary = summarizeGroupLatencyTest(result, total);
      applyLatencyResults(result, group);
      const detail = `测试完成：${summary.success} 成功，${summary.failed} 失败${summary.failed ? `（超时 ${summary.failed}）` : ""}`;
      const present = summary.failed ? toast.warning : toast.success;
      present(group, { id: toastId, description: detail, duration: Infinity });
      window.setTimeout(() => toast.dismiss(toastId), 4200);
    } catch (reason) {
      toast.error(group, {
        id: toastId,
        description: reason instanceof Error ? reason.message : "延迟测试失败",
        duration: Infinity,
      });
      window.setTimeout(() => toast.dismiss(toastId), 5000);
    } finally {
      setBusy("");
    }
  };

  const testAll = async () => {
    if (!allSubscriptionNodes.length) return;
    setBusy("test:all");
    try {
      const results = await Promise.allSettled(allSubscriptionNodes.map(node => api.testProxy(node.name, testUrl, timeout)));
      applyLatencyResults(successfulProxyDelays(allSubscriptionNodes, results));
      const failed = results.filter(result => result.status === "rejected").length;
      onToast(failed ? `延迟测试完成，${failed} 个节点失败` : "延迟测试完成");
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "延迟测试失败");
    } finally {
      setBusy("");
    }
  };

  const testNode = async (name: string) => {
    if (testingNodesRef.current.has(name)) return;
    testingNodesRef.current.add(name);
    setTestingNodes(new Set(testingNodesRef.current));
    try {
      const { delay } = await api.testProxy(name, testUrl, timeout);
      applyLatencyResults({ [name]: delay });
      onToast(`${name} 延迟测试完成`);
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "延迟测试失败");
    } finally {
      testingNodesRef.current.delete(name);
      setTestingNodes(new Set(testingNodesRef.current));
    }
  };

  const testSubscription = async (subscription: Subscription, nodes: ControllerProxy[]) => {
    if (!nodes.length) return;
    setBusy(`test-subscription:${subscription.id}`);
    try {
      const results = await Promise.allSettled(nodes.map(node => api.testProxy(node.name, testUrl, timeout)));
      applyLatencyResults(successfulProxyDelays(nodes, results));
      const failed = results.filter(result => result.status === "rejected").length;
      onToast(failed ? `${subscription.name} 延迟测试完成，${failed} 个节点失败` : `${subscription.name} 延迟测试完成`);
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "延迟测试失败");
    } finally {
      setBusy("");
    }
  };

  const refreshSubscription = async (subscription: Subscription) => {
    setBusy(`subscription:${subscription.id}`);
    try {
      await api.refreshSubscription(subscription.id);
      await load();
      onToast(`${subscription.name} 已刷新`);
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "刷新失败");
    } finally {
      setBusy("");
    }
  };

  const refreshAllSubscriptions = async () => {
    if (!subscriptions.length) return;
    setBusy("subscription:all");
    try {
      const results = await Promise.allSettled(subscriptions.map(subscription => api.refreshSubscription(subscription.id)));
      await load();
      const failed = results.filter(result => result.status === "rejected").length;
      onToast(failed ? `订阅刷新完成，${failed} 个失败` : "全部订阅已刷新");
    } finally {
      setBusy("");
    }
  };

  const currentItems: Array<{ id: string; label: string; icon?: string }> = tab === "groups"
    ? customGroups.map(group => ({ id: group.name, label: group.name, icon: groupIcon(group.name) }))
    : tab === "nodes"
      ? nodeGroups.map(group => ({ id: group.name, label: group.name, icon: groupIcon(group.name) }))
      : subscriptions.map(subscription => ({ id: subscription.id, label: subscription.name }));
  const strategyItems = customGroups.map(group => ({ id: group.name, label: group.name, icon: groupIcon(group.name) }));
  const strategyOrder = completeProxyViewOrder(viewSettings.strategyOrder, strategyItems.map(item => item.id));
  const displayedStrategyOrder = dragOrder ?? strategyOrder;
  const allCurrentCardsCollapsed = currentItems.length > 0 && currentItems.every(item => collapsed[proxyCardKey(tab, item.id)]);
  const toggleAllCards = () => setCollapsed(current => toggleProxyCardsCollapsed(current, tab, currentItems.map(item => item.id)));
  const captureOrderRects = () => {
    orderItemRects.current = new Map(Array.from(orderItemRefs.current, ([id, element]) => [id, element.getBoundingClientRect()]));
  };
  const previewItemMove = (source: string, target: string) => {
    if (source === target) return;
    const current = dragOrder ?? strategyOrder;
    const next = reorderProxyViewIds(current, source, target);
    if (next === current) return;
    captureOrderRects();
    setDragOrder(next);
  };
  useLayoutEffect(() => {
    const previousRects = orderItemRects.current;
    orderItemRects.current = null;
    if (!previousRects || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    for (const [id, element] of orderItemRefs.current) {
      const previous = previousRects.get(id);
      if (!previous) continue;
      const offset = previous.top - element.getBoundingClientRect().top;
      if (offset) element.animate(
        [{ transform: `translateY(${offset}px)` }, { transform: "translateY(0)" }],
        { duration: 180, easing: "cubic-bezier(.2, .8, .2, 1)" },
      );
    }
  }, [dragOrder]);

  return <>
  <main className="page proxies-page">
    <div className="proxy-controls">
      <SegmentedGroup className="proxy-tabs" value={tab} onValueChange={changeTab} aria-label="代理视图">
        <SegmentedItem value="groups">策略 <span>({customGroups.length})</span></SegmentedItem>
        <SegmentedItem value="nodes">节点 <span>({nodeGroups.length})</span></SegmentedItem>
        <SegmentedItem value="subscriptions">订阅 <span>({subscriptions.length})</span></SegmentedItem>
      </SegmentedGroup>
      <SearchInputGroup label="搜索代理" value={query} onChange={setQuery} onClear={() => setQuery("")} placeholder="搜索 | 多个关键词用空格分隔" />
      <div className="proxy-tools">
        <ProxyTool label="策略设置" onClick={() => setSettingsOpen(true)}><WrenchScrewdriverIcon /></ProxyTool>
        <ProxyTool label={allCurrentCardsCollapsed ? "展开全部卡片" : "收起全部卡片"} disabled={!currentItems.length} onClick={toggleAllCards}>{allCurrentCardsCollapsed ? <ChevronDownIcon /> : <ChevronUpIcon />}</ProxyTool>
        <ProxyTool label="全部测速" disabled={Boolean(busy) || !allSubscriptionNodes.length} onClick={() => void testAll()}><BoltIcon className={busy === "test:all" ? "testing-bolt" : undefined} /></ProxyTool>
        {tab === "subscriptions" && <ProxyTool label="刷新所有订阅" disabled={Boolean(busy) || !subscriptions.length} onClick={() => void refreshAllSubscriptions()}><ArrowPathIcon /></ProxyTool>}
      </div>
    </div>
    {Boolean(error) && <ErrorState title="代理数据加载失败" error={error} onRetry={() => void load()} />}

    {tab === "groups" && <div className="policy-list" style={listStyle}>{visibleGroups.map(group => <ProxyGroupCard
      key={group.name}
      group={group}
      data={data}
      collapsed={Boolean(collapsed[proxyCardKey("groups", group.name)])}
      busy={Boolean(busy)}
      onToggle={() => setCollapsed(value => ({ ...value, [proxyCardKey("groups", group.name)]: !value[proxyCardKey("groups", group.name)] }))}
      penetrationExpanded={Boolean(penetrationExpanded[group.name])}
      onTogglePenetration={() => setPenetrationExpanded(value => ({ ...value, [group.name]: !value[group.name] }))}
      onSelect={select}
      onTest={testGroup}
      onTestNode={testNode}
      variant="policy"
      hideUnavailable={hideUnavailable}
      thresholds={thresholds}
      history={latencyHistory[group.name] ?? group.history ?? []}
      latencyHistory={latencyHistory}
      testingGroup={testingGroup}
      testingNodes={testingNodes}
      query={query}
      nodeSort={viewSettings.nodeSort}
      groupByProvider={viewSettings.groupByProvider}
    />)}</div>}

    {tab === "nodes" && <div className="policy-list" style={listStyle}>{visibleNodeGroups.map(group => <ProxyGroupCard
      key={group.name}
      group={group}
      data={data}
      collapsed={Boolean(collapsed[proxyCardKey("nodes", group.name)])}
      busy={Boolean(busy)}
      onToggle={() => setCollapsed(value => ({ ...value, [proxyCardKey("nodes", group.name)]: !value[proxyCardKey("nodes", group.name)] }))}
      onSelect={select}
      onTest={testGroup}
      onTestNode={testNode}
      variant="nodes"
      hideUnavailable={hideUnavailable}
      thresholds={thresholds}
      history={latencyHistory[group.name] ?? group.history ?? []}
      latencyHistory={latencyHistory}
      testingGroup={testingGroup}
      testingNodes={testingNodes}
      query={query}
      nodeSort={viewSettings.nodeSort}
      groupByProvider={viewSettings.groupByProvider}
    />)}</div>}

    {tab === "subscriptions" && <div className="policy-list">{visibleSubscriptions.map(subscription => {
      const allOptions = sortProxyNodes(subscriptionNodes(subscription, entries).filter(option => isVisibleProxy(option, hideUnavailable)), viewSettings.nodeSort);
      const options = allOptions.filter(option => proxyMatchesQuery(option, query));
      const tested = options.filter(option => latestLatency(option.history)).length;
      const cardKey = proxyCardKey("subscriptions", subscription.id);
      const isCollapsed = Boolean(collapsed[cardKey]);
      const isTesting = busy === "test:all" || busy === `test-subscription:${subscription.id}`;
      return <article className={`surface subscription-provider-card${isCollapsed ? " subscription-provider-collapsed" : ""}`} key={subscription.id}>
        <header>
          <button
            type="button"
            className="subscription-provider-toggle"
            aria-label={`${isCollapsed ? "展开" : "收起"} ${subscription.name}`}
            aria-expanded={!isCollapsed}
            onClick={() => setCollapsed(value => ({ ...value, [cardKey]: !value[cardKey] }))}
          >
            <div>
              <h2>{subscription.name}</h2><span>({tested}/{allOptions.length || subscription.nodeCount})</span>
              <small>更新于 {relativeTime(subscription.updatedAt)}</small>
              <div className={`node-health-summary${isCollapsed ? " visible" : ""}`} aria-hidden={!isCollapsed}>
                <div className="node-health-summary-inner"><NodeHealthDots options={allOptions} thresholds={thresholds} /></div>
              </div>
            </div>
          </button>
          <div className="provider-actions">
            <ProxyTool label="测试这条订阅的节点延迟" disabled={Boolean(busy) || !allOptions.length} onClick={() => void testSubscription(subscription, allOptions)}><BoltIcon className={isTesting ? "testing-bolt" : undefined} /></ProxyTool>
            <ProxyTool label="刷新" disabled={Boolean(busy)} onClick={() => void refreshSubscription(subscription)}><ArrowPathIcon /></ProxyTool>
            <ProxyTool label="修改订阅" onClick={() => { sessionStorage.setItem("openbox:settings-section", "subscriptions"); window.location.hash = "#/settings"; }}><PencilSquareIcon /></ProxyTool>
          </div>
        </header>
        <section className={`subscription-provider-collapse${isCollapsed ? " collapsed" : ""}`} aria-hidden={isCollapsed} inert={isCollapsed || undefined}>
          <div className="subscription-provider-collapse-inner"><div className="policy-options provider-options">{options.map(option => <ProxyOption
            key={option.name}
            option={option}
            active={false}
            disabled={Boolean(busy) || testingNodes.has(option.name)}
            thresholds={thresholds}
            testing={isTesting || testingNodes.has(option.name)}
            onTest={() => void testNode(option.name)}
          />)}</div></div>
        </section>
      </article>;
    })}</div>}
  </main>;
  {settingsOpen && <Modal title="策略设置" className="proxy-settings-modal" onClose={() => setSettingsOpen(false)}>
    <div className="proxy-settings">
      <label className="proxy-setting-row"><span>节点排序方式</span><SelectControl label="节点排序方式" value={viewSettings.nodeSort} onValueChange={value => saveViewSettings({ ...viewSettings, nodeSort: value as ProxyNodeSortMode })} options={[{ value: "default", label: "按配置排序" }, { value: "nameAsc", label: "按名称升序" }, { value: "nameDesc", label: "按名称降序" }, { value: "latencyAsc", label: "按延迟升序" }, { value: "latencyDesc", label: "按延迟降序" }]} /></label>
      <label className="proxy-setting-row"><span>节点根据提供商分组</span><input className="proxy-setting-toggle" type="checkbox" checked={viewSettings.groupByProvider} onChange={event => saveViewSettings({ ...viewSettings, groupByProvider: event.target.checked })} /></label>
      <label className="proxy-setting-row"><span>节点卡片最小宽度</span><span className="proxy-width-control"><input aria-label="节点卡片最小宽度" type="number" min="100" max="320" value={viewSettings.nodeCardMinWidth} onChange={event => saveViewSettings({ ...viewSettings, nodeCardMinWidth: clampNodeCardWidth(event.target.value) })} /><button type="button" onClick={() => saveViewSettings({ ...viewSettings, nodeCardMinWidth: 145 })}>重置</button></span></label>
      <div className="proxy-settings-divider" />
      <div className="proxy-settings-columns">
        <section className="proxy-settings-column">
          <h3>显示排序</h3>
          <div className="proxy-order-list">{displayedStrategyOrder.map(id => {
            const item = strategyItems.find(candidate => candidate.id === id);
            if (!item) return null;
            return <div
              className={`proxy-order-item${draggedItem === id ? " dragging" : ""}`}
              ref={element => { if (element) orderItemRefs.current.set(id, element); else orderItemRefs.current.delete(id); }}
              draggable
              key={id}
              onDragStart={event => { setDraggedItem(id); setDragOrder(strategyOrder); event.dataTransfer.effectAllowed = "move"; event.dataTransfer.setData("text/plain", id); }}
              onDragEnd={() => { captureOrderRects(); setDraggedItem(null); setDragOrder(null); }}
              onDragEnter={() => { if (draggedItem) previewItemMove(draggedItem, id); }}
              onDragOver={event => { event.preventDefault(); event.dataTransfer.dropEffect = "move"; }}
              onDrop={event => { event.preventDefault(); updateOrder(dragOrder ?? strategyOrder); setDraggedItem(null); setDragOrder(null); }}
            >
              <span className="proxy-order-handle" aria-hidden="true"><Bars3Icon /></span>
              {item.icon && <img src={item.icon} alt="" />}
              <strong>{item.label}</strong>
            </div>;
          })}</div>
          <p>拖动调整;代理页的「策略」按这个顺序显示,和命中顺序无关。</p>
        </section>
        <section className="proxy-settings-column">
          <h3>命中规则排序</h3>
          <div className="proxy-rule-order-list">
            <div className="proxy-rule-order-item pinned"><MapPinIcon /><strong>前置自定义分流</strong><small>前置</small><em>未生效</em></div>
            {customGroups.map((group, index) => <div className="proxy-rule-order-item" key={group.name}><span>{index + 1}</span><img src={groupIcon(group.name)} alt="" /><strong>{group.name}</strong></div>)}
          </div>
          <p>流量按这个顺序逐条匹配,首条命中生效;有域名的访问只看域名条件,都没命中就走兜底,IP 条件只管直接按 IP 发起的连接。要改去「设置 → 目标分流」。</p>
        </section>
      </div>
    </div>
  </Modal>}
  </>;
}




function ProxyTool({ label, disabled, onClick, children }: {
  label: string;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return <span className="proxy-tool-with-tooltip">
    <IconButton label={label} title={undefined} disabled={disabled} onClick={onClick}>{children}</IconButton>
    <span className="proxy-tool-tooltip" role="tooltip">{label}</span>
  </span>;
}




function subscriptionNodes(subscription: Subscription, entries: ControllerProxy[]) {
  const prefix = `${subscription.name} |`;
  return entries.filter(proxy => !proxy.all?.length && proxy.name.startsWith(prefix));
}

export function uniqueSubscriptionNodes(subscriptions: Subscription[], entries: ControllerProxy[]) {
  const unique = new Map<string, ControllerProxy>();
  subscriptions.forEach(subscription => subscriptionNodes(subscription, entries).forEach(node => unique.set(node.name, node)));
  return [...unique.values()];
}

function successfulProxyDelays(nodes: ControllerProxy[], results: PromiseSettledResult<{ delay: number }>[]) {
  return Object.fromEntries(results.flatMap((result, index) => result.status === "fulfilled"
    ? [[nodes[index].name, result.value.delay]]
    : []));
}








function finiteSetting(value: string | undefined, fallback: number) {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : fallback;
}

function relativeTime(value: string | number) {
  const timestamp = new Date(value).getTime();
  if (!Number.isFinite(timestamp)) return String(value);
  const hours = Math.max(0, Math.floor((Date.now() - timestamp) / 3_600_000));
  return hours < 1 ? "刚刚" : `${hours} 小时前`;
}

export function getProxyViewGroups(data: ProxiesResponse | null) {
  const entries = Object.values(data?.proxies ?? {});
  return {
    customGroups: entries.filter(proxy => proxy.type === "Selector" && !["GLOBAL", "所有-手动"].includes(proxy.name)),
    nodeGroups: ["所有-自动", "所有-手动"].map(name => data?.proxies[name]).filter(Boolean) as ControllerProxy[],
  };
}

export function proxyViewTab(value: string | null): ViewTab {
  return value === "nodes" || value === "subscriptions" ? value : "groups";
}

export function proxyViewSettings(value: string | null): ProxyViewSettings {
  const fallback = defaultViewSettings();
  if (!value) return fallback;
  try {
    const parsed = JSON.parse(value) as Partial<ProxyViewSettings> & { groups?: { order?: unknown } };
    const validSortModes: ProxyNodeSortMode[] = ["default", "nameAsc", "nameDesc", "latencyAsc", "latencyDesc"];
    if (typeof parsed.nodeSort === "string" && validSortModes.includes(parsed.nodeSort as ProxyNodeSortMode)) fallback.nodeSort = parsed.nodeSort as ProxyNodeSortMode;
    if (typeof parsed.groupByProvider === "boolean") fallback.groupByProvider = parsed.groupByProvider;
    fallback.nodeCardMinWidth = clampNodeCardWidth(parsed.nodeCardMinWidth);
    const order = Array.isArray(parsed.strategyOrder) ? parsed.strategyOrder : parsed.groups?.order;
    fallback.strategyOrder = Array.isArray(order) ? order.filter((item): item is string => typeof item === "string") : [];
  } catch {
    return fallback;
  }
  return fallback;
}

export function clampNodeCardWidth(value: unknown) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? Math.min(320, Math.max(100, Math.round(parsed))) : 145;
}

export function sortProxyViewItems<T>(items: T[], settings: { mode: ProxySortMode; order: string[] }, idOf: (item: T) => string, nameOf: (item: T) => string, latencyOf: (item: T) => number | null) {
  const indexed = items.map((item, index) => ({ item, index }));
  if (settings.mode === "default") return items;
  if (settings.mode === "name") return indexed.sort((left, right) => nameOf(left.item).localeCompare(nameOf(right.item), "zh-CN") || left.index - right.index).map(entry => entry.item);
  if (settings.mode === "latency") return indexed.sort((left, right) => {
    const leftDelay = latencyOf(left.item);
    const rightDelay = latencyOf(right.item);
    if (leftDelay == null && rightDelay == null) return left.index - right.index;
    if (leftDelay == null) return 1;
    if (rightDelay == null) return -1;
    return leftDelay - rightDelay || left.index - right.index;
  }).map(entry => entry.item);
  const positions = new Map(settings.order.map((id, index) => [id, index]));
  return indexed.sort((left, right) => {
    const leftPosition = positions.get(idOf(left.item));
    const rightPosition = positions.get(idOf(right.item));
    if (leftPosition == null && rightPosition == null) return left.index - right.index;
    if (leftPosition == null) return 1;
    if (rightPosition == null) return -1;
    return leftPosition - rightPosition || left.index - right.index;
  }).map(entry => entry.item);
}

export function completeProxyViewOrder(savedOrder: string[], currentIds: string[]) {
  const current = new Set(currentIds);
  const seen = new Set<string>();
  return [...savedOrder, ...currentIds].filter(id => {
    if (!current.has(id) || seen.has(id)) return false;
    seen.add(id);
    return true;
  });
}

export function reorderProxyViewIds(order: string[], source: string, target: string) {
  const from = order.indexOf(source);
  const to = order.indexOf(target);
  return from < 0 || to < 0 ? order : moveProxyViewId(order, from, to);
}

export function moveProxyViewId(order: string[], from: number, to: number) {
  if (from < 0 || to < 0 || from >= order.length || to >= order.length || from === to) return order;
  const next = [...order];
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}

function proxyViewLabel(tab: ViewTab) {
  return tab === "groups" ? "策略" : tab === "nodes" ? "节点" : "订阅";
}

function proxyCardKey(tab: ViewTab, id: string) {
  return `${tab}:${id}`;
}

export function toggleProxyCardsCollapsed(current: Record<string, boolean>, tab: ViewTab, ids: string[]) {
  if (!ids.length) return current;
  const shouldCollapse = !ids.every(id => current[proxyCardKey(tab, id)]);
  return {
    ...current,
    ...Object.fromEntries(ids.map(id => [proxyCardKey(tab, id), shouldCollapse])),
  };
}

function subscriptionLatency(subscription: Subscription, entries: ControllerProxy[]) {
  const measured = subscriptionNodes(subscription, entries).map(node => latestLatency(node.history)).filter((delay): delay is number => delay != null);
  return measured.length ? Math.min(...measured) : null;
}
