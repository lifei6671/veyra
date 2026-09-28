import { useState } from "react";
import {
  ArrowPathIcon,
  ArrowsRightLeftIcon,
  Cog6ToothIcon,
  DocumentTextIcon,
  FunnelIcon,
  GlobeAltIcon,
  HomeIcon,
} from "@heroicons/react/24/outline";
import type { StorageResponse } from "../api/types";
import { PanelSettings } from "./settings/PanelSettings";
import { SubscriptionSettings } from "./settings/SubscriptionSettings";
import { GroupSettings } from "./settings/GroupSettings";
import { RoutingSettings } from "./settings/RoutingSettings";
import { StructuredSettings } from "./settings/StructuredSettings";
import { DnsSettings } from "./settings/DnsSettings";
import { BackendSettings } from "./settings/BackendSettings";

const sections = [
  { id: "panel", label: "面板设置", icon: HomeIcon },
  { id: "subscriptions", label: "订阅管理", icon: ArrowPathIcon },
  { id: "groups", label: "出站节点", icon: GlobeAltIcon },
  { id: "routing", label: "目标分流", icon: FunnelIcon },
  { id: "clients", label: "终端分流", icon: DocumentTextIcon },
  { id: "chain", label: "链式代理", icon: ArrowsRightLeftIcon },
  { id: "share", label: "共享网络", icon: ArrowsRightLeftIcon },
  { id: "dns", label: "DNS 设置", icon: GlobeAltIcon },
  { id: "backend", label: "后端设置", icon: Cog6ToothIcon },
] as const;

type SectionId = typeof sections[number]["id"];

export function SettingsPage({ storage, theme, setTheme, onToast }: {
  storage: StorageResponse;
  theme: "light" | "dark";
  setTheme: (theme: "light" | "dark") => void;
  onToast: (message: string) => void;
}) {
  const [section, setSection] = useState<SectionId>("panel");
  return <main className="page settings-page">
    <nav className="settings-top-nav" aria-label="设置分类">{sections.map(item => { const Icon = item.icon; return <button type="button" key={item.id} className={section === item.id ? "active" : ""} onClick={() => setSection(item.id)}><Icon />{item.label}</button>; })}</nav>
    {section === "panel" && <PanelSettings storage={storage} theme={theme} setTheme={setTheme} onSaved={onToast} />}
    {section === "subscriptions" && <SubscriptionSettings onToast={onToast} />}
    {section === "groups" && <GroupSettings onToast={onToast} />}
    {section === "routing" && <RoutingSettings onToast={onToast} />}
    {section === "clients" && <StructuredSettings section="clients" onToast={onToast} />}
    {section === "chain" && <StructuredSettings section="chain" onToast={onToast} />}
    {section === "share" && <StructuredSettings section="share" onToast={onToast} />}
    {section === "dns" && <DnsSettings onToast={onToast} />}
    {section === "backend" && <BackendSettings onToast={onToast} />}
  </main>;
}
