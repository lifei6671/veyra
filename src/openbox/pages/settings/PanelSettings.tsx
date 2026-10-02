import { useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import {
  AdjustmentsHorizontalIcon,
  ArrowUpTrayIcon,
  ArrowUturnLeftIcon,
  QuestionMarkCircleIcon,
  XMarkIcon,
} from "@heroicons/react/24/outline";
import { api } from "../../api/client";
import type { StorageResponse, TestSite } from "../../api/types";
import baiduLogo from "../../assets/baidu.svg";
import googleLogo from "../../assets/google.svg";
import openaiLogo from "../../assets/openai.svg";
import telegramLogo from "../../assets/telegram.svg";
import { CompactSetting, Modal } from "../../components/shared";
import { SelectControl, SwitchControl } from "../../ui/controls";
import { PanelIconPicker, panelIcons } from "./PanelIconPicker";

type EditableTestSite = TestSite & { iconKey: string };

const defaults: EditableTestSite[] = [
  { id: "baidu", iconKey: "brand:baidu", icon: baiduLogo, name: "百度", url: "https://www.baidu.com/favicon.ico" },
  { id: "google", iconKey: "brand:google", icon: googleLogo, name: "Google", url: "https://www.google.com/generate_204" },
  { id: "openai", iconKey: "brand:openai-light", icon: openaiLogo, name: "OpenAI", url: "https://api.openai.com/v1/models" },
  { id: "telegram", iconKey: "brand:telegram", icon: telegramLogo, name: "Telegram", url: "https://telegram.org/favicon.ico" },
];

export function PanelSettings({ storage, setTheme, onSaved }: {
  storage: StorageResponse;
  theme: "light" | "dark";
  setTheme: (theme: "light" | "dark") => void;
  onSaved: (message: string) => void;
}) {
  const entries = storage.entries;
  const [radius, setRadius] = useState(Number(entries["config/global-radius"] ?? 16));
  const [background, setBackground] = useState(entries["config/custom-background-image"] ?? "");
  const [backgroundExpanded, setBackgroundExpanded] = useState(false);
  const [backgroundOpacity, setBackgroundOpacity] = useState(Number(entries["config/dashboard-transparent"] ?? 90));
  const [backgroundBlur, setBackgroundBlur] = useState(Number(entries["config/blur-intensity"] ?? 10));
  const [sites, setSites] = useState<EditableTestSite[]>(() => parseSites(entries["config/test-sites"]));
  const [passwordOpen, setPasswordOpen] = useState(false);
  const [restoreOpen, setRestoreOpen] = useState(false);
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);
  const saveQueue = useRef<Promise<void>>(Promise.resolve());
  const pendingSaves = useRef(0);
  const saveTimers = useRef(new Map<string, ReturnType<typeof setTimeout>>());

  useEffect(() => setRadius(Number(entries["config/global-radius"] ?? 16)), [entries]);
  useEffect(() => setBackground(entries["config/custom-background-image"] ?? ""), [entries]);
  useEffect(() => setBackgroundOpacity(Number(entries["config/dashboard-transparent"] ?? 90)), [entries]);
  useEffect(() => setBackgroundBlur(Number(entries["config/blur-intensity"] ?? 10)), [entries]);
  useEffect(() => setSites(parseSites(entries["config/test-sites"])), [entries]);

  useEffect(() => {
    if (!passwordOpen && !restoreOpen) return;
    const close = (event: KeyboardEvent) => { if (event.key === "Escape") { setPasswordOpen(false); setRestoreOpen(false); } };
    document.addEventListener("keydown", close);
    return () => document.removeEventListener("keydown", close);
  }, [passwordOpen, restoreOpen]);

  const save = (key: string, value: string) => {
    clearTimeout(saveTimers.current.get(key));
    saveTimers.current.delete(key);
    pendingSaves.current += 1;
    setBusy(true);
    const task = saveQueue.current.then(() => api.patchStorage({ [key]: value })).then(
      () => onSaved("面板设置已保存"),
      reason => onSaved(reason instanceof Error ? reason.message : "面板设置保存失败"),
    ).finally(() => {
      pendingSaves.current -= 1;
      if (pendingSaves.current === 0) setBusy(false);
    });
    saveQueue.current = task;
    return task;
  };

  const saveBackgroundReference = async (value: string) => {
    if (value === (entries["config/custom-background-image"] ?? "")) return;
    try {
      if (!value.includes("local-image")) await api.deleteBackgroundImage();
      await save("config/custom-background-image", value);
    } catch (reason) {
      onSaved(reason instanceof Error ? reason.message : "面板背景保存失败");
    }
  };

  const scheduleSave = (key: string, value: string) => {
    clearTimeout(saveTimers.current.get(key));
    saveTimers.current.set(key, setTimeout(() => {
      saveTimers.current.delete(key);
      if (key === "config/custom-background-image") void saveBackgroundReference(value);
      else void save(key, value);
    }, 400));
  };

  const uploadBackground = async (file?: File) => {
    if (!file) return;
    if (!file.type.startsWith("image/")) { onSaved("请选择图片文件"); return; }
    setBusy(true);
    try {
      const image = await readFileAsDataUrl(file);
      await api.saveBackgroundImage(image);
      const reference = `local-image-${Date.now()}`;
      setBackground(reference);
      await save("config/custom-background-image", reference);
      setBackgroundExpanded(true);
    } catch (reason) {
      onSaved(reason instanceof Error ? reason.message : "面板背景上传失败");
    } finally {
      if (fileInput.current) fileInput.current.value = "";
      if (pendingSaves.current === 0) setBusy(false);
    }
  };

  const updateSite = (id: string, field: "name" | "url", value: string) => {
    const next = sites.map(site => site.id === id ? { ...site, [field]: value } : site);
    setSites(next);
    scheduleSave("config/test-sites", JSON.stringify(next.map(({ id: siteId, name, url, iconKey }) => ({ id: siteId, name, url, icon: iconKey }))));
  };
  const saveSites = async (nextSites = sites) => {
    const payload = nextSites.map(({ id, name, url, iconKey }) => ({ id, name, url, icon: iconKey }));
    await save("config/test-sites", JSON.stringify(payload));
  };

  const changePassword = async () => {
    if (newPassword.length < 4) { onSaved("密码至少 4 位。"); return; }
    if (newPassword !== confirmPassword) { onSaved("两次输入的新密码不一致"); return; }
    setBusy(true);
    try {
      await api.changePassword(currentPassword, newPassword);
      setPasswordOpen(false);
      setCurrentPassword(""); setNewPassword(""); setConfirmPassword("");
      onSaved("访问密码已修改");
    } catch (reason) {
      onSaved(reason instanceof Error ? reason.message : "密码修改失败");
    } finally { setBusy(false); }
  };

  return <div className="panel-settings">
    <section className="surface settings-block general-block">
      <h2>通用</h2>
      <div className="panel-settings-grid">
        <CompactSetting label="面板语言"><SelectControl className="panel-select language-select" label="面板语言" value={entries["config/language"] ?? "zh-CN"} onValueChange={value => { localStorage.setItem("config/language", value); document.documentElement.lang = value; void save("config/language", value); }} options={[{ value: "en-US", label: "English" }, { value: "zh-CN", label: "简体中文" }, { value: "zh-TW", label: "繁體中文" }]} /></CompactSetting>
        <CompactSetting label="面板背景">
          <div className="background-setting-control" onDragOver={event => event.preventDefault()} onDrop={event => { event.preventDefault(); void uploadBackground(event.dataTransfer.files[0]); }}>
            <div className="background-upload-group"><div className="compact-background"><input aria-label="面板背景" value={background} onChange={event => { setBackground(event.target.value); setBackgroundExpanded(Boolean(event.target.value)); scheduleSave("config/custom-background-image", event.target.value); }} />{background && <button type="button" aria-label="清空面板背景" disabled={busy} onMouseDown={event => event.preventDefault()} onClick={() => { setBackground(""); setBackgroundExpanded(false); void saveBackgroundReference(""); }}><XMarkIcon /></button>}</div><button className="background-action" type="button" aria-label="上传面板背景" disabled={busy} onClick={() => fileInput.current?.click()}><ArrowUpTrayIcon /></button></div>
            {background && <button className={`background-action background-adjust${backgroundExpanded ? " active" : ""}`} type="button" aria-label="调整面板背景" aria-expanded={backgroundExpanded} onClick={() => setBackgroundExpanded(current => !current)}><AdjustmentsHorizontalIcon /></button>}
            <input ref={fileInput} className="background-file-input" type="file" accept="image/*" onChange={event => void uploadBackground(event.target.files?.[0])} />
          </div>
        </CompactSetting>
        {backgroundExpanded && background && <>
          <CompactSetting label="透明度"><input className="panel-range" aria-label="透明度" type="range" min={0} max={100} value={backgroundOpacity} onChange={event => { const value = Number(event.target.value); setBackgroundOpacity(value); scheduleSave("config/dashboard-transparent", String(value)); }} /></CompactSetting>
          <CompactSetting label="毛玻璃强度"><input className="panel-range" aria-label="毛玻璃强度" type="range" min={0} max={40} value={backgroundBlur} onChange={event => { const value = Number(event.target.value); setBackgroundBlur(value); scheduleSave("config/blur-intensity", String(value)); }} /></CompactSetting>
        </>}
        <CompactSetting label="全局圆角"><div className="stepper"><button type="button" aria-label="减小全局圆角" disabled={radius <= 0} onClick={() => { const value = Math.max(0, radius - 1); setRadius(value); void save("config/global-radius", String(value)); }}>-</button><b>{radius}px</b><button type="button" aria-label="增大全局圆角" disabled={radius >= 24} onClick={() => { const value = Math.min(24, radius + 1); setRadius(value); void save("config/global-radius", String(value)); }}>+</button></div></CompactSetting>
        <CompactSetting label="主题"><SelectControl className="panel-select theme-select" label="主题" value={entries["config/theme-mode"] ?? "system"} onValueChange={value => { const next = value === "system" ? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light") : value as "light" | "dark"; setTheme(next); void save("config/theme-mode", value); }} options={[{ value: "system", label: "跟随系统" }, { value: "light", label: "亮色" }, { value: "dark", label: "暗色" }]} /></CompactSetting>
        <CompactSetting label="修改密码"><button className="compact-button" type="button" onClick={() => { setCurrentPassword(""); setNewPassword(""); setConfirmPassword(""); setPasswordOpen(true); }}>修改密码</button></CompactSetting>
        <div className="compact-setting"><span>IP信息API <IpInfoTooltip /></span><div><SelectControl className="panel-select ip-api-select" label="IP信息API" value={entries["config/geoip-info-api"] ?? "ip.sb"} onValueChange={value => void save("config/geoip-info-api", value)} options={[{ value: "ip.sb", label: "ip.sb" }, { value: "ipwho.is", label: "ipwho.is" }, { value: "ipapi.is", label: "ipapi.is" }]} /></div></div>
      </div>
    </section>

    <section className="surface settings-block latency-block">
      <h2>延迟</h2>
      <div className="panel-settings-grid">
        <NumberSetting label="测速超时" storageKey="config/speedtest-timeout" initial={Number(entries["config/speedtest-timeout"] ?? 5000)} suffix="ms" save={scheduleSave} />
        <NumberSetting label="黄色的阈值" storageKey="config/low-latency" initial={Number(entries["config/low-latency"] ?? 400)} suffix="ms" save={scheduleSave} />
        <NumberSetting label="红色的阈值" storageKey="config/medium-latency" initial={Number(entries["config/medium-latency"] ?? 800)} suffix="ms" save={scheduleSave} />
        <CompactSetting label="IPv6 测试"><SwitchControl label="IPv6 测试" checked={entries["config/ipv6-test"] === "true"} onCheckedChange={value => void save("config/ipv6-test", String(value))} /></CompactSetting>
        <CompactSetting label="隐藏不可用节点"><SwitchControl label="隐藏不可用节点" checked={entries["config/hide-unavailable-proxies"] === "true"} onCheckedChange={value => void save("config/hide-unavailable-proxies", String(value))} /></CompactSetting>
      </div>
      <h2 className="layout-title">布局</h2>
      <div className="panel-settings-grid"><CompactSetting label="代理组分列"><SelectControl className="panel-select group-columns-select" label="代理组分列" value={entries["config/proxy-group-columns"] ?? (entries["config/two-columns"] === "false" ? "1" : "2")} onValueChange={value => void save("config/proxy-group-columns", value)} options={[{ value: "1", label: "单列" }, { value: "2", label: "双列" }, { value: "3", label: "三列" }]} /></CompactSetting></div>
    </section>

    <section className="surface settings-block test-sites-block">
      <h2>测试站点 <span className="settings-info tooltip-trigger test-sites-tooltip" role="img" tabIndex={0} aria-label="测试站点说明" data-tooltip="概览里的延时小卡片和规则页右上角的快捷查询共用这四个站点。图标从图标库选;名称空着就用图标的品牌名(没有品牌名就用网址的主机名);网址填 http(s) 地址,延时按内核经当前分流访问这个地址计"><QuestionMarkCircleIcon /></span></h2>
      <button className="restore-button tooltip-trigger" type="button" aria-label="恢复默认测试站点" data-tooltip="恢复默认" onClick={() => setRestoreOpen(true)}><ArrowUturnLeftIcon /></button>
      <div className="panel-settings-grid test-sites-grid">{sites.map(site => <div className="test-site-row" key={site.id}>
        <PanelIconPicker value={site.iconKey} label={`${site.name} 图标`} onChange={iconKey => { const next = sites.map(item => item.id === site.id ? { ...item, iconKey, icon: iconAsset(iconKey) } : item); setSites(next); void saveSites(next); }} />
        <input aria-label={`${site.name} 名称`} value={site.name} onChange={event => updateSite(site.id, "name", event.target.value)} />
        <div className={`site-url-field${validTestUrl(site.url) ? "" : " invalid"}`}><input aria-label={`${site.name} 测试地址`} placeholder="https://…" value={site.url} onChange={event => updateSite(site.id, "url", event.target.value)} />{site.url && <button type="button" aria-label={`清空 ${site.name} 测试地址`} onMouseDown={event => event.preventDefault()} onClick={() => { const next = sites.map(item => item.id === site.id ? { ...item, url: "" } : item); setSites(next); void saveSites(next); }}><XMarkIcon /></button>}</div>
      </div>)}</div>
    </section>

    {restoreOpen && <Modal title="恢复默认" className="panel-settings-dialog" onClose={() => setRestoreOpen(false)} footer={<><button type="button" className="compact-button" onClick={() => setRestoreOpen(false)}>取消</button><button type="button" className="panel-confirm-button" disabled={busy} onClick={() => { setSites(defaults); setRestoreOpen(false); void saveSites(defaults); }}>确定</button></>}><p>四个站点的图标、名称和网址都会覆盖回随包默认。确定吗?</p></Modal>}
    {passwordOpen && <Modal title="修改密码" className="panel-settings-dialog" onClose={() => setPasswordOpen(false)} footer={<button type="button" className="primary-button" disabled={busy} onClick={() => void changePassword()}>修改密码</button>}><div className="form-stack" onKeyDown={event => { if (event.key === "Enter" && !busy) void changePassword(); }}><p>请先输入当前密码，再设置新密码。</p><label><span>当前密码</span><input autoFocus type="password" autoComplete="current-password" value={currentPassword} onChange={event => setCurrentPassword(event.target.value)} /></label><label><span>新密码</span><input type="password" autoComplete="new-password" value={newPassword} onChange={event => setNewPassword(event.target.value)} /><small>密码至少 4 位。</small></label><label><span>确认新密码</span><input type="password" autoComplete="new-password" value={confirmPassword} onChange={event => setConfirmPassword(event.target.value)} /></label></div></Modal>}
  </div>;
}

function NumberSetting({ label, storageKey, initial, suffix, save }: { label: string; storageKey: string; initial: number; suffix: string; save: (key: string, value: string) => void }) {
  const [value, setValue] = useState(String(initial));
  useEffect(() => setValue(String(initial)), [initial]);
  const commit = () => {
    const number = Number(value);
    if (!value.trim() || !Number.isFinite(number) || number < 0) { setValue(String(initial)); return; }

  };
  return <CompactSetting label={label}><label className="number-field"><input aria-label={label} type="number" min={0} value={value} onChange={event => { const next = event.target.value; setValue(next); if (next.trim() && Number.isFinite(Number(next)) && Number(next) >= 0) save(storageKey, String(Number(next))); }} onBlur={commit} /><span>{suffix}</span></label></CompactSetting>;
}

function parseSites(value?: string): EditableTestSite[] {
  if (!value) return defaults;
  try {
    const items = JSON.parse(value) as Array<{ id: string; icon?: string; name: string; url: string }>;
    return items.map(item => {
      const fallback = defaults.find(site => site.id === item.id) ?? defaults[0];
      const iconKey = item.icon ?? fallback.iconKey;
      return { ...item, iconKey, icon: iconAsset(iconKey) };
    });
  } catch {
    return defaults;
  }
}

function validTestUrl(value: string) {
  try { return ["http:", "https:"].includes(new URL(value).protocol); } catch { return false; }
}

function iconAsset(iconKey: string) {
  return panelIcons.find(option => option.code === iconKey)?.asset ?? "";
}

function readFileAsDataUrl(file: File) {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => typeof reader.result === "string" ? resolve(reader.result) : reject(new Error("无法读取图片"));
    reader.onerror = () => reject(reader.error ?? new Error("无法读取图片"));
    reader.readAsDataURL(file);
  });
}

