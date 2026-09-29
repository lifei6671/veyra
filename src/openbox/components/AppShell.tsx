import type { CSSProperties, ReactNode } from "react";
import {
  ArrowPathIcon,
  ArrowsRightLeftIcon,
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
import { IconButton } from "./shared";

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

export function AppShell({ route, onNavigate, service, stats, collapsed, onCollapse, onServiceAction, onRefresh, busy, appearance, children }: {
  route: RouteKey;
  onNavigate: (route: RouteKey) => void;
  service: ServiceStatus | null;
  stats: ShellStats;
  collapsed: boolean;
  onCollapse: () => void;
  onServiceAction: (action: "start" | "stop") => void;
  onRefresh: () => void;
  busy: boolean;
  appearance: { radius: number; background: string; backgroundOpacity: number; backgroundBlur: number; nodeCardMinWidth: number };
  children: ReactNode;
}) {
  const running = service?.core.running ?? false;
  const sidebarLabel = collapsed ? "展开侧边栏" : "收起侧边栏";
  const customBackground = /^(https?:|data:|blob:)/i.test(appearance.background) ? `url(${JSON.stringify(appearance.background)})` : undefined;
  const style = { "--global-radius": `${appearance.radius}px`, "--proxy-node-card-min-width": `${appearance.nodeCardMinWidth}px`, "--panel-background-opacity": String(Math.min(100, Math.max(0, appearance.backgroundOpacity)) / 100), "--panel-background-blur": `${Math.min(40, Math.max(0, appearance.backgroundBlur))}px`, ...(customBackground ? { "--panel-background-image": customBackground } : {}) } as CSSProperties;
  return <div className={`openbox-app${collapsed ? " sidebar-collapsed" : ""}`} style={style}>
    <aside className="sidebar">
      <div className="sidebar-top"><span className="brand"><img src={openBoxLogo} alt="Open-Box" /></span><span className="sidebar-collapse-control"><IconButton label={sidebarLabel} title={undefined} onClick={onCollapse}><span className="sidebar-collapse-icon" aria-hidden="true" /></IconButton><span className="sidebar-collapse-tooltip" role="tooltip">{sidebarLabel}</span></span></div>
      <nav className="main-nav" aria-label="主导航">{routes.map(item => { const RouteIcon = item.icon; return <button key={item.key} type="button" aria-label={item.label} className={route === item.key ? "active" : ""} onClick={() => onNavigate(item.key)} aria-current={route === item.key ? "page" : undefined}><RouteIcon /><span>{item.label}</span></button>; })}</nav>
      <div className="sidebar-status">
        <div className="status-grid"><Stat label="连接数" value={String(stats.connections)} /><Stat label="内存使用" value={formatMemory(stats.memory)} /><Stat label="进站流量" value={formatBytes(stats.inbound, 2)} /><Stat label="进站速率" value={formatRate(stats.inboundRate)} /><Stat label="出站流量" value={formatBytes(stats.outbound, 2)} /><Stat label="出站速率" value={formatRate(stats.outboundRate)} /></div>
        <div className="sidebar-status-footer">
          <div className="uptime"><span>运行时长</span><strong>{running ? formatDuration(service?.core.uptimeSeconds ?? 0) : "内核已停止"}</strong></div>
          <div className="sidebar-actions"><IconButton label="启动内核" disabled={busy || running} onClick={() => onServiceAction("start")}><PlayIcon /></IconButton><IconButton label="停止内核" disabled={busy || !running} onClick={() => onServiceAction("stop")}><StopIcon /></IconButton><IconButton label="刷新数据" disabled={busy} onClick={onRefresh}><ArrowPathIcon /></IconButton></div>
        </div>
      </div>
    </aside>
    <div className="workspace">{children}</div>
  </div>;
}

function Stat({ label, value }: { label: string; value: string }) {
  return <div className="mini-stat"><span>{label}</span><strong>{value}</strong></div>;
}

function formatMemory(value: number) {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  return `${(value / 1024 / 1024).toFixed(1).replace(/\.0$/, "")} MiB`;
}
