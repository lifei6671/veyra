import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowPathIcon, ArrowRightCircleIcon, BoltIcon, ChevronDownIcon, MagnifyingGlassIcon, MapIcon, SparklesIcon, XMarkIcon } from "@heroicons/react/24/outline";
import { toast } from "sonner";
import { SelectControl } from "../ui/controls";
import { api } from "../api/client";
import type { ControllerProxy, ControllerRule, PenetrationResult, RouteDiagnosticResult, RouteProbeMethod, StorageResponse, TerminalTestCapability } from "../api/types";
import { latestLatency, matchesQuery } from "../lib/format";
import { EmptyState, ErrorState, IconButton } from "../components/shared";
import { mergeProxyGroupLatencyResult, mergeProxyLatency, ProxyGroupCard, proxyIcon, summarizeGroupLatencyTest } from "../components/ProxyCards";
import baiduIcon from "../assets/baidu.svg";
import googleSiteIcon from "../assets/google.svg";
import openaiIcon from "../assets/openai.svg";
import telegramIcon from "../assets/telegram.svg";

const quickFilters = [
  { name: "百度", value: "www.baidu.com", icon: baiduIcon },
  { name: "Google", value: "www.google.com", icon: googleSiteIcon },
  { name: "OpenAI", value: "api.openai.com", icon: openaiIcon },
  { name: "Telegram", value: "telegram.org", icon: telegramIcon },
] as const;

