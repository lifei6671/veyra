import { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowPathIcon,
  ChevronDownIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  PauseIcon,
  PlayIcon,
} from "@heroicons/react/24/outline";
import { api } from "../api/client";
import type {
  SiteLatencyHistory,
  SiteLatencyPoint,
  StorageResponse,
  TestSite,
  TrafficDay,
  TrafficDimension,
  TrafficDrill,
  TrafficMonth,
  TrafficRow,
} from "../api/types";
import baiduLogo from "../assets/baidu.svg";
import googleLogo from "../assets/google.svg";
import openaiLogo from "../assets/openai.svg";
import telegramLogo from "../assets/telegram.svg";
import { formatBytes, formatRate, getCurrentDay, getCurrentMonth, matchesQuery } from "../lib/format";
import { SearchInputGroup, SegmentedGroup, SegmentedItem, SwitchControl, SwitchLabel } from "../ui/controls";
import type { ShellStats } from "../components/AppShell";
import { ErrorState, IconButton, PromoBar } from "../components/shared";

const defaultSites: TestSite[] = [
  { id: "baidu", icon: baiduLogo, name: "百度", url: "https://www.baidu.com/favicon.ico" },
  { id: "google", icon: googleLogo, name: "Google", url: "https://www.google.com/generate_204" },
  { id: "openai", icon: openaiLogo, name: "OpenAI", url: "https://api.openai.com/v1/models" },
  { id: "telegram", icon: telegramLogo, name: "Telegram", url: "https://telegram.org/favicon.ico" },
];

type TrafficTab = "clients" | "nodes" | "hosts";
type MetricKey = "inboundRate" | "outboundRate" | "memory" | "connections";
type MetricPoint = { value: number; time: number };
type MetricHistory = Record<MetricKey, MetricPoint[]>;
const metricWindowSize = 60;

const trafficDimensions: Record<TrafficTab, TrafficDimension> = { clients: "client", nodes: "node", hosts: "host" };
const drillDimensions: Record<TrafficTab, Array<{ value: TrafficDimension; label: string }>> = {
  clients: [{ value: "host", label: "访问目标" }, { value: "node", label: "出站节点" }],
  nodes: [{ value: "host", label: "访问目标" }, { value: "client", label: "终端设备" }],
  hosts: [{ value: "client", label: "终端设备" }, { value: "node", label: "出站节点" }],
};

