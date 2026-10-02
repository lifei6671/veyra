import { useEffect, useRef, useState } from "react";
import {
  CpuChipIcon,
  DevicePhoneMobileIcon,
  HomeIcon,
  LinkIcon,
  MapIcon,
  PlusIcon,
  RectangleStackIcon,
  RssIcon,
  ServerStackIcon,
  ShareIcon,
} from "@heroicons/react/24/outline";
import type { StorageResponse } from "../api/types";
import { PanelSettings } from "./settings/PanelSettings";
import { SubscriptionSettings } from "./settings/SubscriptionSettings";
import { GroupSettings } from "./settings/GroupSettings";
import { RoutingSettings } from "./settings/RoutingSettings";
import { StructuredSettings } from "./settings/StructuredSettings";
import { DnsSettings } from "./settings/DnsSettings";
import { BackendSettings } from "./settings/BackendSettings";

export const settingsSections = [
  { id: "panel", label: "面板设置", icon: HomeIcon },
  { id: "subscriptions", label: "订阅管理", icon: RssIcon },
  { id: "groups", label: "出站节点", icon: RectangleStackIcon },
  { id: "routing", label: "目标分流", icon: MapIcon },
  { id: "clients", label: "终端分流", icon: DevicePhoneMobileIcon },
  { id: "chain", label: "链式代理", icon: LinkIcon },
  { id: "share", label: "共享网络", icon: ShareIcon },
  { id: "dns", label: "DNS 设置", icon: ServerStackIcon },
  { id: "backend", label: "后端设置", icon: CpuChipIcon },
] as const;

type SectionId = typeof settingsSections[number]["id"];

export function SettingsPage({ storage, theme, setTheme, onToast }: {
  storage: StorageResponse;
  theme: "light" | "dark";
  setTheme: (theme: "light" | "dark") => void;
  onToast: (message: string) => void;
}) {
  const [section, setSection] = useState<SectionId>(() => {
    const saved = sessionStorage.getItem("openbox:settings-section");
    return settingsSections.some(item => item.id === saved) ? saved as SectionId : "panel";
  });
  const [subscriptionAddToken, setSubscriptionAddToken] = useState(0);
  const navRef = useRef<HTMLElement>(null);
  const [navEdges, setNavEdges] = useState({ left: false, right: false });
  useEffect(() => {
    const nav = navRef.current!;
    const updateEdges = () => {
      const left = nav.scrollLeft > 1;
      const right = nav.scrollLeft + nav.clientWidth < nav.scrollWidth - 1;
      setNavEdges(previous => previous.left === left && previous.right === right ? previous : { left, right });
    };
    updateEdges();
    const observer = new ResizeObserver(updateEdges);
    observer.observe(nav);
    nav.addEventListener("scroll", updateEdges, { passive: true });
    return () => {
      observer.disconnect();
      nav.removeEventListener("scroll", updateEdges);
    };
  }, []);
  const selectSection = (next: SectionId) => {
    sessionStorage.setItem("openbox:settings-section", next);
    setSection(next);
  };
  return <main className="page settings-page">
    <div className="settings-top-nav">
      <nav ref={navRef} className="settings-nav-scroll" aria-label="设置分类" data-scroll-left={navEdges.left} data-scroll-right={navEdges.right}>{settingsSections.map(item => { const Icon = item.icon; return <button type="button" key={item.id} className={section === item.id ? "active" : ""} onClick={() => selectSection(item.id)}><Icon />{item.label}</button>; })}</nav>
      <div id="settings-header-actions" className="settings-header-actions">{section === "subscriptions" && <button type="button" className="settings-nav-add tooltip-trigger tooltip-align-right" aria-label="添加订阅或节点" data-tooltip="添加订阅或节点" onClick={() => setSubscriptionAddToken(value => value + 1)}><PlusIcon /></button>}</div>
    </div>
    <div className="settings-content">
    {section === "panel" && <PanelSettings storage={storage} theme={theme} setTheme={setTheme} onSaved={onToast} />}
    {section === "subscriptions" && <SubscriptionSettings storage={storage} onToast={onToast} addToken={subscriptionAddToken} />}
    {section === "groups" && <GroupSettings onToast={onToast} />}
    {section === "routing" && <RoutingSettings onToast={onToast} />}
    {section === "clients" && <StructuredSettings section="clients" onToast={onToast} />}
    {section === "chain" && <StructuredSettings section="chain" onToast={onToast} />}
    {section === "share" && <StructuredSettings section="share" onToast={onToast} />}
    {section === "dns" && <DnsSettings onToast={onToast} />}
    {section === "backend" && <BackendSettings onToast={onToast} />}
    </div>
  </main>;
}