export function RulesPage({ storage, onToast }: { storage: StorageResponse; onToast: (message: string) => void }) {
  const [items, setItems] = useState<ControllerRule[]>([]);
  const [proxies, setProxies] = useState<Record<string, ControllerProxy>>({});
  const [query, setQuery] = useState("");
  const [expandedRule, setExpandedRule] = useState<number | null>(null);
  const [penetrationExpanded, setPenetrationExpanded] = useState<Record<number, boolean>>({});
  const [busy, setBusy] = useState("");
  const [testingNodes, setTestingNodes] = useState<Set<string>>(() => new Set());
  const testingNodesRef = useRef(new Set<string>());
  const [error, setError] = useState<unknown>(null);
  const [routeTarget, setRouteTarget] = useState("");
  const [routeMode, setRouteMode] = useState<"terminal" | "kernel">("terminal");
  const [routeMethod, setRouteMethod] = useState<RouteProbeMethod>("GET");
  const [penetration, setPenetration] = useState<PenetrationResult | null>(null);
  const [penetrationLoading, setPenetrationLoading] = useState(false);
  const [penetrationError, setPenetrationError] = useState("");
  const [capability, setCapability] = useState<TerminalTestCapability | null>(null);
  const [actualRoute, setActualRoute] = useState<RouteDiagnosticResult | null>(null);
  const [actualLoading, setActualLoading] = useState(false);
  const [actualError, setActualError] = useState("");
  const penetrationRequest = useRef(0);
  const actualRequest = useRef(0);

  const load = async () => {
    setError(null);
    try {
      const [rules, proxyData] = await Promise.all([api.rules(), api.proxies()]);
      setItems(rules.rules);
      setProxies(proxyData.proxies);
    } catch (reason) {
      setError(reason);
    }
  };

  useEffect(() => { void load(); }, []);

  const timeout = Number(storage.entries["config/speedtest-timeout"] ?? 5000);
  const testUrl = storage.entries["config/speedtest-url"] ?? "http://www.gstatic.com/generate_204";
  const hideUnavailable = storage.entries["config/hide-unavailable-proxies"] === "true";
  const thresholds = {
    low: finiteSetting(storage.entries["config/low-latency"], 500),
    medium: finiteSetting(storage.entries["config/medium-latency"], 1000),
  };
  const testingGroup = busy.startsWith("test:") ? busy.slice(5) : "";

  const runActualRoute = async (target: string, mode: "terminal" | "kernel", method: RouteProbeMethod) => {
    const requestId = ++actualRequest.current;
    setActualLoading(true);
    setActualError("");
    setActualRoute(null);
    try {
      if (mode === "kernel") {
        const result = await api.routeTest(target);
        if (requestId === actualRequest.current) setActualRoute(result);
        return;
      }
      const nextCapability = await api.terminalTestCapability();
      if (requestId !== actualRequest.current) return;
      setCapability(nextCapability);
      if (!nextCapability.ok) return;
      const result = await api.terminalTest(target, undefined, method);
      if (requestId === actualRequest.current) setActualRoute(result);
    } catch (reason) {
      if (requestId === actualRequest.current) setActualError(reason instanceof Error ? reason.message : "真实路由测试失败");
    } finally {
      if (requestId === actualRequest.current) setActualLoading(false);
    }
  };

  const inspectQuickRoute = async (target: string) => {
    setQuery(target);
    setRouteTarget(target);
    setCapability(null);
    setPenetrationError("");
    setPenetrationLoading(true);
    const requestId = ++penetrationRequest.current;
    void runActualRoute(target, routeMode, routeMethod);
    try {
      const result = await api.penetration(target);
      if (requestId === penetrationRequest.current) setPenetration(result);
    } catch (reason) {
      if (requestId === penetrationRequest.current) {
        setPenetration(null);
        setPenetrationError(reason instanceof Error ? reason.message : "规则路由推算失败");
      }
    } finally {
      if (requestId === penetrationRequest.current) setPenetrationLoading(false);
    }
  };

  const clearRouteLookup = () => {
    penetrationRequest.current += 1;
    actualRequest.current += 1;
    setQuery("");
    setRouteTarget("");
    setPenetration(null);
    setPenetrationError("");
    setActualRoute(null);
    setCapability(null);
    setActualError("");
    setPenetrationLoading(false);
    setActualLoading(false);
  };

  const changeRouteMode = (mode: "terminal" | "kernel") => {
    if (!routeTarget || mode === routeMode) return;
    setRouteMode(mode);
    void runActualRoute(routeTarget, mode, routeMethod);
  };

  const changeRouteMethod = (method: string) => {
    const next = method as RouteProbeMethod;
    setRouteMethod(next);
    if (routeTarget && routeMode === "terminal") void runActualRoute(routeTarget, "terminal", next);
  };

  const select = async (group: string, name: string) => {
    setBusy(`${group}:${name}`);
    try {
      await api.selectProxy(group, name);
      setProxies(current => ({ ...current, [group]: { ...current[group], now: name } }));
      onToast(`${group} 已切换到 ${name}`);
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "切换失败");
    } finally {
      setBusy("");
    }
  };

  const testGroup = async (group: string) => {
    const total = proxies[group]?.all?.length ?? 0;
    const toastId = toast.loading(group, { description: `0/${total} 测试完成`, duration: Infinity });
    setBusy(`test:${group}`);
    try {
      const result = await api.testProxyGroup(group, testUrl, timeout);
      const summary = summarizeGroupLatencyTest(result, total);
      setProxies(current => mergeProxyGroupLatencyResult(current, group, result));
      const detail = `测试完成：${summary.success} 成功，${summary.failed} 失败${summary.failed ? `（超时 ${summary.failed}）` : ""}`;
      const present = summary.failed ? toast.warning : toast.success;
      present(group, { id: toastId, description: detail, duration: Infinity });
      window.setTimeout(() => toast.dismiss(toastId), 4200);
    } catch (reason) {
      toast.error(group, { id: toastId, description: reason instanceof Error ? reason.message : "延迟测试失败", duration: Infinity });
      window.setTimeout(() => toast.dismiss(toastId), 5000);
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
      setProxies(current => mergeProxyLatency(current, name, delay));
      onToast(`${name} 延迟测试完成`);
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "延迟测试失败");
    } finally {
      testingNodesRef.current.delete(name);
      setTestingNodes(new Set(testingNodesRef.current));
    }
  };

  const visible = useMemo(() => items
    .map((item, index) => ({ item, index }))
    .filter(({ item }) => matchesQuery([item.type, item.payload, item.proxy], query)), [items, query]);

  return <main className="page rules-page">
    <div className="rule-controls">
      <label className="rule-search">
        <MagnifyingGlassIcon aria-hidden="true" />
        <input aria-label="搜索规则" value={query} onChange={event => { setQuery(event.target.value); if (event.target.value !== routeTarget) setRouteTarget(""); }} placeholder="搜索 域名 / IP / 关键字" />
        {query && <button type="button" aria-label="清空搜索" onClick={clearRouteLookup}><XMarkIcon /></button>}
      </label>
      <IconButton label="格式化搜索" tooltip className="rule-format" onClick={() => setQuery(normalizeRuleQuery(query))}><SparklesIcon /></IconButton>
      <div className="rule-quick-icons">
        {quickFilters.map(site => <button
          type="button"
          aria-label={site.name}
          title={`查询 ${site.name} 的分流规则和真实路由`}
          className={query === site.value ? "active" : ""}
          key={site.name}
          onClick={() => void inspectQuickRoute(site.value)}
        ><img src={site.icon} alt="" /></button>)}
      </div>
    </div>

    {Boolean(error) && <ErrorState title="规则加载失败" error={error} onRetry={() => void load()} />}
    <section className="rule-list">
      {routeTarget && <RouteComparison
        target={routeTarget}
        penetration={penetration}
        penetrationLoading={penetrationLoading}
        penetrationError={penetrationError}
        mode={routeMode}
        method={routeMethod}
        capability={capability}
        actual={actualRoute}
        actualLoading={actualLoading}
        actualError={actualError}
        ipv6Enabled={storage.entries["config/ipv6-test"] === "true"}
        onModeChange={changeRouteMode}
        onMethodChange={changeRouteMethod}
        onRetry={() => void runActualRoute(routeTarget, routeMode, routeMethod)}
      />}
      {visible.map(({ item, index }) => {
        const groupName = ruleProxyGroup(item.proxy);
        const group = groupName ? proxies[groupName] : undefined;
        const expandable = canExpandRule(item, proxies);
        const expanded = expandable && expandedRule === index;
        const summary = <RuleSummary item={item} index={index} proxies={proxies} />;
        return <article className={`surface rule-row${expandable ? " expandable" : ""}${expanded ? " expanded" : ""}`} key={`${index}-${item.type}-${item.payload}`}>
          {expandable
            ? <button type="button" className="rule-summary" aria-expanded={expanded} onClick={() => setExpandedRule(expanded ? null : index)}>{summary}</button>
            : <div className="rule-summary">{summary}</div>}
          {expandable && group && <section className={`rule-policy-collapse policy-card-collapse${expanded ? "" : " collapsed"}`} aria-hidden={!expanded} inert={!expanded || undefined}>
            <div className="policy-card-collapse-inner">
              <RuleProxyDetails
                group={group}
                proxies={proxies}
                busy={Boolean(busy)}
                hideUnavailable={hideUnavailable}
                thresholds={thresholds}
                testingGroup={testingGroup}
                testingNodes={testingNodes}
                penetrationExpanded={Boolean(penetrationExpanded[index])}
                onTogglePenetration={() => setPenetrationExpanded(current => ({ ...current, [index]: !current[index] }))}
                onSelect={select}
                onTest={testGroup}
                onTestNode={testNode}
              />
            </div>
          </section>}
        </article>;
      })}
    </section>
    {!error && !visible.length && !routeTarget && <EmptyState icon="◌" title="没有匹配的规则" text="尝试调整搜索关键字" />}
  </main>;
}

