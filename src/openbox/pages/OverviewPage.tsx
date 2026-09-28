import { useEffect, useMemo, useState } from "react";
import { ArrowPathIcon, PauseIcon, PlayIcon } from "@heroicons/react/24/outline";
import { api } from "../api/client";
import type { SiteLatencyHistory, StorageResponse, TestSite, TrafficDay, TrafficMonth, TrafficRow } from "../api/types";
import baiduLogo from "../assets/baidu.svg";
import googleLogo from "../assets/google.svg";
import openaiLogo from "../assets/openai.svg";
import telegramLogo from "../assets/telegram.svg";
import { formatBytes, getCurrentDay, getCurrentMonth, latestLatency, matchesQuery } from "../lib/format";
import { SearchInputGroup, SegmentedGroup, SegmentedItem, SwitchControl, SwitchLabel } from "../ui/controls";
import type { ShellStats } from "../components/AppShell";
import { ErrorState, IconButton, PageToolbar } from "../components/shared";

const defaultSites: TestSite[] = [
  { id: "baidu", icon: baiduLogo, name: "百度", url: "https://www.baidu.com/favicon.ico" },
  { id: "google", icon: googleLogo, name: "Google", url: "https://www.google.com/generate_204" },
  { id: "openai", icon: openaiLogo, name: "OpenAI", url: "https://api.openai.com/v1/models" },
  { id: "telegram", icon: telegramLogo, name: "Telegram", url: "https://telegram.org/favicon.ico" },
];

type TrafficTab = "clients" | "nodes" | "hosts";

export function OverviewPage({ stats, storage, onToast }: { stats: ShellStats; storage: StorageResponse | null; onToast: (message: string) => void }) {
  const [latency, setLatency] = useState<SiteLatencyHistory | null>(null);
  const [month, setMonth] = useState<TrafficMonth | null>(null);
  const [day, setDay] = useState<TrafficDay | null>(null);
  const [trafficTab, setTrafficTab] = useState<TrafficTab>("clients");
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [testing, setTesting] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [chartsPaused, setChartsPaused] = useState(false);
  const [chartStats, setChartStats] = useState(stats);
  const sites = useMemo(() => parseSites(storage), [storage]);
  const [autoCheck, setAutoCheck] = useState(storage?.entries["config/auto-connection-check"] !== "false");
  const [countDirect, setCountDirect] = useState(storage?.entries["config/traffic-count-direct"] !== "false");

  useEffect(() => {
    setAutoCheck(storage?.entries["config/auto-connection-check"] !== "false");
    setCountDirect(storage?.entries["config/traffic-count-direct"] !== "false");
  }, [storage]);

  useEffect(() => {
    if (!chartsPaused) setChartStats(stats);
  }, [chartsPaused, stats]);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const [latencyValue, monthValue, dayValue] = await Promise.all([
        api.siteLatencyHistory(),
        api.trafficMonth(getCurrentMonth()),
        api.trafficDay(getCurrentDay()),
      ]);
      setLatency(latencyValue);
      setMonth(monthValue);
      setDay(dayValue);
    } catch (reason) {
      setError(reason);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { void load(); }, []);

  const runSiteTest = async () => {
    setTesting(true);
    try {
      await api.testSites(sites.map(({ id, name, url }) => ({ id, name, url, icon: "" })));
      setLatency(await api.siteLatencyHistory());
      onToast("四个站点延迟已重新测试");
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "测速失败");
    } finally {
      setTesting(false);
    }
  };

  const patchBoolean = async (key: string, value: boolean, setValue: (value: boolean) => void) => {
    setValue(value);
    try {
      await api.patchStorage({ [key]: String(value) });
      onToast("设置已保存到后端");
    } catch (reason) {
      setValue(!value);
      onToast(reason instanceof Error ? reason.message : "设置保存失败");
    }
  };

  const rows = useMemo(() => {
    const source = trafficTab === "clients" ? day?.clients : trafficTab === "nodes" ? day?.nodes : day?.hosts;
    return (source ?? []).filter(row => matchesQuery([row.key, row.name], query));
  }, [day, query, trafficTab]);
  const maxTraffic = Math.max(1, ...rows.map(row => row.up + row.down));

  return <main className="page overview-page">
    <PageToolbar title="面板概览" subtitle={loading ? "正在同步实时数据" : "数据来自 Open-Box 后端"}>
      <SwitchLabel label="打开概览自动测速"><SwitchControl label="打开概览自动测速" checked={autoCheck} onCheckedChange={value => void patchBoolean("config/auto-connection-check", value, setAutoCheck)} /></SwitchLabel>
      <button type="button" className="compact-button" disabled={testing} onClick={() => void runSiteTest()}><ArrowPathIcon />{testing ? "测速中" : "重测这四个站点"}</button>
    </PageToolbar>

    {Boolean(error) && <ErrorState title="概览数据加载失败" error={error} onRetry={() => void load()} />}

    <section className="latency-sites" aria-label="站点延迟">{sites.map(site => {
      const delay = latestLatency(latency?.history[site.id]);
      return <button type="button" key={site.id} disabled={testing} onClick={() => void runSiteTest()}><img src={site.icon} alt="" /><span>{site.name}</span><strong className={!delay ? "muted" : delay > 1000 ? "danger" : delay > 500 ? "warning" : "success"}>{delay ?? "—"}</strong><small>{delay ? "ms" : ""}</small></button>;
    })}</section>

    <section className="overview-metrics surface">
      <OverviewMetric label="连接数" value={String(chartStats.connections)} values={[2, 3, 4, 4, chartStats.connections]} paused={chartsPaused} />
      <OverviewMetric label="内存使用" value={formatBytes(chartStats.memory)} values={[52, 55, 54, 58, Math.max(1, chartStats.memory / 1_000_000)]} paused={chartsPaused} />
      <OverviewMetric label="进站流量" value={formatBytes(chartStats.inbound)} values={[18, 32, 22, 48, Math.max(1, chartStats.inboundRate / 80)]} paused={chartsPaused} />
      <OverviewMetric label="出站流量" value={formatBytes(chartStats.outbound)} values={[12, 24, 18, 33, Math.max(1, chartStats.outboundRate / 80)]} paused={chartsPaused} />
      <IconButton label={chartsPaused ? "继续刷新" : "暂停刷新"} onClick={() => setChartsPaused(value => !value)}>{chartsPaused ? <PlayIcon /> : <PauseIcon />}</IconButton>
    </section>

    <section className="surface traffic-insight">
      <header><div><h2>流量洞察</h2><p>{month ? `${month.month.replace("-", "年")}月 共 ${formatBytes(month.total.down + month.total.up)}，${month.total.conns} 个连接。` : "正在读取本月数据"}</p></div><SwitchLabel label="统计直连流量"><SwitchControl label="统计直连流量" checked={countDirect} onCheckedChange={value => void patchBoolean("config/traffic-count-direct", value, setCountDirect)} /></SwitchLabel></header>
      <MonthBars data={month} />
      <div className="traffic-summary"><span><i className="in" />进站 {formatBytes(day?.total.down)}</span><span><i className="out" />出站 {formatBytes(day?.total.up)}</span><strong>{day?.day ?? "—"} · 总流量 {formatBytes((day?.total.down ?? 0) + (day?.total.up ?? 0))}</strong></div>
      <div className="insight-controls"><SegmentedGroup value={trafficTab} onValueChange={value => setTrafficTab(value as TrafficTab)} aria-label="流量统计维度"><SegmentedItem value="clients">终端设备 ({day?.clients.length ?? 0})</SegmentedItem><SegmentedItem value="nodes">出站节点 ({day?.nodes.length ?? 0})</SegmentedItem><SegmentedItem value="hosts">访问目标 ({day?.hosts.length ?? 0})</SegmentedItem></SegmentedGroup><SearchInputGroup label="搜索流量记录" value={query} onChange={setQuery} onClear={() => setQuery("")} placeholder="搜索名称" /></div>
      <div className="insight-table" role="table"><div className="insight-head" role="row"><span>名称</span><span>进站</span><span>出站</span><span>总流量</span><span>占比</span></div>{rows.map(row => <TrafficTableRow key={row.key} row={row} maximum={maxTraffic} />)}</div>
    </section>
  </main>;
}

