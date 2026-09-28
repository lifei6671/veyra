import { useEffect, useMemo, useState } from "react";
import { ArrowPathIcon, BoltIcon, ChevronUpIcon } from "@heroicons/react/24/outline";
import { api } from "../api/client";
import type { ControllerProxy, ProxiesResponse, StorageResponse, Subscription } from "../api/types";
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
import { latestLatency, matchesQuery, formatDateTime } from "../lib/format";
import { SearchInputGroup, SegmentedGroup, SegmentedItem } from "../ui/controls";
import { EmptyState, ErrorState, IconButton, PageToolbar } from "../components/shared";

const groupIcons: Record<string, string> = {
  Speed: speedGroupIcon, AI: aiGroupIcon, Youtube: youtubeGroupIcon, TikTok: tiktokGroupIcon,
  Netflix: netflixGroupIcon, Github: githubGroupIcon, Google: googleGroupIcon, Microsoft: microsoftGroupIcon,
  Apple: appleGroupIcon, Games: gamesGroupIcon, "国外": globalGroupIcon, "国内": chinaGroupIcon, "其他": otherGroupIcon,
};

const optionIcons: Record<string, string> = { "直连": directOptionIcon, "所有-自动": autoOptionIcon, "所有-手动": manualOptionIcon, "拒绝": rejectOptionIcon };

type ViewTab = "groups" | "nodes" | "subscriptions";

export function ProxiesPage({ storage, onToast }: { storage: StorageResponse | null; onToast: (message: string) => void }) {
  const [data, setData] = useState<ProxiesResponse | null>(null);
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [tab, setTab] = useState<ViewTab>("groups");
  const [query, setQuery] = useState("");
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState("");
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  const load = async () => {
    setError(null);
    try {
      const [proxies, subscriptionItems] = await Promise.all([api.proxies(), api.subscriptions()]);
      setData(proxies);
      setSubscriptions(subscriptionItems);
    } catch (reason) {
      setError(reason);
    }
  };
  useEffect(() => { void load(); }, []);

  const entries = Object.values(data?.proxies ?? {});
  const groups = useMemo(() => entries.filter(proxy => proxy.type === "Selector" && !["GLOBAL", "所有-手动"].includes(proxy.name) && matchesQuery([proxy.name, proxy.now], query)), [entries, query]);
  const nodes = useMemo(() => entries.filter(proxy => !proxy.all?.length && !["Direct", "Reject"].includes(proxy.type) && matchesQuery([proxy.name, proxy.type], query)), [entries, query]);
  const timeout = Number(storage?.entries["config/speedtest-timeout"] ?? 5000);
  const testUrl = storage?.entries["config/speedtest-url"] ?? "http://www.gstatic.com/generate_204";

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

  const test = async (group: string) => {
    setBusy(`test:${group}`);
    try {
      await api.testProxyGroup(group, testUrl, timeout);
      await load();
      onToast(`${group} 延迟测试完成`);
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "延迟测试失败");
    } finally {
      setBusy("");
    }
  };

  return <main className="page proxies-page">
    <PageToolbar title="代理" subtitle={data ? `${groups.length} 个策略组 · ${nodes.length} 个节点` : "正在读取代理数据"}>
      <IconButton label="刷新代理" onClick={() => void load()}><ArrowPathIcon /></IconButton>
    </PageToolbar>
    <div className="proxy-controls"><SegmentedGroup className="proxy-tabs" value={tab} onValueChange={value => setTab(value as ViewTab)} aria-label="代理视图"><SegmentedItem value="groups">策略 <span>({groups.length})</span></SegmentedItem><SegmentedItem value="nodes">节点 <span>({nodes.length})</span></SegmentedItem><SegmentedItem value="subscriptions">订阅 <span>({subscriptions.length})</span></SegmentedItem></SegmentedGroup><SearchInputGroup label="搜索代理" value={query} onChange={setQuery} onClear={() => setQuery("")} placeholder="搜索 | 多个关键词用空格分隔" /><div className="proxy-tools"><IconButton label="回到顶部" onClick={() => document.querySelector(".page")?.scrollTo({ top: 0, behavior: "smooth" })}><ChevronUpIcon /></IconButton><IconButton label="测试全部策略组" disabled={Boolean(busy)} onClick={() => void Promise.all(groups.map(group => test(group.name)))}><BoltIcon /></IconButton></div></div>
    {Boolean(error) && <ErrorState title="代理数据加载失败" error={error} onRetry={() => void load()} />}

    {tab === "groups" && <div className="policy-list">{groups.map(group => {
      const options = (group.all ?? []).map(name => data?.proxies[name]).filter(Boolean) as ControllerProxy[];
      const isCollapsed = collapsed[group.name];
      return <article className="surface policy-group" key={group.name}>
        <div className="policy-head"><button type="button" className="policy-collapse" aria-label={`${isCollapsed ? "展开" : "收起"} ${group.name}`} onClick={() => setCollapsed(value => ({ ...value, [group.name]: !value[group.name] }))}><img className="policy-icon" src={groupIcons[group.name] ?? otherGroupIcon} alt="" /></button><div className="policy-summary"><div><h2>{group.name}</h2><span>{group.type}</span><small>{group.now ? `1/${group.all?.length ?? 0}` : group.all?.length}</small></div><p>🌐 {group.now ?? "未选择"}</p></div><IconButton label={`测试 ${group.name} 延迟`} disabled={Boolean(busy)} onClick={() => void test(group.name)}><BoltIcon /></IconButton></div>
        {!isCollapsed && <div className="policy-options">{options.map(option => {
          const active = option.name === group.now;
          const delay = latestLatency(option.history);
          return <button type="button" className={`policy-option${active ? " active" : ""}`} key={option.name} disabled={Boolean(busy)} onClick={() => void select(group.name, option.name)}><img src={optionIcons[option.name] ?? groupIcons[option.name] ?? otherGroupIcon} alt="" /><strong>{option.name}</strong><span>{option.type}</span><em className={delay && delay > 1000 ? "danger" : delay && delay > 500 ? "warning" : "success"}>{delay ? `${delay} ms` : "—"}</em></button>;
        })}</div>}
      </article>;
    })}</div>}

    {tab === "nodes" && <div className="node-grid">{nodes.map(node => <article className="surface node-card" key={node.name}><img src={groupIcons[node.name] ?? otherGroupIcon} alt="" /><div><h2>{node.name}</h2><p>{node.type}{node.udp ? " / udp" : ""}</p></div><strong>{latestLatency(node.history) ? `${latestLatency(node.history)} ms` : "未测速"}</strong></article>)}</div>}

    {tab === "subscriptions" && <div className="subscription-grid">{subscriptions.map(subscription => <article className="surface subscription-card" key={subscription.id}><div><h2>{subscription.name}</h2><p>{subscription.format} · {subscription.nodeCount} 个节点</p></div><span className={subscription.kernelStale ? "warning" : "success"}>{subscription.kernelStale ? "等待内核应用" : "已生效"}</span><small>更新于 {formatDateTime(subscription.updatedAt)}</small></article>)}{!subscriptions.length && <EmptyState icon="↻" title="还没有订阅" text="可在设置的订阅管理中添加真实订阅。" />}</div>}
  </main>;
}