type RouteStage = {
  number: number;
  title: string;
  badge?: string;
  headline: string;
  icon?: string;
  meta?: string;
  metaIcon?: string;
  resultBadge?: string;
  resultBadgeAfterMeta?: boolean;
  resultBadgeTone?: "success" | "error";
  badgeTone?: "success" | "warning" | "error" | "muted" | "proxy";
  chain?: string[];
  value?: string;
  source?: string;
  tone?: "success" | "warning" | "muted";
  details?: RouteStageDetail[];
};

type RouteStageDetail = {
  text: string;
  label?: string;
  values?: string[];
  chain?: string[];
  tone?: "warning";
  mono?: boolean;
  valueMono?: boolean;
};

export function RouteComparison({ target, penetration, penetrationLoading, penetrationError, mode, method, capability, actual, actualLoading, actualError, ipv6Enabled = false, onModeChange, onMethodChange, onRetry }: {
  target: string;
  penetration: PenetrationResult | null;
  penetrationLoading: boolean;
  penetrationError: string;
  mode: "terminal" | "kernel";
  method: RouteProbeMethod;
  capability: TerminalTestCapability | null;
  actual: RouteDiagnosticResult | null;
  actualLoading: boolean;
  actualError: string;
  ipv6Enabled?: boolean;
  onModeChange: (mode: "terminal" | "kernel") => void;
  onMethodChange: (method: string) => void;
  onRetry: () => void;
}) {
  const effectivePenetration = penetrationError ? null : penetration;
  const predictedStages = buildPredictedStages(target, effectivePenetration);
  const unavailable = mode === "terminal" && capability && !capability.ok;
  const actualStages = unavailable
    ? buildUnavailableStages(target, capability)
    : actualLoading
      ? buildLoadingStages(target, mode)
      : buildActualStages(target, mode, actual, ipv6Enabled);
  const actualStatus = actualLoading ? "测试中" : unavailable ? "无法模拟" : actualError ? "测试失败" : actual ? diagnosticStatus(actual) : "等待测试";

  return <section className="route-comparison" aria-label={`${target} 路由诊断`}>
    <RoutePanel
      icon={<MapIcon />}
      title="规则路由"
      subtitle="依据查询条件推算"
      status={penetrationError ? "推算失败" : penetrationLoading && !effectivePenetration ? "推算中" : effectivePenetration ? "已推算" : "推算失败"}
      statusTone={penetrationError ? "warning" : effectivePenetration ? "success" : penetrationLoading ? "muted" : "warning"}
      stages={predictedStages}
      loading={!penetrationError && penetrationLoading && !effectivePenetration}
      error={penetrationError}
    />
    <RoutePanel
      icon={<BoltIcon />}
      title="真实路由"
      subtitle={mode === "terminal" ? "模拟一台 LAN 终端的访问" : "面板自身发起的测试"}
      status={actualStatus}
      statusTone={actualLoading ? "muted" : actualError || actual?.exit?.error ? "error" : unavailable || (actual && actual.exit?.ok !== true) ? "warning" : actual ? "success" : "muted"}
      stages={actualStages}
      loading={actualLoading}
      showLoadingSpinner
      actions={<div className="route-diagnostic-actions">
        {mode === "terminal" && <SelectControl className="route-method-select" label="探测方式" value={method} onValueChange={onMethodChange} options={["GET", "HEAD", "TCP", "TLS"].map(value => ({ value, label: value }))} />}
        <div className="route-mode-toggle">
          <button type="button" className={mode === "terminal" ? "active" : ""} title="模拟一台 LAN 终端的访问" onClick={() => onModeChange("terminal")}>模拟终端</button>
          <button type="button" className={mode === "kernel" ? "active" : ""} title="内核诊断走面板回环入站，只展示内核内部的分流" onClick={() => onModeChange("kernel")}>内核诊断</button>
        </div>
        <button type="button" className="route-retry" title="重新测试" aria-label="重新测试" onClick={onRetry}><ArrowPathIcon /><span>重新测试</span></button>
      </div>}
      footer={unavailable ? <button type="button" className="route-switch-button" onClick={() => onModeChange("kernel")}>改用内核诊断</button> : undefined}
      error={actualError}
    />
  </section>;
}

