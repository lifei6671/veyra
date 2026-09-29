import { useEffect, useRef, useState } from "react";
import {
  AdjustmentsHorizontalIcon,
  ArrowUpTrayIcon,
  ArrowUturnLeftIcon,
  ChevronDownIcon,
  QuestionMarkCircleIcon,
} from "@heroicons/react/24/outline";
import { api } from "../../api/client";
import type { StorageResponse, TestSite } from "../../api/types";
import baiduLogo from "../../assets/baidu.svg";
import googleLogo from "../../assets/google.svg";
import openaiLogo from "../../assets/openai.svg";
import telegramLogo from "../../assets/telegram.svg";
import { CompactSetting, Modal } from "../../components/shared";
import { SelectControl, SwitchControl } from "../../ui/controls";

type EditableTestSite = TestSite & { iconKey: string };

const iconOptions = [
  { value: "brand:baidu", label: "百度", asset: baiduLogo },
  { value: "brand:google", label: "Google", asset: googleLogo },
  { value: "brand:openai-light", label: "OpenAI", asset: openaiLogo },
  { value: "brand:telegram", label: "Telegram", asset: telegramLogo },
];

const defaults: EditableTestSite[] = [
  { id: "baidu", iconKey: "brand:baidu", icon: baiduLogo, name: "百度", url: "https://www.baidu.com/favicon.ico" },
  { id: "google", iconKey: "brand:google", icon: googleLogo, name: "Google", url: "https://www.google.com/generate_204" },
  { id: "openai", iconKey: "brand:openai-light", icon: openaiLogo, name: "OpenAI", url: "https://api.openai.com/v1/models" },
  { id: "telegram", iconKey: "brand:telegram", icon: telegramLogo, name: "Telegram", url: "https://telegram.org/favicon.ico" },
];