const ipInfoTooltipText = "此API会用于IP检查中全球节点IP信息查询、连接详情中的IP地理信息查询、面板DNS查询中的IP地理信息查询。";

function IpInfoTooltip() {
  const id = useId();
  const [position, setPosition] = useState<{ left: number; top: number } | null>(null);
  const show = (element: HTMLElement) => {
    const rect = element.getBoundingClientRect();
    const halfWidth = Math.min(280, window.innerWidth - 32) / 2;
    setPosition({ left: Math.max(halfWidth + 16, Math.min(rect.left + rect.width / 2, window.innerWidth - halfWidth - 16)), top: rect.bottom + 6 });
  };
  useEffect(() => {
    if (!position) return;
    const close = () => setPosition(null);
    window.addEventListener("scroll", close, { capture: true, passive: true });
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
    };
  }, [position]);
  return <>
    <span className="settings-info ip-info-tooltip" role="img" tabIndex={0} aria-label="IP信息API说明" aria-describedby={position ? id : undefined} data-tooltip={ipInfoTooltipText}
      onMouseEnter={event => show(event.currentTarget)}
      onMouseLeave={event => { if (document.activeElement !== event.currentTarget) setPosition(null); }}
      onFocus={event => show(event.currentTarget)} onBlur={() => setPosition(null)}
      onKeyDown={event => { if (event.key === "Escape") setPosition(null); }}>
      <QuestionMarkCircleIcon />
    </span>
    {position && createPortal(<div id={id} role="tooltip" className="panel-ip-tooltip" style={{ left: position.left, top: position.top }}>{ipInfoTooltipText}</div>, document.body)}
  </>;
}