function RoutePanel({ icon, title, subtitle, status, statusTone, stages, actions, footer, loading, showLoadingSpinner = false, error }: {
  icon: React.ReactNode;
  title: string;
  subtitle: string;
  status: string;
  statusTone: "success" | "warning" | "error" | "muted";
  stages: RouteStage[];
  actions?: React.ReactNode;
  footer?: React.ReactNode;
  loading: boolean;
  showLoadingSpinner?: boolean;
  error?: string;
}) {
  return <article className="surface route-panel">
    <header className="route-panel-header">
      <span className="route-panel-icon" aria-hidden="true">{icon}</span>
      <div><h2>{title} <em className={`route-status ${statusTone}${showLoadingSpinner && loading ? " loading" : ""}`}>{showLoadingSpinner && loading && <ArrowPathIcon />}{status}</em></h2><p>{subtitle}</p></div>
      {actions}
    </header>
    <div className={`route-stages${loading ? " loading" : ""}`}>
      {stages.map(stage => <RouteStageRow key={stage.number} stage={stage} footer={stage.number === 1 ? footer : undefined} />)}
      {error && <p className="route-diagnostic-error" role="alert">{error}</p>}
    </div>
  </article>;
}

function RouteStageRow({ stage, footer }: { stage: RouteStage; footer?: React.ReactNode }) {
  const [expanded, setExpanded] = useState(false);
  const hasDetails = Boolean(stage.details?.length);
  return <div className={`route-stage ${stage.tone ?? ""}`}>
    <div className="route-stage-rail"><span>{stage.number}</span></div>
    <div className="route-stage-content">
      <div className="route-stage-label">{stage.title}{stage.badge && <em className={stage.badgeTone}>{stage.badge}</em>}</div>
      <div className="route-stage-value">
        {stage.chain?.length
          ? <RouteChain chain={stage.chain} />
          : <>{stage.icon && <img src={stage.icon} alt="" />}<strong>{stage.headline}</strong></>}
        {stage.metaIcon && <img src={stage.metaIcon} alt="" />}
        {stage.resultBadge && !stage.resultBadgeAfterMeta && <em className={stage.resultBadgeTone}>{stage.resultBadge}</em>}
        {stage.meta && <span>{stage.meta}</span>}
        {stage.resultBadge && stage.resultBadgeAfterMeta && <em className={stage.resultBadgeTone}>{stage.resultBadge}</em>}
        {stage.value && <span>{stage.value}</span>}
        {stage.source && <span className="route-stage-source">{stage.source}</span>}
      </div>
      {footer}
      {hasDetails && <button type="button" className="route-details-toggle" aria-expanded={expanded} onClick={() => setExpanded(value => !value)}>查看详情 <ChevronDownIcon /></button>}
      {expanded && hasDetails && <div className="route-stage-details">
        {stage.details?.map((detail, index) => detail.chain?.length
          ? <div className="route-stage-detail-chain" key={`${index}-${detail.text}`}><span>{detail.text}</span><RouteChain chain={detail.chain} /></div>
          : detail.values?.length
            ? <div className="route-stage-detail-values" key={`${index}-${detail.label ?? detail.text}`}><span>{detail.label}</span>{detail.values.map(value => <em key={value}>{value}</em>)}</div>
            : <p className={`${detail.tone ?? ""}${detail.mono ? " mono" : ""}`.trim()} key={`${index}-${detail.text}`}>{detail.label && <><span className="route-stage-detail-label">{detail.label}</span>{" "}</>}{detail.valueMono ? <span className="route-stage-detail-mono">{detail.text}</span> : detail.text}</p>)}
      </div>}
    </div>
  </div>;
}

function RouteChain({ chain }: { chain: string[] }) {
  return <span className="route-stage-chain">
    {chain.map((name, index) => <span className="route-stage-chain-step" key={`${name}-${index}`}>
      {index > 0 && <ArrowRightCircleIcon aria-hidden="true" />}
      <span className="route-stage-chain-name">{proxyIcon(name) && <img src={proxyIcon(name)} alt="" />}{name}</span>
    </span>)}
  </span>;
}