export function PanelSettings({ storage, theme, setTheme, onSaved }: {
  storage: StorageResponse;
  theme: "light" | "dark";
  setTheme: (theme: "light" | "dark") => void;
  onSaved: (message: string) => void;
}) {
  const entries = storage.entries;
  const [radius, setRadius] = useState(Number(entries["config/global-radius"] ?? 15));
  const [background, setBackground] = useState(entries["config/custom-background-image"] ?? "");
  const [backgroundExpanded, setBackgroundExpanded] = useState(false);
  const [backgroundOpacity, setBackgroundOpacity] = useState(Number(entries["config/dashboard-transparent"] ?? 90));
  const [backgroundBlur, setBackgroundBlur] = useState(Number(entries["config/blur-intensity"] ?? 10));
  const [sites, setSites] = useState<EditableTestSite[]>(() => parseSites(entries["config/test-sites"]));
  const [passwordOpen, setPasswordOpen] = useState(false);
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);
  const saveQueue = useRef<Promise<void>>(Promise.resolve());
  const pendingSaves = useRef(0);

  useEffect(() => setRadius(Number(entries["config/global-radius"] ?? 15)), [entries]);
  useEffect(() => setBackground(entries["config/custom-background-image"] ?? ""), [entries]);
  useEffect(() => setBackgroundOpacity(Number(entries["config/dashboard-transparent"] ?? 90)), [entries]);
  useEffect(() => setBackgroundBlur(Number(entries["config/blur-intensity"] ?? 10)), [entries]);
  useEffect(() => setSites(parseSites(entries["config/test-sites"])), [entries]);

  const save = (key: string, value: string) => {
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
    const previous = entries["config/custom-background-image"] ?? "";
    if (previous.startsWith("local-image-") && value !== previous) await api.deleteBackgroundImage();
    await save("config/custom-background-image", value);
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

  const updateSite = (id: string, field: "name" | "url", value: string) => setSites(current => current.map(site => site.id === id ? { ...site, [field]: value } : site));
  const saveSites = async (nextSites = sites) => {
    const payload = nextSites.map(({ id, name, url, iconKey }) => ({ id, name, url, icon: iconKey }));
    await save("config/test-sites", JSON.stringify(payload));
  };

  const changePassword = async () => {
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
    <section className={`surface settings-block general-block${backgroundExpanded && background ? " background-expanded" : ""}`}><h2>通用</h2><div className="settings-two-columns"><div>
      <CompactSetting label="面板语言"><SelectControl label="面板语言" value={entries["config/language"] ?? "zh-CN"} onValueChange={value => { localStorage.setItem("config/language", value); document.documentElement.lang = value; void save("config/language", value); }} options={[{ value: "en-US", label: "English" }, { value: "zh-CN", label: "简体中文" }, { value: "zh-TW", label: "繁體中文" }]} /></CompactSetting>
      {backgroundExpanded && background && <CompactSetting label="透明度"><label className="background-range"><input aria-label="透明度" type="range" min={0} max={100} value={backgroundOpacity} onChange={event => setBackgroundOpacity(Number(event.target.value))} onPointerUp={() => void save("config/dashboard-transparent", String(backgroundOpacity))} onKeyUp={() => void save("config/dashboard-transparent", String(backgroundOpacity))} /></label></CompactSetting>}
      <CompactSetting label="全局圆角"><div className="stepper"><button className="tooltip-trigger" type="button" aria-label="减小全局圆角" data-tooltip="减小全局圆角" disabled={busy || radius <= 0} onClick={() => { const value = Math.max(0, radius - 1); setRadius(value); void save("config/global-radius", String(value)); }}>−</button><b>{radius}px</b><button className="tooltip-trigger" type="button" aria-label="增大全局圆角" data-tooltip="增大全局圆角" disabled={busy || radius >= 24} onClick={() => { const value = Math.min(24, radius + 1); setRadius(value); void save("config/global-radius", String(value)); }}>＋</button></div></CompactSetting>
      <CompactSetting label="修改密码"><button className="compact-button" type="button" onClick={() => setPasswordOpen(true)}>修改密码</button></CompactSetting>
    </div><div>
      <CompactSetting label="面板背景"><div className="background-setting-control" onDragOver={event => event.preventDefault()} onDrop={event => { event.preventDefault(); void uploadBackground(event.dataTransfer.files[0]); }}><div className="compact-background"><input aria-label="面板背景" value={background} onChange={event => setBackground(event.target.value)} onBlur={() => void saveBackgroundReference(background)} /><button className="tooltip-trigger" type="button" aria-label="清空面板背景" data-tooltip="清空面板背景" disabled={busy || !background} onClick={() => { setBackground(""); setBackgroundExpanded(false); void saveBackgroundReference(""); }}>×</button></div><button className="background-action tooltip-trigger" type="button" aria-label="上传面板背景" data-tooltip="上传面板背景" disabled={busy} onClick={() => fileInput.current?.click()}><ArrowUpTrayIcon /></button>{background && <button className={`background-action tooltip-trigger${backgroundExpanded ? " active" : ""}`} type="button" aria-label="调整面板背景" data-tooltip="调整面板背景" aria-expanded={backgroundExpanded} onClick={() => setBackgroundExpanded(current => !current)}><AdjustmentsHorizontalIcon /></button>}<input ref={fileInput} className="background-file-input" type="file" accept="image/*" onChange={event => void uploadBackground(event.target.files?.[0])} /></div></CompactSetting>
      {backgroundExpanded && background && <CompactSetting label="毛玻璃强度"><label className="background-range"><input aria-label="毛玻璃强度" type="range" min={0} max={40} value={backgroundBlur} onChange={event => setBackgroundBlur(Number(event.target.value))} onPointerUp={() => void save("config/blur-intensity", String(backgroundBlur))} onKeyUp={() => void save("config/blur-intensity", String(backgroundBlur))} /></label></CompactSetting>}
      <CompactSetting label="主题"><SelectControl label="主题" value={entries["config/theme-mode"] ?? theme} onValueChange={value => { const next = value === "system" ? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light") : value as "light" | "dark"; setTheme(next); void save("config/theme-mode", value); }} options={[{ value: "system", label: "跟随系统" }, { value: "light", label: "亮色" }, { value: "dark", label: "暗色" }]} /></CompactSetting>
      <CompactSetting label="IP信息API"><SelectControl label="IP信息API" value={entries["config/geoip-info-api"] ?? "ip.sb"} onValueChange={value => void save("config/geoip-info-api", value)} options={[{ value: "ip.sb", label: "ip.sb" }, { value: "ipwho.is", label: "ipwho.is" }, { value: "ipapi.is", label: "ipapi.is" }]} /></CompactSetting>
    </div></div></section>

    <section className="surface settings-block latency-block"><h2>延迟</h2><div className="settings-two-columns"><div>
      <NumberSetting label="测速超时" storageKey="config/speedtest-timeout" initial={Number(entries["config/speedtest-timeout"] ?? 5000)} suffix="ms" save={save} />
      <NumberSetting label="红色的阈值" storageKey="config/medium-latency" initial={Number(entries["config/medium-latency"] ?? 1000)} suffix="ms" save={save} />
      <CompactSetting label="隐藏不可用节点"><SwitchControl label="隐藏不可用节点" checked={entries["config/hide-unavailable-proxies"] === "true"} onCheckedChange={value => void save("config/hide-unavailable-proxies", String(value))} /></CompactSetting>
    </div><div>
      <NumberSetting label="黄色的阈值" storageKey="config/low-latency" initial={Number(entries["config/low-latency"] ?? 500)} suffix="ms" save={save} />
      <CompactSetting label="IPv6 测试"><SwitchControl label="IPv6 测试" checked={entries["config/ipv6-test"] === "true"} onCheckedChange={value => void save("config/ipv6-test", String(value))} /></CompactSetting>
    </div></div><h2 className="layout-title">布局</h2><div className="layout-setting"><span>代理组分列</span><SelectControl label="代理组分列" value={entries["config/proxy-group-columns"] ?? "1"} onValueChange={value => void save("config/proxy-group-columns", value)} options={[{ value: "1", label: "单列" }, { value: "2", label: "双列" }, { value: "3", label: "三列" }]} /></div></section>

    <section className="surface settings-block test-sites-block"><h2>测试站点 <span className="settings-info tooltip-trigger" role="img" tabIndex={0} aria-label="测试站点说明" data-tooltip="用于概览页与代理延迟测试"><QuestionMarkCircleIcon /></span></h2><button className="restore-button tooltip-trigger" type="button" aria-label="恢复默认测试站点" data-tooltip="恢复默认测试站点" onClick={() => { setSites(defaults); void saveSites(defaults); }}><ArrowUturnLeftIcon /></button><div className="test-sites-grid">{sites.map(site => <div className="test-site-row" key={site.id}><label className="site-icon-picker tooltip-trigger" data-tooltip={`选择${site.name}图标`}><img src={site.icon} alt="" /><ChevronDownIcon /><select aria-label={`${site.name} 图标`} value={site.iconKey} onChange={event => { const iconKey = event.target.value; const next = sites.map(item => item.id === site.id ? { ...item, iconKey, icon: iconAsset(iconKey) } : item); setSites(next); void saveSites(next); }}>{iconOptions.map(option => <option key={option.value} value={option.value}>{option.label}</option>)}</select></label><input aria-label={`${site.name} 名称`} value={site.name} onChange={event => updateSite(site.id, "name", event.target.value)} onBlur={() => void saveSites()} /><input aria-label={`${site.name} 测试地址`} value={site.url} onChange={event => updateSite(site.id, "url", event.target.value)} onBlur={() => void saveSites()} /><button className="tooltip-trigger tooltip-top" type="button" aria-label={`清空 ${site.name} 测试地址`} data-tooltip={`清空 ${site.name} 测试地址`} disabled={!site.url} onClick={() => { const next = sites.map(item => item.id === site.id ? { ...item, url: "" } : item); setSites(next); void saveSites(next); }}>×</button></div>)}</div></section>

    {passwordOpen && <Modal title="修改访问密码" onClose={() => setPasswordOpen(false)} footer={<><button type="button" className="compact-button" onClick={() => setPasswordOpen(false)}>取消</button><button type="button" className="primary-button" disabled={busy || newPassword.length < 8} onClick={() => void changePassword()}>保存新密码</button></>}><div className="form-stack"><label><span>当前密码</span><input type="password" autoComplete="current-password" value={currentPassword} onChange={event => setCurrentPassword(event.target.value)} /></label><label><span>新密码</span><input type="password" autoComplete="new-password" minLength={8} value={newPassword} onChange={event => setNewPassword(event.target.value)} /></label><label><span>确认新密码</span><input type="password" autoComplete="new-password" minLength={8} value={confirmPassword} onChange={event => setConfirmPassword(event.target.value)} /></label></div></Modal>}
  </div>;
}

function NumberSetting({ label, storageKey, initial, suffix, save }: { label: string; storageKey: string; initial: number; suffix: string; save: (key: string, value: string) => Promise<void> }) {
  const [value, setValue] = useState(String(initial));
  const commit = () => {
    const number = Number(value);
    if (!value.trim() || !Number.isFinite(number) || number < 0) { setValue(String(initial)); return; }
    void save(storageKey, String(number));
  };
  return <CompactSetting label={label}><label className="number-field"><input aria-label={label} type="number" min={0} value={value} onChange={event => setValue(event.target.value)} onBlur={commit} /><span>{suffix}</span></label></CompactSetting>;
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

function iconAsset(iconKey: string) {
  return iconOptions.find(option => option.value === iconKey)?.asset ?? baiduLogo;
}

function readFileAsDataUrl(file: File) {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => typeof reader.result === "string" ? resolve(reader.result) : reject(new Error("无法读取图片"));
    reader.onerror = () => reject(reader.error ?? new Error("无法读取图片"));
    reader.readAsDataURL(file);
  });
}