export function OverviewPage({ stats, storage, onToast }: { stats: ShellStats; storage: StorageResponse | null; onToast: (message: string) => void }) {
  const [latency, setLatency] = useState<SiteLatencyHistory | null>(null);
  const [month, setMonth] = useState<TrafficMonth | null>(null);
  const [day, setDay] = useState<TrafficDay | null>(null);
  const [selectedMonth, setSelectedMonth] = useState(getCurrentMonth());
  const [selectedDay, setSelectedDay] = useState(getCurrentDay());
  const [selectedHour, setSelectedHour] = useState<number | null>(null);
  const [trafficTab, setTrafficTab] = useState<TrafficTab>("clients");
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [reloadToken, setReloadToken] = useState(0);
  const [testingSite, setTestingSite] = useState<string | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [paused, setPaused] = useState<Record<MetricKey, boolean>>({ inboundRate: false, outboundRate: false, memory: false, connections: false });
  const [history, setHistory] = useState<MetricHistory>(() => initialMetricHistory(stats));
  const [expandedKey, setExpandedKey] = useState<string | null>(null);
  const [drillBy, setDrillBy] = useState<TrafficDimension>("host");
  const [drillCache, setDrillCache] = useState<Partial<Record<TrafficDimension, TrafficDrill>>>({});
  const [drillLoading, setDrillLoading] = useState(false);
  const autoRan = useRef(false);
  const drillRequests = useRef(createLatestRequestGuard());
  const sites = useMemo(() => parseSites(storage), [storage]);
  const [autoCheck, setAutoCheck] = useState(storage?.entries["config/auto-connection-check"] !== "false");
  const [countDirect, setCountDirect] = useState(storage?.entries["config/traffic-count-direct"] !== "false");
  const lowLatency = Number(storage?.entries["config/low-latency"] ?? 500);
  const mediumLatency = Number(storage?.entries["config/medium-latency"] ?? 1000);

  useEffect(() => {
    setAutoCheck(storage?.entries["config/auto-connection-check"] !== "false");
    setCountDirect(storage?.entries["config/traffic-count-direct"] !== "false");
  }, [storage]);

  useEffect(() => {
    setHistory(previous => ({
      inboundRate: paused.inboundRate ? previous.inboundRate : appendMetric(previous.inboundRate, stats.inboundRate),
      outboundRate: paused.outboundRate ? previous.outboundRate : appendMetric(previous.outboundRate, stats.outboundRate),
      memory: paused.memory ? previous.memory : appendMetric(previous.memory, stats.memory),
      connections: paused.connections ? previous.connections : appendMetric(previous.connections, stats.connections),
    }));
  }, [paused, stats]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    void Promise.all([api.siteLatencyHistory(), api.trafficMonth(selectedMonth, countDirect)])
      .then(([latencyValue, monthValue]) => {
        if (cancelled) return;
        setLatency(latencyValue);
        setMonth(monthValue);
      })
      .catch(reason => { if (!cancelled) setError(reason); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [selectedMonth, countDirect, reloadToken]);

  useEffect(() => {
    let cancelled = false;
    drillRequests.current.invalidate();
    setExpandedKey(null);
    setDrillCache({});
    setDrillLoading(false);
    void api.trafficDay(selectedDay, selectedHour, countDirect)
      .then(value => { if (!cancelled) setDay(value); })
      .catch(reason => { if (!cancelled) setError(reason); });
    return () => { cancelled = true; };
  }, [selectedDay, selectedHour, countDirect, reloadToken]);

  useEffect(() => {
    if (!autoCheck || autoRan.current || !sites.length) return;
    autoRan.current = true;
    void runSiteTest(sites);
  }, [autoCheck, sites]);

  const runSiteTest = async (targets: TestSite[]) => {
    setTestingSite(targets.length === sites.length ? "all" : targets[0]?.id ?? "all");
    try {
      await api.testSites(targets.map(({ id, name, url }) => ({ id, name, url, icon: "" })));
      setLatency(await api.siteLatencyHistory());
      onToast(targets.length === 1 ? `${targets[0]?.name ?? "站点"}延迟已更新` : "四个站点延迟已重新测试");
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "测速失败");
    } finally {
      setTestingSite(null);
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

  const chooseMonth = (value: string) => {
    if (!isMonthInHistoryRange(value)) return;
    setSelectedMonth(value);
    setSelectedDay(value === getCurrentMonth() ? getCurrentDay() : lastDayOfMonth(value));
    setSelectedHour(null);
  };

  const rows = useMemo(() => {
    const source = trafficTab === "clients" ? day?.clients : trafficTab === "nodes" ? day?.nodes : day?.hosts;
    return (source ?? []).filter(row => matchesQuery([row.key, row.name], query));
  }, [day, query, trafficTab]);
  const rowTotal = (day?.total.up ?? 0) + (day?.total.down ?? 0);
  const dimensions = drillDimensions[trafficTab];
  const drill = drillCache[drillBy] ?? null;

  const toggleDrill = async (key: string) => {
    if (expandedKey === key) {
      drillRequests.current.invalidate();
      setExpandedKey(null);
      setDrillCache({});
      setDrillLoading(false);
      return;
    }
    const by = dimensions[0].value;
    setExpandedKey(key);
    setDrillBy(by);
    setDrillCache({});
    setDrillLoading(true);
    const requestId = drillRequests.current.issue();
    try {
      const results = await Promise.all(dimensions.map(item => api.trafficDrill(selectedDay, trafficDimensions[trafficTab], key, item.value, selectedHour, countDirect)));
      if (drillRequests.current.isCurrent(requestId)) {
        setDrillCache(Object.fromEntries(results.map(result => [result.by, result])));
      }
    } catch (reason) {
      if (drillRequests.current.isCurrent(requestId)) onToast(reason instanceof Error ? reason.message : "流量明细加载失败");
    } finally {
      if (drillRequests.current.isCurrent(requestId)) setDrillLoading(false);
    }
  };

  const changeTrafficTab = (value: TrafficTab) => {
    drillRequests.current.invalidate();
    setTrafficTab(value);
    setExpandedKey(null);
    setDrillCache({});
    setDrillLoading(false);
    setDrillBy(drillDimensions[value][0].value);
  };

  return <>
    <PromoBar />
    <main className="page overview-page">
      {Boolean(error) && <ErrorState title="概览数据加载失败" error={error} onRetry={() => setReloadToken(value => value + 1)} />}

      <section className="surface overview-panel">
        <header className="section-heading">
          <div className="overview-title">
            <h2>面板概览</h2>
            <SwitchLabel label="打开概览自动测速">
              <span title="打开概览页时自动测一遍这四个站点的延时。和面板设置里的「自动检查连接」是同一个开关。">
                <SwitchControl label="打开概览自动测速" checked={autoCheck} onCheckedChange={value => void patchBoolean("config/auto-connection-check", value, setAutoCheck)} />
              </span>
            </SwitchLabel>
          </div>
          <IconButton label="重测这四个站点" disabled={testingSite !== null} onClick={() => void runSiteTest(sites)}>
            <ArrowPathIcon className={testingSite ? "spinning" : ""} />
          </IconButton>
        </header>

        <section className="latency-grid" aria-label="站点延迟">
          {sites.map(site => <LatencyCard
            key={site.id}
            site={site}
            points={latency?.history[site.id] ?? []}
            testing={testingSite === "all" || testingSite === site.id}
            lowLatency={lowLatency}
            mediumLatency={mediumLatency}
            onTest={() => void runSiteTest([site])}
          />)}
        </section>

        <section className="metric-row" aria-label="实时状态">
          <MetricValue label="连接数" value={String(stats.connections)} />
          <MetricValue label="内存使用" value={formatMemory(stats.memory)} />
          <MetricValue label="进站流量" value={formatBytes(stats.inbound, 1)} />
          <MetricValue label="进站速率" value={formatRate(stats.inboundRate)} />
          <MetricValue label="出站流量" value={formatBytes(stats.outbound, 2)} />
          <MetricValue label="出站速率" value={formatRate(stats.outboundRate)} />
        </section>

        <section className="sparkline-grid" aria-label="实时图表">
          <SparkCard label="进站速率" value={formatRate(stats.inboundRate)} points={history.inboundRate} formatValue={formatRate} paused={paused.inboundRate} onPause={() => setPaused(value => ({ ...value, inboundRate: !value.inboundRate }))} />
          <SparkCard label="出站速率" value={formatRate(stats.outboundRate)} points={history.outboundRate} formatValue={formatRate} paused={paused.outboundRate} color="blue" onPause={() => setPaused(value => ({ ...value, outboundRate: !value.outboundRate }))} />
          <SparkCard label="内存使用" value={formatMemory(stats.memory)} points={history.memory} formatValue={formatMemory} paused={paused.memory} filled onPause={() => setPaused(value => ({ ...value, memory: !value.memory }))} />
          <SparkCard label="连接数" value={String(stats.connections)} points={history.connections} formatValue={value => String(Math.round(value))} paused={paused.connections} onPause={() => setPaused(value => ({ ...value, connections: !value.connections }))} />
        </section>
      </section>

      <section className="surface traffic-panel">
        <header className="traffic-heading">
          <div>
            <div className="overview-title">
              <h2>流量洞察</h2>
              <SwitchLabel label="统计直连流量">
                <span title="关掉后不计入走直连出站的流量（库里的记录不变）；终端和站点互相下钻的构成里分不出直连那份，那里照旧。">
                  <SwitchControl label="统计直连流量" checked={countDirect} onCheckedChange={value => void patchBoolean("config/traffic-count-direct", value, setCountDirect)} />
                </span>
              </SwitchLabel>
            </div>
            <p>{month ? <>{formatMonthLabel(month.month)}共 <b>{formatBytes(month.total.down + month.total.up)}</b>,{month.total.conns} 个连接。</> : "正在读取本月数据"}</p>
          </div>
          <MonthPicker month={selectedMonth} onChange={chooseMonth} />
        </header>

        <div className="traffic-body">
          <MonthlyChart data={month} selectedDay={selectedDay} onSelectDay={value => { setSelectedDay(value); setSelectedHour(null); }} loading={loading} />
          <div className="chart-legend">
            <span><i />进站 {formatBytes(month?.total.down)}</span>
            <span><i className="out" />出站 {formatBytes(month?.total.up)}</span>
            <b title="按当前月份已过去的天数计算">i</b>
          </div>

          <div className="daily-heading">
            <strong>{day?.day ?? selectedDay}</strong><span>·</span><strong>总流量 {formatBytes((day?.total.down ?? 0) + (day?.total.up ?? 0))}</strong><small>{day?.total.conns ?? 0} 个连接</small>
            <button type="button" disabled={selectedHour === null} onClick={() => setSelectedHour(null)}>恢复看整天</button>
          </div>
          <DailyChart data={day} selectedHour={selectedHour} onSelectHour={setSelectedHour} />

          <div className="insight-toolbar">
            <SegmentedGroup className="segment-tabs" value={trafficTab} onValueChange={value => changeTrafficTab(value as TrafficTab)} aria-label="流量统计维度">
              <SegmentedItem value="clients">终端设备 <em>({day?.clientsCount ?? day?.clients.length ?? 0})</em></SegmentedItem>
              <SegmentedItem value="nodes">出站节点 <em>({day?.nodes.length ?? 0})</em></SegmentedItem>
              <SegmentedItem value="hosts">访问目标 <em>({day?.hostsCount ?? day?.hosts.length ?? 0})</em></SegmentedItem>
            </SegmentedGroup>
            <SearchInputGroup className="insight-search" label="搜索" value={query} onChange={setQuery} onClear={() => setQuery("")} placeholder="搜索" />
          </div>

          <div className="insight-list" role="table">
            <div className="insight-table-head" role="row"><span>名称</span><span>进站</span><span>出站</span><span>总流量</span><span>占比</span></div>
            {rows.map(row => <div className="traffic-row-group" key={row.key}>
              <TrafficTableRow row={row} total={rowTotal} expanded={expandedKey === row.key} onOpen={() => void toggleDrill(row.key)} />
              {expandedKey === row.key && <DrillTable
                dimensions={dimensions}
                selected={drillBy}
                loading={drillLoading}
                data={drill}
                dataByDimension={drillCache}
                onSelect={setDrillBy}
              />}
            </div>)}
          </div>
        </div>
      </section>
    </main>
  </>;
}

function LatencyCard({ site, points, testing, lowLatency, mediumLatency, onTest }: {
  site: TestSite;
  points: SiteLatencyPoint[];
  testing: boolean;
  lowLatency: number;
  mediumLatency: number;
  onTest: () => void;
}) {
  const [hovered, setHovered] = useState<number | null>(null);
  const signalRef = useRef<HTMLSpanElement>(null);
  const [signalWidth, setSignalWidth] = useState(0);
  const delay = currentSiteLatency(points);
  const unreachable = delay === null;
  const samples = points.slice(-latencyBarCount(signalWidth));
  const bars = samples.length ? samples : Array.from({ length: latencyBarCount(signalWidth) }, () => ({ delay: 1 }));
  const tone = unreachable ? "failed" : !delay ? "" : delay > mediumLatency ? "slow" : delay > lowLatency ? "medium" : "";
  const point = hovered == null || !samples.length ? null : samples[hovered];

  useEffect(() => {
    const element = signalRef.current;
    if (!element) return;
    const measure = () => setSignalWidth(element.getBoundingClientRect().width);
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(([entry]) => setSignalWidth(entry?.contentRect.width ?? 0));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  return <button type="button" className="latency-card" aria-label={`${site.name} ${unreachable ? "不通" : delay ? `${delay}ms` : "未测速"}`} aria-busy={testing} onClick={onTest} onMouseLeave={() => setHovered(null)} onBlur={() => setHovered(null)}>
    <span className="site-logo"><img src={site.icon} alt="" /></span>
    <span ref={signalRef} className="latency-signal">{bars.map((sample, index) => {
      const failed = sample.delay <= 0 && samples.length > 0;
      return <i key={index} className={failed ? "failed" : ""} style={{ height: failed ? "100%" : "2px" }} onMouseEnter={() => samples.length && setHovered(index)} />;
    })}</span>
    {point && <span className="chart-tooltip latency-tooltip" role="tooltip" style={{ left: `clamp(132px, ${latencyTooltipLeft(hovered ?? 0, samples.length)}%, calc(100% - 132px))` }}>
      <b>{latencyPath(site.name, point.node)}</b>
      <span>{formatLatencyTime(point.time)} · {point.delay > 0 ? `${point.delay} ms` : "不通"}</span>
    </span>}
    <span className="latency-copy"><small>{site.name}</small><strong className={tone}>{testing ? "…" : unreachable ? "不通" : delay ?? "—"}{delay && !testing ? <em>ms</em> : null}</strong></span>
  </button>;
}

export function latencyBarCount(width: number) {
  return width > 0 ? Math.max(1, Math.min(60, Math.floor((width + 1) / 8))) : 10;
}

export function currentSiteLatency(points: SiteLatencyPoint[]) {
  const latest = points.at(-1);
  if (!latest) return undefined;
  return latest.delay > 0 ? latest.delay : null;
}

function MetricValue({ label, value }: { label: string; value: string }) {
  return <div><span>{label}</span><strong>{value}</strong></div>;
}

function SparkCard({ label, value, points, formatValue, paused, filled = false, color = "green", onPause }: {
  label: string;
  value: string;
  points: MetricPoint[];
  formatValue: (value: number) => string;
  paused: boolean;
  filled?: boolean;
  color?: "green" | "blue";
  onPause: () => void;
}) {
  const [hovered, setHovered] = useState<number | null>(null);
  const values = points.map(point => point.value);
  const scale = sparkScale(label, values);
  const active = hovered == null ? null : points[hovered];
  return <article className={`spark-card ${color}${filled ? " filled" : ""}`}>
    <div><b>{label}</b><small>{value}</small><IconButton label={paused ? "继续刷新" : "暂停刷新"} active={paused} onClick={onPause}>{paused ? <PlayIcon /> : <PauseIcon />}</IconButton></div>
    <span className="spark-axis" aria-hidden="true">{scale.labels.map(item => <i key={item}>{item}</i>)}</span>
    <div className="spark-plot" onMouseLeave={() => setHovered(null)}>
      <LineChart values={values} maximum={scale.maximum} filled={filled} hovered={hovered} />
      <div className="spark-hit-zones" style={{ left: `${historyPointPosition(0, points.length)}%` }}>{points.map((point, index) => <span key={`${point.time}-${index}`} onMouseEnter={() => setHovered(index)} />)}</div>
      {active && <span className="chart-tooltip light spark-tooltip" role="tooltip" style={{ left: `clamp(90px, ${historyPointPosition(hovered ?? 0, points.length)}%, calc(100% - 90px))` }}><i className={color} />{label} ({formatClock(active.time)}): {formatValue(active.value)}</span>}
    </div>
  </article>;
}

function LineChart({ values, maximum, filled, hovered }: { values: number[]; maximum: number; filled: boolean; hovered: number | null }) {
  const data = values.slice(-metricWindowSize);
  const chartPoints = data.map((value, index) => `${historyPointPosition(index, data.length)},${48 - (value / maximum) * 39}`).join(" ");
  const firstX = historyPointPosition(0, data.length);
  const lastX = historyPointPosition(Math.max(0, data.length - 1), data.length);
  const area = `${firstX},48 ${chartPoints} ${lastX},48`;
  return <svg viewBox="0 0 100 52" preserveAspectRatio="none" aria-hidden="true">
    <line x1="0" y1="9" x2="100" y2="9" className="chart-grid-line" />
    <line x1="0" y1="28" x2="100" y2="28" className="chart-grid-line" />
    <line x1="0" y1="48" x2="100" y2="48" className="chart-grid-line" />
    {filled && data.length > 1 && <polygon points={area} className="chart-area-fill" />}
    {data.length > 1 && <polyline points={chartPoints} className="chart-line" vectorEffect="non-scaling-stroke" />}
    {hovered != null && data[hovered] != null && <>
      <line x1={historyPointPosition(hovered, data.length)} y1="9" x2={historyPointPosition(hovered, data.length)} y2="48" className="chart-hover-line" vectorEffect="non-scaling-stroke" />
      <circle cx={historyPointPosition(hovered, data.length)} cy={48 - (data[hovered].valueOf() / maximum) * 39} r="1.2" className="chart-hover-dot" vectorEffect="non-scaling-stroke" />
    </>}
  </svg>;
}

function sparkScale(label: string, values: number[]) {
  const observed = Math.max(1, ...values);
  if (label === "内存使用") {
    const maximum = Math.max(100 * 1024 * 1024, observed);
    return { maximum, labels: [formatMemory(maximum), formatMemory(maximum * .85), formatMemory(maximum * .57), formatMemory(maximum * .28)] };
  }
  if (label === "连接数") {
    const maximum = Math.max(90, observed);
    return { maximum, labels: [String(Math.round(maximum)), String(Math.round(maximum * 2 / 3)), String(Math.round(maximum / 3))] };
  }
  const maximum = Math.max(60_000, observed);
  return { maximum, labels: [formatRate(maximum), formatRate(maximum * 2 / 3), formatRate(maximum / 3)] };
}

function MonthPicker({ month, onChange }: { month: string; onChange: (value: string) => void }) {
  const previous = addMonths(month, -1);
  const next = addMonths(month, 1);
  const previousDisabled = !isMonthInHistoryRange(previous);
  const nextDisabled = next > getCurrentMonth();
  return <div className="month-picker">
    <IconButton label="上一个月" disabled={previousDisabled} onClick={() => onChange(previous)}><ChevronLeftIcon /></IconButton>
    <div className="month-tabs" role="tablist">
      <button type="button" role="tab" disabled={previousDisabled} onClick={() => onChange(previous)}>{formatMonthLabel(previous)}</button>
      <button type="button" role="tab" aria-selected="true" className="active">{formatMonthLabel(month)}</button>
      <button type="button" role="tab" disabled={nextDisabled} onClick={() => onChange(next)}>{formatMonthLabel(next)}</button>
    </div>
    <IconButton label="下一个月" disabled={nextDisabled} onClick={() => onChange(next)}><ChevronRightIcon /></IconButton>
  </div>;
}

function MonthlyChart({ data, selectedDay, onSelectDay, loading }: { data: TrafficMonth | null; selectedDay: string; onSelectDay: (value: string) => void; loading: boolean }) {
  const [hoveredDay, setHoveredDay] = useState<string | null>(null);
  const maximum = Math.max(1, ...(data?.days.map(item => item.up + item.down) ?? [1]));
  const average = (data?.avg.up ?? 0) + (data?.avg.down ?? 0);
  const activeIndex = data?.days.findIndex(item => item.day === hoveredDay) ?? -1;
  const active = activeIndex >= 0 ? data?.days[activeIndex] : null;
  return <div className={`monthly-chart${loading ? " loading" : ""}`}>
    <div className="bar-chart" onMouseLeave={() => setHoveredDay(null)}>
      {(data?.days ?? []).map(item => {
        const total = item.up + item.down;
        const height = total > 0 ? Math.max(4, (total / maximum) * 88) : 0;
        const downRatio = total ? (item.down / total) * 100 : 0;
        return <button type="button" key={item.day} className={`month-day${selectedDay === item.day ? " active" : ""}`} aria-label={`${item.day}，总流量 ${formatBytes(total)}`} onMouseEnter={() => setHoveredDay(item.day)} onFocus={() => setHoveredDay(item.day)} onBlur={() => setHoveredDay(null)} onClick={() => onSelectDay(item.day)}>
          {total > 0 && <em style={{ bottom: `calc(${height}% + 4px)` }}>{compactBytes(total)}</em>}
          {total > 0 && <i style={{ height: `${height}%` }}><span className="month-down" style={{ height: `${downRatio}%` }} /><span className="month-up" style={{ height: `${100 - downRatio}%` }} /></i>}
          <small>{Number(item.day.slice(-2))}</small>
        </button>;
      })}
      {active && <span className="chart-tooltip light month-tooltip" role="tooltip" style={{ left: `clamp(94px, ${((activeIndex + .5) / Math.max(1, data?.days.length ?? 1)) * 100}%, calc(100% - 94px))` }}>
        <b>{active.day}</b><span><i />进站 {formatBytes(active.down)}</span><span><i className="blue" />出站 {formatBytes(active.up)}</span><span>总流量 {formatBytes(active.down + active.up)} · {active.conns} 个连接</span>
      </span>}
      <span className="chart-average">日均 {formatBytes(average)}</span>
    </div>
  </div>;
}

function DailyChart({ data, selectedHour, onSelectHour }: { data: TrafficDay | null; selectedHour: number | null; onSelectHour: (value: number) => void }) {
  const [hoveredHour, setHoveredHour] = useState<number | null>(null);
  const sourceHours = new Map(data?.hours.map(item => [item.hour, item]));
  const hours = Array.from({ length: 24 }, (_, hour) => sourceHours.get(hour) ?? { hour, up: 0, down: 0, conns: 0 });
  const maximum = Math.max(1, ...hours.flatMap(item => [item.up, item.down]));
  const downPoints = areaPoints(hours.map(item => item.down), maximum);
  const upPoints = areaPoints(hours.map(item => item.up), maximum);
  const peak = hours.reduce((best, item) => item.up + item.down > best.up + best.down ? item : best, hours[0] ?? { hour: 0, up: 0, down: 0, conns: 0 });
  const active = hoveredHour == null ? null : hours.find(item => item.hour === hoveredHour) ?? null;
  const activeX = active ? pointPosition(active.hour, hours.length) : 0;
  return <div className="daily-chart">
    <div className="area-chart-wrap" onMouseLeave={() => setHoveredHour(null)}>
      <div className="axis-y"><span>{compactBytes(maximum)}</span><span>{compactBytes(maximum * .75)}</span><span>{compactBytes(maximum * .5)}</span><span>{compactBytes(maximum * .25)}</span><span>0 B</span></div>
      <svg className="area-chart" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
        <polygon points={`0,100 ${downPoints} 100,100`} className="daily-area-down" />
        <polyline points={downPoints} className="daily-line-down" vectorEffect="non-scaling-stroke" />
        <polygon points={`0,100 ${upPoints} 100,100`} className="daily-area-up" />
        <polyline points={upPoints} className="daily-line-up" vectorEffect="non-scaling-stroke" />
        {active && <>
          <line x1={activeX} y1="0" x2={activeX} y2="100" className="daily-hover-line" vectorEffect="non-scaling-stroke" />
        </>}
      </svg>
      {active && <>
        <span className="daily-hover-dot-html down" style={{ left: `calc(${42 * (1 - activeX / 100)}px + ${activeX}%)`, top: `${(100 - (active.down / maximum) * 92) * .886}%` }} />
        <span className="daily-hover-dot-html up" style={{ left: `calc(${42 * (1 - activeX / 100)}px + ${activeX}%)`, top: `${(100 - (active.up / maximum) * 92) * .886}%` }} />
      </>}
      <div className="hour-hit-zones">{hours.map(item => <button type="button" key={item.hour} className={selectedHour === item.hour ? "active" : ""} aria-label={`${item.hour}:00–${String(item.hour + 1).padStart(2, "0")}:00，${formatBytes(item.up + item.down)}`} onMouseEnter={() => setHoveredHour(item.hour)} onFocus={() => setHoveredHour(item.hour)} onBlur={() => setHoveredHour(null)} onClick={() => onSelectHour(item.hour)} />)}</div>
      {active && <span className="chart-tooltip light daily-tooltip" role="tooltip" style={{ left: `clamp(96px, calc(${42 * (1 - activeX / 100)}px + ${activeX}%), calc(100% - 96px))` }}>
        <b>{String(active.hour).padStart(2, "0")}:00–{String(active.hour).padStart(2, "0")}:59</b><span><i />进站 {formatBytes(active.down)}</span><span><i className="blue" />出站 {formatBytes(active.up)}</span>
      </span>}
      <div className="axis-x"><span>00:00</span><span>03:00</span><span>06:00</span><span>09:00</span><span>12:00</span><span>15:00</span><span>18:00</span><span>21:00</span></div>
    </div>
    <div className="peak-legend"><span>峰值 {String(peak.hour).padStart(2, "0")}:00–{String(peak.hour).padStart(2, "0")}:59</span><span><i />进站 {formatBytes(peak.down)}</span><span><i className="out" />出站 {formatBytes(peak.up)}</span></div>
  </div>;
}

function TrafficTableRow({ row, total, expanded, onOpen }: { row: TrafficRow; total: number; expanded: boolean; onOpen: () => void }) {
  const value = row.up + row.down;
  const percentage = total > 0 ? Math.round((value / total) * 100) : 0;
  return <div className={`insight-row interactive${expanded ? " expanded" : ""}`} role="row" tabIndex={0} onClick={onOpen} onKeyDown={event => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onOpen(); } }}>
    <div><b><ChevronDownIcon aria-hidden="true" />{row.key}</b>{(row.name || row.self?.iface) && <span>{row.name || `本机 · ${row.self?.iface}`}</span>}</div>
    <span>{formatBytes(row.down)}</span><span>{formatBytes(row.up)}</span><strong>{formatBytes(value)}</strong>
    <div className="share-cell"><div className="usage-bar" role="progressbar" aria-valuenow={percentage} aria-valuemin={0} aria-valuemax={100}><i style={{ width: `${percentage}%` }} /></div><em>{percentage}%</em></div>
  </div>;
}

function DrillTable({ dimensions, selected, loading, data, dataByDimension, onSelect }: {
  dimensions: Array<{ value: TrafficDimension; label: string }>;
  selected: TrafficDimension;
  loading: boolean;
  data: TrafficDrill | null;
  dataByDimension: Partial<Record<TrafficDimension, TrafficDrill>>;
  onSelect: (value: TrafficDimension) => void;
}) {
  const total = (data?.sum.up ?? 0) + (data?.sum.down ?? 0);
  return <section className="drill-panel" aria-label="流量下钻明细" aria-busy={loading}>
    <div className="drill-tabs" role="tablist">{dimensions.map(item => <button type="button" role="tab" aria-selected={selected === item.value} className={selected === item.value ? "active" : ""} key={item.value} onClick={event => { event.stopPropagation(); onSelect(item.value); }}>{item.label}{dataByDimension[item.value] ? ` (${dataByDimension[item.value]?.count})` : ""}</button>)}</div>
    {loading ? <p className="drill-loading">正在读取明细…</p> : <div className="drill-table" role="table">
      <div className="insight-table-head" role="row"><span>名称</span><span>进站</span><span>出站</span><span>总流量</span><span>占比</span></div>
      {(data?.rows ?? []).map(row => <StaticTrafficRow key={row.key} row={row} total={total} />)}
    </div>}
  </section>;
}

function StaticTrafficRow({ row, total }: { row: TrafficRow; total: number }) {
  const value = row.up + row.down;
  const percentage = total > 0 ? Math.round((value / total) * 100) : 0;
  return <div className="insight-row" role="row"><div><b>{row.key}</b></div><span>{formatBytes(row.down)}</span><span>{formatBytes(row.up)}</span><strong>{formatBytes(value)}</strong><div className="share-cell"><div className="usage-bar" role="progressbar" aria-valuenow={percentage} aria-valuemin={0} aria-valuemax={100}><i style={{ width: `${percentage}%` }} /></div><em>{percentage}%</em></div></div>;
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

function initialMetricHistory(stats: ShellStats): MetricHistory {
  const time = Date.now();
  const emptyWindow = Array.from({ length: metricWindowSize - 1 }, (_, index) => ({
    value: 0,
    time: time - (metricWindowSize - 1 - index) * 1000,
  }));
  return {
    inboundRate: [...emptyWindow, { value: stats.inboundRate, time }],
    outboundRate: [...emptyWindow, { value: stats.outboundRate, time }],
    memory: [...emptyWindow, { value: stats.memory, time }],
    connections: [...emptyWindow, { value: stats.connections, time }],
  };
}

function appendMetric(points: MetricPoint[], value: number) {
  return [...points.slice(-(metricWindowSize - 1)), { value, time: Date.now() }];
}

function formatMemory(value: number) {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  return `${(value / 1024 / 1024).toFixed(1).replace(/\.0$/, "")} MiB`;
}

function compactBytes(value: number) {
  return formatBytes(Math.max(0, Math.round(value)), 1).replace(" ", "");
}

function areaPoints(values: number[], maximum: number) {
  return values.map((value, index) => `${(index / Math.max(1, values.length - 1)) * 100},${100 - (value / maximum) * 92}`).join(" ");
}

export function addMonths(month: string, amount: number) {
  const [year, monthNumber] = month.split("-").map(Number);
  const date = new Date(year, monthNumber - 1 + amount, 1);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}`;
}

export function isMonthInHistoryRange(month: string, currentMonth = getCurrentMonth()) {
  return month >= addMonths(currentMonth, -12) && month <= currentMonth;
}

export function lastDayOfMonth(month: string) {
  const [year, monthNumber] = month.split("-").map(Number);
  const last = new Date(year, monthNumber, 0).getDate();
  return `${month}-${String(last).padStart(2, "0")}`;
}

export function formatMonthLabel(month: string) {
  const [year, monthNumber] = month.split("-").map(Number);
  return `${year}年${monthNumber}月`;
}

function pointPosition(index: number, length: number) {
  return (index / Math.max(1, length - 1)) * 100;
}

export function historyPointPosition(index: number, sampleCount: number) {
  const count = Math.min(metricWindowSize, Math.max(1, sampleCount));
  const offset = metricWindowSize - count;
  return ((offset + Math.min(Math.max(0, index), count - 1)) / (metricWindowSize - 1)) * 100;
}

function latencyTooltipLeft(index: number, length: number) {
  return 22 + pointPosition(index, length) * .48;
}

function formatClock(value: number) {
  return new Intl.DateTimeFormat("zh-CN", { hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false }).format(value);
}

function formatLatencyTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${month}-${day} ${formatClock(date.getTime())}`;
}

function latencyPath(siteName: string, node?: string) {
  if (!node) return siteName;
  return node.startsWith(`${siteName} →`) || node === siteName ? node : `${siteName} → ${node}`;
}

export function createLatestRequestGuard() {
  let current = 0;
  return {
    issue() {
      current += 1;
      return current;
    },
    invalidate() {
      current += 1;
    },
    isCurrent(requestId: number) {
      return requestId === current;
    },
  };
}