export function buildPredictedStages(target: string, result: PenetrationResult | null): RouteStage[] {
  const entry = result?.matched?.entries?.[0];
  const server = result?.dns?.server;
  const addresses = result?.resolved?.addresses ?? [];
  const owner = result?.owner?.name || result?.chain?.[0] || "";
  const rawChain = result?.chain ?? [];
  const routeChain = result?.owner?.kind === "policy" && rawChain[0] === result.owner.name ? rawChain.slice(1) : rawChain;
  const finalOutbound = routeChain.at(-1) || result?.finalOutbound || owner || "—";
  const finalRouteType = result ? isDirectOutbound(finalOutbound) ? "直连" : "代理" : undefined;
  const ruleIndex = result?.matched?.index;
  const dnsHeadline = server ? `${result?.dns?.viaProxy ? "代理" : "直连"} DNS` : "—";
  const fakeIp = result?.resolved?.fakeIp === true;
  const dnsMeta = server ? `${String(server.type ?? "").toUpperCase()} ${server.server ?? server.tag ?? ""}${fakeIp ? " 返回 FakeIP 占位地址" : addresses.length ? ` 解析到 ${addresses.length} 个地址` : ""}`.trim() : undefined;
  const bypassSets = result?.firstLayer?.nativeBypass?.sets ?? [];
  const entryDetails: RouteStageDetail[] = [];
  const dnsDetails: RouteStageDetail[] = [];
  if (result?.firstLayer?.nativeBypass?.enabled && bypassSets.length) {
    entryDetails.push(
      { text: `直连 IP 集合 ${bypassSets.join(", ")} 在入口旁路（nft）,不进内核` },
      { text: "按目标 IP 判:终端连的地址在旁路集合里才旁路,面板预知不了终端拿到的地址。" },
    );
  }
  if (result?.firstLayer?.entryMode?.reason) entryDetails.push({ text: `入口没有默认放行:${result.firstLayer.entryMode.reason}` });
  if (server?.tag) dnsDetails.push({ text: `解析器 ${server.tag}` });
  if (fakeIp) {
    dnsDetails.push({ text: "内核 DNS 返回的是 FakeIP 占位地址:连接进内核后按域名找回目标,按 IP 判的规则不参与" });
  } else if (addresses.length) {
    dnsDetails.push({ text: `推算时向内核 DNS 解析到:${[...addresses].reverse().join("、")};有域名的访问不看任何 IP 条件,这个地址只拿去比内置的私网直连、节点站点直连` });
  }
  if (server?.detour) dnsDetails.push({ text: "解析经由", chain: [server.detour] });
  if (server || addresses.length) {
    dnsDetails.push({ text: "这里是内核 DNS 规则的推算;终端的查询是否进入内核,由上方的入口策略决定。" });
  }
  return [
    { number: 5, title: "最终出口", badge: finalRouteType, badgeTone: finalRouteType === "代理" ? "proxy" : "success", headline: finalOutbound, icon: proxyIcon(finalOutbound), chain: routeChain.length ? routeChain : undefined, tone: result ? "success" : "muted" },
    { number: 4, title: "规则匹配", badge: ruleIndex == null ? undefined : `第 ${ruleIndex + 1} 条`, headline: result ? "站点集" : "—", meta: owner || undefined, metaIcon: proxyIcon(owner), resultBadge: entry ? entryTypeLabel(entry.type) : undefined, resultBadgeAfterMeta: true, value: entry?.value, source: entry?.source, tone: result ? "success" : "muted", details: ruleDetails(result?.matched?.rule) },
    { number: 3, title: "业务入口", badge: result ? "配置推算" : undefined, headline: result ? "本目标是否旁路未判定" : "—", tone: result ? "warning" : "muted", details: entryDetails.length ? entryDetails : undefined },
    { number: 2, title: "DNS 解析", badge: result ? "规则推算" : undefined, headline: dnsHeadline, meta: dnsMeta, tone: result ? "success" : "muted", details: dnsDetails.length ? dnsDetails : undefined },
    { number: 1, title: "发起访问", badge: "域名", headline: target, meta: "终端来源 IP 未指定 · 端口未指定" },
  ];
}

export function buildLoadingStages(target: string, mode: "terminal" | "kernel"): RouteStage[] {
  const waiting: RouteStage[] = [
    { number: 5, title: "最终出口 / 访问结果" },
    { number: 4, title: "规则匹配" },
    { number: 3, title: "业务入口" },
    { number: 2, title: "DNS 解析" },
  ].map(stage => ({ ...stage, headline: "等待查询...", tone: "muted" as const }));
  const requestDetails: RouteStageDetail[] = mode === "kernel" ? [
    { text: "由面板经内核的回环入站发起,没有模拟任何终端的来源;结果不代表某台终端的实际访问。" },
    { text: "内核诊断走面板的回环入站,不经过 LAN 入口,只能看到内核内部的分流,看不到入口旁路。" },
  ] : [];
  waiting.push({
    number: 1,
    title: "发起访问",
    headline: mode === "kernel" ? "面板自身测试" : target,
    tone: "muted",
    badge: "域名",
    meta: mode === "kernel" ? target : "模拟 LAN 终端访问",
    details: requestDetails.length ? requestDetails : undefined,
  });
  return waiting;
}

