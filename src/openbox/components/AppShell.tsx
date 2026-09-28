import type { ReactNode } from "react";
import {
  ArrowPathIcon,
  ArrowsRightLeftIcon,
  ChevronDoubleLeftIcon,
  ChevronDoubleRightIcon,
  Cog6ToothIcon,
  DocumentTextIcon,
  FunnelIcon,
  GlobeAltIcon,
  HomeIcon,
  PlayIcon,
  StopIcon,
} from "@heroicons/react/24/outline";
import type { RouteKey, ServiceStatus } from "../api/types";
import openBoxLogo from "../assets/openbox-logo.png";
import { formatBytes, formatDuration, formatRate } from "../lib/format";
import { IconButton, PromoBar } from "./shared";

const routes: Array<{ key: RouteKey; label: string; icon: typeof HomeIcon }> = [
  { key: "overview", label: "概览", icon: HomeIcon },
  { key: "proxies", label: "代理", icon: GlobeAltIcon },
  { key: "connections", label: "连接", icon: ArrowsRightLeftIcon },
  { key: "logs", label: "日志", icon: DocumentTextIcon },
  { key: "rules", label: "规则", icon: FunnelIcon },
  { key: "settings", label: "设置", icon: Cog6ToothIcon },
];

export type ShellStats = {
  connections: number;
  memory: number;
  inbound: number;
  inboundRate: number;
  outbound: number;
  outboundRate: number;
};

export function AppShell({ route, onNavigate, service, stats, collapsed, onCollapse, onServiceAction, onRefresh, busy, children }: {
  route: RouteKey;
  onNavigate: (route: RouteKey) => void;
  service: ServiceStatus | null;
  stats: ShellStats;
  collapsed: boolean;
  onCollapse: () => void;
  onServiceAction: (action: "start" | "stop") => void;
  onRefresh: () => void;
  busy: boolean;
  children: ReactNode;
}) {
  const running = service?.core.running ?? false;
  return <div className={`openbox-app${collapsed ? " sidebar-collapsed" : ""}`}>
    <aside className="sidebar">
      <div className="sidebar-top"><span className="brand"><img src={openBoxLogo} alt="Open-Box" /></span><IconButton label={collapsed ? "展开侧栏" : "收起侧栏"} onClick={onCollapse}>{collapsed ? <ChevronDoubleRightIcon /> : <ChevronDoubleLeftIcon />}</IconButton></div>
      <nav className="main-nav" aria-label="主导航">{routes.map(item => { const RouteIcon = item.icon; return <button key={item.key} type="button" aria-label={item.label} className={route === item.key ? "active" : ""} onClick={() => onNavigate(item.key)} aria-current={route === item.key ? "page" : undefined}><RouteIcon /><span>{item.label}</span></button>; })}</nav>
      <div className="sidebar-status">
        <div className="status-grid"><Stat label="连接数" value={String(stats.connections)} /><Stat label="内存使用" value={formatBytes(stats.memory, 1)} /><Stat label="进站流量" value={formatBytes(stats.inbound, 2)} /><Stat label="进站速率" value={formatRate(stats.inboundRate)} accent="green" /><Stat label="出站流量" value={formatBytes(stats.outbound, 2)} /><Stat label="出站速率" value={formatRate(stats.outboundRate)} accent="blue" /></div>
        <div className="uptime"><span className={running ? "online-dot" : "offline-dot"} /><span>{running ? `运行时长 ${formatDuration(service?.core.uptimeSeconds ?? 0)}` : "内核已停止"}</span></div>
        <div className="sidebar-actions"><IconButton label="刷新数据" disabled={busy} onClick={onRefresh}><ArrowPathIcon /></IconButton><IconButton label={running ? "停止内核" : "启动内核"} disabled={busy} onClick={() => onServiceAction(running ? "stop" : "start")}>{running ? <StopIcon /> : <PlayIcon />}</IconButton></div>
      </div>
    </aside>
    <div className="workspace"><PromoBar />{children}</div>
  </div>;
}

function Stat({ label, value, accent }: { label: string; value: string; accent?: string }) {
  return <div className={`mini-stat${accent ? ` ${accent}` : ""}`}><span>{label}</span><strong>{value}</strong></div>;
}
