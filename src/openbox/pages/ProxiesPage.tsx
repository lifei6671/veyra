import { useEffect, useId, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { ArrowPathIcon, Bars3Icon, BoltIcon, ChevronDownIcon, ChevronUpIcon, MapPinIcon, PencilSquareIcon, WrenchScrewdriverIcon } from "@heroicons/react/24/outline";
import { toast } from "sonner";
import { api } from "../api/client";
import type { ControllerProxy, ProxiesResponse, ProxyHistory, StorageResponse, Subscription } from "../api/types";
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
import { SearchInputGroup, SegmentedGroup, SegmentedItem } from "../ui/controls";
import { ErrorState, IconButton, Modal } from "../components/shared";

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

type ViewTab = "groups" | "nodes" | "subscriptions";
export type ProxySortMode = "default" | "latency" | "name" | "custom";
export type ProxyNodeSortMode = "default" | "nameAsc" | "nameDesc" | "latencyAsc" | "latencyDesc";
type ProxyViewSettings = {
  nodeSort: ProxyNodeSortMode;
  groupByProvider: boolean;
  nodeCardMinWidth: number;
  strategyOrder: string[];
};
export type LatencyThresholds = { low: number; medium: number };
const PROXY_VIEW_TAB_KEY = "openbox:proxy-view-tab";
const PROXY_VIEW_SETTINGS_KEY = "openbox:proxy-view-settings";
const defaultViewSettings = (): ProxyViewSettings => ({ nodeSort: "latencyAsc", groupByProvider: true, nodeCardMinWidth: 145, strategyOrder: [] });

export function ProxiesPage({ storage, onToast }: { storage: StorageResponse | null; onToast: (message: string) => void }) {
  const [data, setData] = useState<ProxiesResponse | null>(null);
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [latencyHistory, setLatencyHistory] = useState<Record<string, ProxyHistory[]>>({});
  const [tab, setTab] = useState<ViewTab>(() => proxyViewTab(typeof window === "undefined" ? null : window.sessionStorage.getItem(PROXY_VIEW_TAB_KEY)));
  const [query, setQuery] = useState("");
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState("");
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const [penetrationExpanded, setPenetrationExpanded] = useState<Record<string, boolean>>({});
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [viewSettings, setViewSettings] = useState<ProxyViewSettings>(() => proxyViewSettings(typeof window === "undefined" ? null : window.localStorage.getItem(PROXY_VIEW_SETTINGS_KEY)));

  const load = async () => {
    setError(null);
    try {
      const [proxies, subscriptionItems, latency] = await Promise.all([api.proxies(), api.subscriptions(), api.proxyLatencyHistory()]);
      setData(proxies);
      setSubscriptions(subscriptionItems);
      setLatencyHistory(latency.history);
    } catch (reason) {
      setError(reason);
    }
  };
  useEffect(() => { void load(); }, []);

  const entries = useMemo(() => Object.values(data?.proxies ?? {}), [data]);
  const { customGroups, nodeGroups } = useMemo(() => getProxyViewGroups(data), [data]);
  const filteredGroups = useMemo(() => customGroups.filter(group => groupMatches(group, data, query)), [customGroups, data, query]);
  const filteredNodeGroups = useMemo(() => nodeGroups.filter(group => groupMatches(group, data, query)), [nodeGroups, data, query]);
  const filteredSubscriptions = useMemo(() => subscriptions.filter(item => matchesQuery([item.name, item.format], query)), [subscriptions, query]);
  const timeout = Number(storage?.entries["config/speedtest-timeout"] ?? 5000);
  const testUrl = storage?.entries["config/speedtest-url"] ?? "http://www.gstatic.com/generate_204";
  const columns = Math.min(3, Math.max(1, Number(storage?.entries["config/proxy-group-columns"] ?? 1)));
  const hideUnavailable = storage?.entries["config/hide-unavailable-proxies"] === "true";
  const thresholds = {
    low: finiteSetting(storage?.entries["config/low-latency"], 500),
    medium: finiteSetting(storage?.entries["config/medium-latency"], 1000),
  };
  const listStyle = { gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` };
  const pageStyle = { "--proxy-node-card-min-width": `${viewSettings.nodeCardMinWidth}px` } as React.CSSProperties;
  const testingGroup = busy.startsWith("test:") ? busy.slice(5) : "";
  const visibleGroups = useMemo(() => sortProxyViewItems(filteredGroups, { mode: "custom", order: viewSettings.strategyOrder }, group => group.name, group => group.name, group => selectedProxyDelay(group, data)), [data, filteredGroups, viewSettings.strategyOrder]);
  const visibleNodeGroups = filteredNodeGroups;
  const visibleSubscriptions = filteredSubscriptions;
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
  };

  const updateOrder = (order: string[]) => {
    saveViewSettings({ ...viewSettings, strategyOrder: order });
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
      await load();
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
      await load();
      const failed = results.filter(result => result.status === "rejected").length;
      onToast(failed ? `延迟测试完成，${failed} 个节点失败` : "延迟测试完成");
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "延迟测试失败");
    } finally {
      setBusy("");
    }
  };

  const testNode = async (name: string) => {
    setBusy(`node:${name}`);
    try {
      await api.testProxy(name, testUrl, timeout);
      await load();
      onToast(`${name} 延迟测试完成`);
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "延迟测试失败");
    } finally {
      setBusy("");
    }
  };

  const testSubscription = async (subscription: Subscription, nodes: ControllerProxy[]) => {
    if (!nodes.length) return;
    setBusy(`test-subscription:${subscription.id}`);
    try {
      const results = await Promise.allSettled(nodes.map(node => api.testProxy(node.name, testUrl, timeout)));
      await load();
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
  const collapseAll = () => setCollapsed(current => ({
    ...current,
    ...Object.fromEntries(currentItems.map(item => [proxyCardKey(tab, item.id), true])),
  }));
  const moveItem = (source: string, target: string) => {
    if (source === target) return;
    updateOrder(reorderProxyViewIds(strategyOrder, source, target));
  };

  return <>
  <main className="page proxies-page" style={pageStyle}>
    <div className="proxy-controls">
      <SegmentedGroup className="proxy-tabs" value={tab} onValueChange={changeTab} aria-label="代理视图">
        <SegmentedItem value="groups">策略 <span>({customGroups.length})</span></SegmentedItem>
        <SegmentedItem value="nodes">节点 <span>({nodeGroups.length})</span></SegmentedItem>
        <SegmentedItem value="subscriptions">订阅 <span>({subscriptions.length})</span></SegmentedItem>
      </SegmentedGroup>
      <SearchInputGroup label="搜索代理" value={query} onChange={setQuery} onClear={() => setQuery("")} placeholder="搜索 | 多个关键词用空格分隔" />
      <div className="proxy-tools">
        <ProxyTool label="策略设置" onClick={() => setSettingsOpen(true)}><WrenchScrewdriverIcon /></ProxyTool>
        <ProxyTool label="收起全部卡片" disabled={!currentItems.length} onClick={collapseAll}><ChevronUpIcon /></ProxyTool>
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
      nodeSort={viewSettings.nodeSort}
      groupByProvider={viewSettings.groupByProvider}
    />)}</div>}

    {tab === "subscriptions" && <div className="policy-list">{visibleSubscriptions.map(subscription => {
      const options = sortProxyNodes(subscriptionNodes(subscription, entries).filter(option => isVisibleProxy(option, hideUnavailable)), viewSettings.nodeSort);
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
              <h2>{subscription.name}</h2><span>({tested}/{options.length || subscription.nodeCount})</span>
              <small>更新于 {relativeTime(subscription.updatedAt)}</small>
              {isCollapsed && <NodeHealthDots options={options} thresholds={thresholds} />}
            </div>
          </button>
          <div className="provider-actions">
            <IconButton label={`测试 ${subscription.name} 节点延迟`} disabled={Boolean(busy) || !options.length} onClick={() => void testSubscription(subscription, options)}><BoltIcon className={isTesting ? "testing-bolt" : undefined} /></IconButton>
            <IconButton label={`刷新 ${subscription.name}`} disabled={Boolean(busy)} onClick={() => void refreshSubscription(subscription)}><ArrowPathIcon /></IconButton>
            <IconButton label={`修改 ${subscription.name}`} onClick={() => { sessionStorage.setItem("openbox:settings-section", "subscriptions"); window.location.hash = "#/settings"; }}><PencilSquareIcon /></IconButton>
          </div>
        </header>
        {!isCollapsed && <div className="policy-options provider-options">{options.map(option => <ProxyOption
          key={option.name}
          option={option}
          active={false}
          disabled={Boolean(busy)}
          thresholds={thresholds}
          testing={isTesting || busy === `node:${option.name}`}
          onTest={() => void testNode(option.name)}
        />)}</div>}
      </article>;
    })}</div>}
  </main>;
  {settingsOpen && <Modal title="策略设置" className="proxy-settings-modal" onClose={() => setSettingsOpen(false)}>
    <div className="proxy-settings">
      <label className="proxy-setting-row"><span>节点排序方式</span><select aria-label="节点排序方式" value={viewSettings.nodeSort} onChange={event => saveViewSettings({ ...viewSettings, nodeSort: event.target.value as ProxyNodeSortMode })}>
        <option value="default">按配置排序</option>
        <option value="nameAsc">按名称升序</option>
        <option value="nameDesc">按名称降序</option>
        <option value="latencyAsc">按延迟升序</option>
        <option value="latencyDesc">按延迟降序</option>
      </select></label>
      <label className="proxy-setting-row"><span>节点根据提供商分组</span><input className="proxy-setting-toggle" type="checkbox" checked={viewSettings.groupByProvider} onChange={event => saveViewSettings({ ...viewSettings, groupByProvider: event.target.checked })} /></label>
      <label className="proxy-setting-row"><span>节点卡片最小宽度</span><span className="proxy-width-control"><input aria-label="节点卡片最小宽度" type="number" min="100" max="320" value={viewSettings.nodeCardMinWidth} onChange={event => saveViewSettings({ ...viewSettings, nodeCardMinWidth: clampNodeCardWidth(event.target.value) })} /><button type="button" onClick={() => saveViewSettings({ ...viewSettings, nodeCardMinWidth: 145 })}>重置</button></span></label>
      <div className="proxy-settings-divider" />
      <div className="proxy-settings-columns">
        <section className="proxy-settings-column">
          <h3>显示排序</h3>
          <div className="proxy-order-list">{strategyOrder.map(id => {
            const item = strategyItems.find(candidate => candidate.id === id);
            if (!item) return null;
            return <div
              className={`proxy-order-item${draggedItem === id ? " dragging" : ""}`}
              draggable
              key={id}
              onDragStart={event => { setDraggedItem(id); event.dataTransfer.effectAllowed = "move"; event.dataTransfer.setData("text/plain", id); }}
              onDragEnd={() => setDraggedItem(null)}
              onDragOver={event => { event.preventDefault(); event.dataTransfer.dropEffect = "move"; }}
              onDrop={event => { event.preventDefault(); moveItem(event.dataTransfer.getData("text/plain") || draggedItem || "", id); setDraggedItem(null); }}
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

export function ProxyGroupCard({ group, data, collapsed, busy, onToggle, onSelect, onTest, onTestNode, variant, hideUnavailable, thresholds, history, latencyHistory, testingGroup, nodeSort = "latencyAsc", groupByProvider = true, penetrationExpanded = false, onTogglePenetration }: {
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
  nodeSort?: ProxyNodeSortMode;
  groupByProvider?: boolean;
  penetrationExpanded?: boolean;
  onTogglePenetration?: () => void;
}) {
  const visibleOptions = ((group.all ?? []).map(name => data?.proxies[name]).filter(Boolean) as ControllerProxy[])
    .filter(option => isVisibleProxy(option, hideUnavailable));
  const options = sortProxyNodes(visibleOptions, nodeSort);
  const tested = options.filter(option => latestLatency(option.history)).length;
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
      <div><h2>{group.name}</h2>{variant === "policy" && <span>域名穿透</span>}<small>{group.type} ({tested}/{options.length})</small></div>
      <p>{variant === "policy" && <img src={optionIcons[group.now ?? ""] ?? otherGroupIcon} alt="" />}{group.now ?? "未选择"}{current?.now && <><b>›</b>{current.now}</>}</p>
    </div>
  </>;
  return <article className={`surface policy-card${nodeClass}${collapsedClass}`}>
    <div className="policy-head">
      <button type="button" className="node-group-header-toggle" aria-label={`${collapsed ? "展开" : "收起"} ${group.name}`} aria-expanded={!collapsed} onClick={onToggle}>{headerContent}</button>
      <PolicyLatency groupName={group.name} history={history.length ? history : current?.history ?? []} delay={selectedDelay} thresholds={thresholds} busy={busy} testing={testing} onTest={onTest} />
      {variant === "nodes" && group.name === "所有-自动" && <small className="group-meta">检测间隔 300 秒 · 容差 100 毫秒</small>}
    </div>
    {variant === "nodes" && <div className={`node-health-summary${nodesCollapsed ? " visible" : ""}`} aria-hidden={!nodesCollapsed}>
      <div className="node-health-summary-inner"><NodeHealthDots options={options} selectedName={group.now} thresholds={thresholds} /></div>
    </div>}
    {variant === "policy" && !collapsed && <div className="policy-options">{options.map(option => <ProxyOption
      key={option.name}
      option={option}
      active={option.name === group.now}
      disabled={busy}
      thresholds={thresholds}
      onClick={() => void onSelect(group.name, option.name)}
    />)}</div>}
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
            disabled={busy}
            thresholds={thresholds}
            showIcon={false}
            onClick={proxyOptionAction(group.type) === "select" ? () => void onSelect(group.name, option.name) : undefined}
            onTest={() => void onTestNode(option.name)}
          />)}</div>
      </div>
    </section>}
    {variant === "policy" && !collapsed && penetration && <>
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
        nodeSort={nodeSort}
      />}
    </>}
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

export function PenetrationGroup({ group, data, busy, onSelect, onTest, onTestNode, hideUnavailable, thresholds, history, testing, nodeSort = "latencyAsc" }: {
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
  nodeSort?: ProxyNodeSortMode;
}) {
  const options = sortProxyNodes(((group.all ?? []).map(name => data?.proxies[name]).filter(Boolean) as ControllerProxy[])
    .filter(option => isVisibleProxy(option, hideUnavailable)), nodeSort);
  const tested = options.filter(option => latestLatency(option.history)).length;
  const selectedDelay = selectedProxyDelay(group, data);
  return <section className="policy-penetration" aria-label={`${group.name} 节点列表`}>
    <div className="penetration-head">
      <img className="penetration-icon" src={groupIcon(group.name)} alt="" />
      <div className="penetration-summary">
        <div><h3>{group.name}</h3><small>{group.type} ({tested}/{options.length})</small></div>
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
      disabled={busy}
      thresholds={thresholds}
      showIcon={false}
      onClick={proxyOptionAction(group.type) === "select" ? () => void onSelect(group.name, option.name) : undefined}
      onTest={() => void onTestNode(option.name)}
    />)}</div>
  </section>;
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

export function ProxyOption({ option, active, disabled, thresholds, showIcon = true, testing = false, onClick, onTest }: {
  option: ControllerProxy;
  active: boolean;
  disabled: boolean;
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
      aria-busy={testing || undefined}
      disabled={disabled}
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

function groupMatches(group: ControllerProxy, data: ProxiesResponse | null, query: string) {
  if (!query.trim()) return true;
  const options = (group.all ?? []).map(name => data?.proxies[name]).filter(Boolean) as ControllerProxy[];
  return matchesQuery([group.name, group.type, group.now, ...options.flatMap(option => [option.name, option.type])], query);
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

function groupIcon(name: string) {
  if (name === "所有-自动") return autoOptionIcon;
  if (name === "所有-手动") return manualOptionIcon;
  return groupIcons[name] ?? otherGroupIcon;
}

function proxyType(value: string) {
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

function finiteSetting(value: string | undefined, fallback: number) {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : fallback;
}

function relativeTime(value: string) {
  const timestamp = new Date(value).getTime();
  if (!Number.isFinite(timestamp)) return value;
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

function subscriptionLatency(subscription: Subscription, entries: ControllerProxy[]) {
  const measured = subscriptionNodes(subscription, entries).map(node => latestLatency(node.history)).filter((delay): delay is number => delay != null);
  return measured.length ? Math.min(...measured) : null;
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