export function buildActualStages(target: string, mode: "terminal" | "kernel", result: RouteDiagnosticResult | null, ipv6Enabled = false): RouteStage[] {
  const server = result?.dns?.server;
  const answers = result?.resolve?.answers ?? [];
  const exit = result?.exit;
  const hasDnsEvidence = Boolean(result?.dns && (server || typeof result.dns.viaProxy === "boolean" || result.dns.runtimeChain?.length));
  const hasIngressEvidence = Boolean(exit);
  const owner = exit?.owner?.name || exit?.chains?.[0] || "";
  const rawExitChain = exit?.chains ?? [];
  const exitChain = exit?.owner?.kind === "policy" && rawExitChain[0] === exit.owner.name ? rawExitChain.slice(1) : rawExitChain;
  const finalOutbound = exitChain.at(-1) || owner;
  const hasConnectionEvidence = Boolean(result?.exit && (owner || result.exit.rule || result.exit.rulePayload));
  const destination = result?.exit?.connectTo || result?.exit?.destinationIP;
  const status = result?.exit?.status;
  const exitHeadline = result?.exit ? finalOutbound || "未知出口" : "—";
  const exitMeta = result?.exit?.ok === true ? "访问正常" : result?.exit ? undefined : result ? "未返回访问结果" : undefined;
  const dnsHeadline = hasDnsEvidence
    ? `${result?.dns?.viaProxy === true ? "代理" : result?.dns?.viaProxy === false ? "直连" : ""} DNS`.trim()
    : result?.resolve?.ok === true ? "DNS 已解析" : result?.resolve?.ok === false ? "DNS 解析失败" : "—";
  const dnsMeta = hasDnsEvidence && server
    ? `${String(server.type ?? "").toUpperCase()} ${server.server ?? server.tag ?? ""}${answers.length ? ` IPv4 已解析 · ${answers.length} 个地址` : ""}`.trim()
    : result?.resolve?.ok === true && answers.length ? `IPv4 已解析 · ${answers.length} 个地址` : undefined;
  const exitDetails: RouteStageDetail[] = [];
  if (result?.exit?.error) exitDetails.push({ text: `访问失败:${result.exit.error}`, tone: "warning" });
  if (destination) exitDetails.push({ text: `目标 ${destination}`, valueMono: true });
  if (result?.exit?.viaProxy && destination) {
    if (result.resolve?.cached) {
      exitDetails.push({ text: `节点拿到的是 IP ${destination},按它直接连。这个地址来自内核缓存,是之前那次解析的答案,未必是当前线路就近的 CDN`, tone: "warning" });
    } else if (result.dns?.viaProxy) {
      exitDetails.push({ text: `节点拿到的是 IP ${destination},按它直接连。这个地址是上面经节点问出来的,已经是节点位置就近的 CDN,不用再解析一次` });
    } else {
      exitDetails.push({ text: `节点拿到的是 IP ${destination},按它直接连。注意这个地址是直连解析出来的,不是节点位置就近的 CDN`, tone: "warning" });
    }
  }
  const connectionDetails: RouteStageDetail[] = [];
  const matchedRule = result?.exit?.rule || result?.exit?.rulePayload;
  if (matchedRule) connectionDetails.push({ text: matchedRule, mono: true });
  if (mode === "kernel" && hasConnectionEvidence) {
    connectionDetails.push({ text: "本列是面板自己的测试连接,来源和入站路径与终端不同,不能用它补齐左侧缺失的终端条件。" });
  }
  const ingressDetails: RouteStageDetail[] = mode === "kernel" && hasIngressEvidence ? [
    { text: "这次测试经过 Open-Box 内核（连接表已记录）" },
    { text: "这次访问是面板以域名进内核的回环入站发起的,和终端按域名访问一样:只按域名条件判,前置自定义分流和站点集里的 IP 段、GeoIP 都不参与,域名都没命中就走兜底。" },
  ] : [];
  const dnsDetails: RouteStageDetail[] = [];
  if (result?.resolve?.ok === true && !ipv6Enabled) dnsDetails.push({ text: "IPv6 未查询（档案未开启 IPv6）" });
  if (server?.tag) dnsDetails.push({ label: "解析器", text: server.tag, valueMono: true });
  const dnsChain = server?.detour ? result?.dns?.runtimeChain?.length ? result.dns.runtimeChain : [server.detour] : [];
  if (dnsChain.length) dnsDetails.push({ text: "解析经由", chain: dnsChain });
  if (result?.resolve?.cached) {
    dnsDetails.push({ text: `命中内核缓存（还剩 ${result.resolve.ttl ?? "?"} 秒）。这份答案是之前解析的,这次没有重新经这条线路问`, tone: "warning" });
  } else if (answers.length && server?.server) {
    dnsDetails.push({ text: `${server.server} 应答` });
  }
  if (answers.length) dnsDetails.push({ label: "A", text: "A", values: answers });
  if (!answers.length && result?.resolve?.error) dnsDetails.push({ text: result.resolve.error, mono: false });
  const requestDetails: RouteStageDetail[] = mode === "kernel" && result ? [
    { text: "由面板经内核的回环入站发起,没有模拟任何终端的来源;结果不代表某台终端的实际访问。" },
    { text: "内核诊断走面板的回环入站,不经过 LAN 入口,只能看到内核内部的分流,看不到入口旁路。" },
  ] : [];
  return [
    { number: 5, title: "最终出口 / 访问结果", badge: result?.exit?.ms == null ? undefined : `${result.exit.ms} ms`, badgeTone: result?.exit?.error ? "error" : result?.exit?.ok === true ? "success" : "warning", headline: exitHeadline, icon: proxyIcon(finalOutbound), chain: exitChain.length ? exitChain : undefined, resultBadge: result?.exit?.error ? "访问失败" : status == null ? undefined : `HTTP ${status}`, resultBadgeTone: result?.exit?.error ? "error" : "success", meta: exitMeta, tone: result?.exit?.ok === true ? "success" : result ? "warning" : "muted", details: exitDetails.length ? exitDetails : undefined },
    { number: 4, title: "规则匹配", badge: hasConnectionEvidence ? "连接记录" : undefined, headline: owner ? "连接归属" : hasConnectionEvidence ? "规则已记录" : "—", meta: owner || undefined, metaIcon: proxyIcon(owner), tone: hasConnectionEvidence ? "success" : "muted", details: connectionDetails.length ? connectionDetails : undefined },
    { number: 3, title: "业务入口", badge: hasIngressEvidence ? "已进入" : undefined, headline: hasIngressEvidence ? (mode === "kernel" ? "面板回环入站" : "LAN 终端入口") : "—", tone: hasIngressEvidence ? "success" : "muted", details: ingressDetails.length ? ingressDetails : undefined },
    { number: 2, title: "DNS 解析", badge: result?.resolve?.ms == null ? undefined : `${result.resolve.ms} ms`, headline: dnsHeadline, meta: dnsMeta, tone: result?.resolve?.ok === false ? "warning" : hasDnsEvidence || result?.resolve?.ok === true ? "success" : "muted", details: dnsDetails.length ? dnsDetails : undefined },
    { number: 1, title: "发起访问", badge: "域名", headline: mode === "kernel" ? "面板自身测试" : target, meta: mode === "kernel" ? result?.exit?.url || `https://${target}/` : "模拟 LAN 终端访问", details: requestDetails.length ? requestDetails : undefined },
  ];
}