function parseSites(storage: StorageResponse | null) {
  const raw = storage?.entries["config/test-sites"];
  if (!raw) return defaultSites;
  try {
    const parsed = JSON.parse(raw) as Array<Omit<TestSite, "icon"> & { icon?: string }>;
    return parsed.map(site => ({ ...site, icon: defaultSites.find(item => item.id === site.id)?.icon ?? defaultSites[0].icon }));
  } catch {
    return defaultSites;
  }
}

function OverviewMetric({ label, value, values, paused }: { label: string; value: string; values: number[]; paused: boolean }) {
  return <article><header><span>{label}</span><strong>{value}</strong></header><MiniChart values={values} paused={paused} /></article>;
}

function MiniChart({ values, paused }: { values: number[]; paused: boolean }) {
  const maximum = Math.max(1, ...values);
  const points = values.map((value, index) => `${index * 25},${48 - (value / maximum) * 40}`).join(" ");
  return <svg viewBox="0 0 100 52" preserveAspectRatio="none" aria-hidden="true" className={paused ? "paused" : ""}><polyline points={points} fill="none" stroke="currentColor" strokeWidth="2.5" vectorEffect="non-scaling-stroke" /></svg>;
}

function MonthBars({ data }: { data: TrafficMonth | null }) {
  const maximum = Math.max(1, ...(data?.days.map(item => item.up + item.down) ?? [1]));
  return <div className="month-bars" aria-label="本月每日流量">{(data?.days ?? []).map(item => <div key={item.day} title={`${item.day} ${formatBytes(item.up + item.down)}`}><i style={{ height: `${Math.max(2, ((item.up + item.down) / maximum) * 100)}%` }} /><span>{Number(item.day.slice(-2))}</span></div>)}</div>;
}

function TrafficTableRow({ row, maximum }: { row: TrafficRow; maximum: number }) {
  const total = row.up + row.down;
  const percentage = Math.max(1, Math.round((total / maximum) * 100));
  return <div className="insight-row" role="row"><div><b>{row.key}</b>{(row.name || row.self?.iface) && <span>{row.name || `本机 · ${row.self?.iface}`}</span>}</div><span>{formatBytes(row.down)}</span><span>{formatBytes(row.up)}</span><strong>{formatBytes(total)}</strong><div className="share-cell"><div className="usage-bar"><i style={{ width: `${percentage}%` }} /></div><em>{percentage}%</em></div></div>;
}