function diagnosticStatus(result: RouteDiagnosticResult) {
  if (result.exit?.ok === true) return "访问成功";
  if (result.exit?.ok === false) return "访问失败";
  if (result.resolve?.ok === false) return "解析失败";
  return "结果不完整";
}

export function buildUnavailableStages(_target: string, capability: TerminalTestCapability): RouteStage[] {
  const missing = capability.missing?.length ? `缺少:${capability.missing.map(terminalMissingLabel).join("、")}` : "当前设备未提供 LAN 终端模拟能力";
  const stages: RouteStage[] = [5, 4, 3, 2].map(number => ({ number, title: number === 5 ? "最终出口 / 访问结果" : number === 4 ? "规则匹配" : number === 3 ? "业务入口" : "DNS 解析", headline: "—", tone: "muted" }));
  stages.push({ number: 1, title: "发起访问", badge: "域名", headline: "此设备不能模拟 LAN 终端", tone: "warning", details: [
    { text: missing, tone: "warning" },
    { text: "不会自动退回内核回环测试:那条路径不经过 LAN 入口,看不到入口旁路。" },
  ] });
  return stages;
}

function ruleDetails(rule?: Record<string, unknown>): RouteStageDetail[] | undefined {
  if (!rule) return undefined;
  const details = Object.entries(rule)
    .filter(([key]) => key !== "outbound")
    .map(([key, value]) => ({ text: `${key}=${Array.isArray(value) ? value.join(", ") : String(value)}`, mono: true }));
  return details.length ? details : undefined;
}

function detailLines(value?: string, mono = false): RouteStageDetail[] | undefined {
  return value ? [{ text: value, mono }] : undefined;
}

function terminalMissingLabel(value: string) {
  return ({ lan: "找不到 LAN 网桥或它的 IPv4 地址", dhcp: "没有 udhcpc" } as Record<string, string>)[value] ?? value;
}

function isDirectOutbound(value: string) {
  return value === "直连" || value.toLowerCase() === "direct";
}

function entryTypeLabel(type: string) {
  return ({ domain_suffix: "域名后缀", domain: "域名", domain_keyword: "域名关键字", ip_cidr: "IP 网段", rule_set: "站点集" } as Record<string, string>)[type] ?? type;
}

function RuleSummary({ item, index, proxies }: { item: ControllerRule; index: number; proxies: Record<string, ControllerProxy> }) {
  const groupName = ruleProxyGroup(item.proxy);
  const group = groupName ? proxies[groupName] : undefined;
  const chain = group ? proxyChain(group, proxies) : [];
  const delay = group ? selectedDelay(group, proxies) : null;
  return <>
    <div className="rule-main">
      <span className="rule-index">{index + 1}.</span>
      <span className="rule-type">{item.type}</span>
      <code>{item.payload}</code>
    </div>
    <div className="rule-result">
      {group ? <RuleChain names={chain} /> : <span>{plainRuleAction(item.proxy)}</span>}
      {delay != null && <em>{delay}</em>}
    </div>
  </>;
}

function RuleChain({ names }: { names: string[] }) {
  return <div className="rule-chain">
    {names.map((name, index) => <span className="rule-chain-part" key={`${name}-${index}`}>
      {index > 0 && <ArrowRightCircleIcon aria-hidden="true" />}
      {iconForProxy(name) && <img src={iconForProxy(name)} alt="" />}
      <span>{name}</span>
    </span>)}
  </div>;
}

export function RuleProxyDetails({ group, proxies, busy, hideUnavailable, thresholds, testingGroup, testingNodes, penetrationExpanded, onTogglePenetration, onSelect, onTest, onTestNode }: {
  group: ControllerProxy;
  proxies: Record<string, ControllerProxy>;
  busy: boolean;
  hideUnavailable: boolean;
  thresholds: { low: number; medium: number };
  testingGroup: string;
  testingNodes?: ReadonlySet<string>;
  penetrationExpanded: boolean;
  onTogglePenetration: () => void;
  onSelect: (group: string, name: string) => Promise<void>;
  onTest: (group: string) => Promise<void>;
  onTestNode: (name: string) => Promise<void>;
}) {
  return <section className="rule-policy-detail">
    <ProxyGroupCard
      group={group}
      data={{ proxies }}
      collapsed={false}
      busy={busy}
      onToggle={() => undefined}
      onSelect={onSelect}
      onTest={onTest}
      onTestNode={onTestNode}
      variant="policy"
      hideUnavailable={hideUnavailable}
      thresholds={thresholds}
      history={group.history ?? []}
      latencyHistory={{}}
      testingGroup={testingGroup}
      testingNodes={testingNodes}
      nodeSort="default"
      penetrationExpanded={penetrationExpanded}
      onTogglePenetration={onTogglePenetration}
      embedded
    />
  </section>;
}

export function ruleProxyGroup(value: string) {
  return /^route\((.+)\)$/i.exec(value.trim())?.[1]?.trim() ?? "";
}

export function canExpandRule(rule: ControllerRule, proxies: Record<string, ControllerProxy>) {
  const group = proxies[ruleProxyGroup(rule.proxy)];
  return Boolean(group?.all?.length);
}

export function normalizeRuleQuery(value: string) {
  const trimmed = value.trim();
  if (!/^https?:\/\//i.test(trimmed)) return trimmed;
  try {
    return new URL(trimmed).hostname || trimmed;
  } catch {
    return trimmed;
  }
}

function proxyChain(group: ControllerProxy, proxies: Record<string, ControllerProxy>) {
  const names = [group.name];
  const visited = new Set(names);
  let current = group;
  while (current.now && !visited.has(current.now) && names.length < 5) {
    names.push(current.now);
    visited.add(current.now);
    const next = proxies[current.now];
    if (!next) break;
    current = next;
  }
  return names;
}

function selectedDelay(group: ControllerProxy, proxies: Record<string, ControllerProxy>) {
  const chain: ControllerProxy[] = [];
  let current: ControllerProxy | undefined = group;
  const visited = new Set<string>();
  while (current && !visited.has(current.name)) {
    visited.add(current.name);
    chain.push(current);
    current = current.now ? proxies[current.now] : undefined;
  }
  const leafKind = chain.at(-1)?.type.toLowerCase();
  if (leafKind === "direct" || leafKind === "reject") return null;
  for (const proxy of chain.reverse()) {
    const delay = latestLatency(proxy.history);
    if (delay != null) return delay;
  }
  return null;
}

function iconForProxy(name: string) {
  return proxyIcon(name);
}

export function proxyDetailLabel(proxy: ControllerProxy) {
  const value = proxy.type.toLowerCase();
  const type = value === "urltest" ? "urltest" : value;
  return `${type}${proxy.udp ? " / udp" : ""}`;
}

function plainRuleAction(value: string) {
  const group = ruleProxyGroup(value);
  return group || value;
}

function finiteSetting(value: string | undefined, fallback: number) {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : fallback;
}
