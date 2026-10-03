import { useEffect, useRef, useState, type InputHTMLAttributes } from "react";
import { ArrowPathIcon, ArrowTopRightOnSquareIcon, CircleStackIcon, CpuChipIcon, ExclamationTriangleIcon, MapIcon, PlayIcon, StopIcon } from "@heroicons/react/24/outline";
import { api } from "../../api/client";
import type { OpenBoxProfile, ProfilePatch, ServiceStatus, TrafficUsage, UpdateStatus } from "../../api/types";
import { ErrorState, IconButton, Modal, Tooltip } from "../../components/shared";
import { SelectControl, SegmentedGroup, SegmentedItem, SwitchControl } from "../../ui/controls";
import openBoxLogo from "../../assets/openbox-logo.png";
import { BackendDataSettings } from "./BackendDataSettings";
import { BackendUpdateControls } from "./BackendUpdateControls";
import { BACKEND_TEXT as T } from "./BackendSettings.messages";
import { backendError, backendMessage, kernelActionMessage, normalizeTestUrl, retentionForecast, retentionMonths, runKernelAction, storageSize, validBypassPorts } from "./BackendSettings.helpers";

function GithubIcon() {
  return <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" /></svg>;
}
function BackendInput({ value, onCommit, normalize = value => value.trim(), retainOnFailure = false, ...props }: Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "onChange"> & { value: string | number; onCommit: (value: string) => Promise<boolean>; normalize?: (value: string) => string; retainOnFailure?: boolean }) {
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);
  const commit = async () => {
    const next = normalize(draft); setDraft(next);
    if (next !== String(value) && !await onCommit(next) && !retainOnFailure) setDraft(String(value));
  };
  return <input {...props} value={draft} onChange={event => setDraft(event.target.value)} onBlur={() => void commit()} onKeyDown={event => { if (event.key === "Enter") event.currentTarget.blur(); }} />;
}
export function BackendSettings({ onToast }: { onToast: (message: string) => void }) {
  const [profile, setProfile] = useState<OpenBoxProfile | null>(null), [service, setService] = useState<ServiceStatus | null>(null), [update, setUpdate] = useState<UpdateStatus | null>(null), [version, setVersion] = useState(""), [usage, setUsage] = useState<TrafficUsage | null>(null);
  const [error, setError] = useState<unknown>(null), [saving, setSaving] = useState(false), [actionBusy, setActionBusy] = useState<string | null>(null), [resetOpen, setResetOpen] = useState(false), [resetBusy, setResetBusy] = useState(false), [countdown, setCountdown] = useState(10);
  const callbacks = useRef({ onToast }); callbacks.current = { onToast };
  const load = async () => { const value = await api.profile(); setProfile(value); setError(null); };
  useEffect(() => {
    let active = true;
    void api.profile().then(value => { if (active) setProfile(value); }, reason => { if (active) setError(reason); });
    void api.kernelVersion().then(value => { if (active) setVersion(value.version); }, reason => { if (active) callbacks.current.onToast(backendMessage("kernelLoadFailed", { message: backendError(reason) })); });
    void api.trafficUsage().then(value => { if (active) setUsage(value); }).catch(() => undefined);
    const status = () => { void api.serviceStatus().then(value => { if (active) setService(value); }, reason => { if (active) callbacks.current.onToast(backendMessage("kernelLoadFailed", { message: backendError(reason) })); }); };
    status(); const timer = window.setInterval(status, 10_000);
    return () => { active = false; window.clearInterval(timer); };
  }, []);
  useEffect(() => { if (!resetOpen) return; setCountdown(10); const timer = window.setInterval(() => setCountdown(value => Math.max(0, value - 1)), 1000); return () => window.clearInterval(timer); }, [resetOpen]);
  useEffect(() => {
    const escape = (event: KeyboardEvent) => { if (event.key === "Escape" && !event.defaultPrevented && !document.querySelector('[data-slot="select-content"]') && !resetBusy) setResetOpen(false); };
    window.addEventListener("keydown", escape); return () => window.removeEventListener("keydown", escape);
  }, [resetBusy]);
  const persist = async (patch: ProfilePatch, message: string = T.routingPolicySaved) => {
    setSaving(true); try { setProfile(await api.saveProfile(patch)); if (message) onToast(message); return true; } catch (reason) { onToast(backendMessage("routingSaveFailed", { message: backendError(reason) })); return false; } finally { setSaving(false); }
  };
  const action = async (value: "start" | "stop" | "restart") => {
    setActionBusy(value);
    try {
      const result = await runKernelAction(value); onToast(kernelActionMessage(value, result));
      const until = Date.now() + 6000;
      do { const next = await api.serviceStatus(); setService(next); if (next.core.running === (value !== "stop") || !result.ok) break; await new Promise(resolve => window.setTimeout(resolve, 400)); } while (Date.now() < until);
    } catch (reason) { onToast(backendMessage("kernelActionRequestFailed", { message: backendError(reason) })); } finally { setActionBusy(null); }
  };
  const reset = async () => { if (countdown || resetBusy) return; setResetBusy(true); try { await api.factoryReset(); onToast(T.factoryResetDone); window.setTimeout(() => location.reload(), 1200); } catch (reason) { onToast(backendMessage("factoryResetFailed", { message: backendError(reason) })); setResetBusy(false); } };
  if (error) return <ErrorState title={T.kernelSettings} error={error} onRetry={() => void load().catch(setError)} />;
  if (!profile) return <div className="surface backend-loading"><span className="loading-spinner" />正在读取后端设置…</div>;
  const disabled = saving || !!actionBusy || resetBusy;
  const ipv6Policy = profile.ipv6Proxy === "ipv4" || profile.ipv6Proxy === "bypass" ? profile.ipv6Proxy : "node";
  return <div className="backend-settings routing-policy-stack">
    <section className="surface backend-card backend-service-card">
      <header><Tooltip label={T.openboxGithubHint}><a className="backend-brand" href="https://github.com/liandu2024/Open-Box" target="_blank" rel="noreferrer" aria-label="Open-Box GitHub"><img src={openBoxLogo} alt="OPEN-BOX" /><span className="backend-mono">{update?.version || "—"}</span><GithubIcon /><span>GitHub</span><ArrowTopRightOnSquareIcon /></a></Tooltip><IconButton label={T.factoryReset} className="backend-reset" tooltip portalTooltip disabled={disabled} onClick={() => setResetOpen(true)}><ExclamationTriangleIcon /></IconButton></header>
      {!!service?.conflicts.length && <div className="backend-conflicts"><strong>{T.kernelConflictTitle}</strong>{service.conflicts.map(item => <p key={item.id}>{backendMessage("kernelConflictItem", { name: item.label })}</p>)}</div>}
      <div className="backend-version-row"><CpuChipIcon /><span>{T.kernelVersionLabel}: <strong>{version || T.kernelVersionUnknown}</strong></span><Tooltip label={T.singboxGithubHint}><a href="https://github.com/SagerNet/sing-box" target="_blank" rel="noreferrer"><GithubIcon />sing-box GitHub<ArrowTopRightOnSquareIcon /></a></Tooltip></div>
      <div className="backend-version-row backend-geo-row">{update?.geoCounts && <><span><MapIcon />{backendMessage("geoSiteCount", { count: update.geoCounts.geosite })}</span><span><CircleStackIcon />{backendMessage("geoIpCount", { count: update.geoCounts.geoip })}</span></>}{update?.geoDate && <span>{update.geoDate}</span>}<Tooltip label={T.geoGithubHint}><a href="https://github.com/MetaCubeX/meta-rules-dat/tree/sing/geo" target="_blank" rel="noreferrer"><GithubIcon />{T.geoGithubLabel}<ArrowTopRightOnSquareIcon /></a></Tooltip></div>
      <div className="backend-divider" /><div className="backend-status-row">{([ [T.kernelCoreLabel, service?.core.running], [T.kernelPanelLabel, service?.panel.running], [T.kernelAutostartLabel, service?.core.autostart] ] as const).map(([label, enabled], index) => <span key={label}>{label}<span className={`backend-badge ${service ? enabled ? "on" : "off" : ""}`}>{service ? index === 2 ? enabled ? T.kernelAutostartOn : T.kernelAutostartOff : enabled ? T.kernelStatusRunning : T.kernelStatusStopped : "—"}</span></span>)}</div>
      <p className="backend-hint">{T.kernelApplyHint}</p><div className="backend-service-actions"><button className="compact-button" disabled={disabled || !service || service.core.running || !!service.conflicts.length} onClick={() => void action("start")}>{actionBusy === "start" ? <span className="loading-spinner" /> : <PlayIcon />}{T.kernelActionStart}</button><Tooltip label={T.kernelActionStopHint}><button className="compact-button" disabled={disabled || !service?.core.running} onClick={() => void action("stop")}>{actionBusy === "stop" ? <span className="loading-spinner" /> : <StopIcon />}{T.kernelActionStop}</button></Tooltip><button className="compact-button" disabled={disabled || !service?.core.running || !!service.conflicts.length} onClick={() => void action("restart")}>{actionBusy === "restart" ? <span className="loading-spinner" /> : <ArrowPathIcon />}{T.kernelActionRestart}</button></div>
      <BackendUpdateControls profile={profile} status={update} setStatus={setUpdate} persist={persist} saving={disabled} onToast={onToast} />
    </section>
    <section className="surface backend-card backend-grouped">
      {([ ["directForNodes", T.nodeDirectTitle, T.nodeDirectDescription], ["directBypass", T.directBypassTitle, T.directBypassDescription], ["rejectQuic", T.blockQuicTitle, T.blockQuicDescription] ] as const).map(([field, title, description]) => <div className="backend-section" key={field}><div className="backend-title"><h2>{title}</h2><SwitchControl label={title} checked={field === "rejectQuic" ? profile[field] === true : profile[field] !== false} disabled={disabled} onCheckedChange={value => void persist({ [field]: value })} /></div><p className="backend-hint">{description}</p></div>)}
      <div className="backend-section"><div className="backend-ports-heading"><div><h2>{T.bypassPortsTitle}</h2><p className="backend-hint">{T.bypassPortsDescription}</p></div><SegmentedGroup aria-label="端口放行模式" value={profile.bypassPortsMode === "whitelist" ? "whitelist" : "blacklist"} disabled={disabled} onValueChange={bypassPortsMode => void persist({ bypassPortsMode })}><SegmentedItem value="blacklist">{T.bypassPortsModeBlacklist}</SegmentedItem><SegmentedItem value="whitelist">{T.bypassPortsModeWhitelist}</SegmentedItem></SegmentedGroup></div><BackendInput aria-label={T.bypassPortsTitle} className="backend-mono" spellCheck={false} retainOnFailure value={profile.bypassPorts || ""} placeholder={T.bypassPortsPlaceholder} disabled={disabled} onCommit={value => { if (!validBypassPorts(value)) { onToast(T.bypassPortsInvalid); return Promise.resolve(false); } return persist({ bypassPorts: value }); }} /><p className="backend-hint">{profile.bypassPortsMode === "whitelist" ? T.bypassPortsWhitelistHint : T.bypassPortsHint}</p></div>
      <div className="backend-section"><div className="backend-title"><h2>IPv6</h2><SwitchControl label="IPv6" checked={profile.ipv6 === true} disabled={disabled} onCheckedChange={ipv6 => void persist({ ipv6 })} /></div><p className="backend-hint">{T.ipv6Description}</p>{profile.ipv6 && <div className="backend-ipv6-policy"><div><span>{T.ipv6ProxyTitle}</span><p className="backend-hint">{T[ipv6Policy === "ipv4" ? "ipv6ProxyIpv4Note" : ipv6Policy === "bypass" ? "ipv6ProxyBypassNote" : "ipv6ProxyNodeNote"]}</p></div><SelectControl label={T.ipv6ProxyTitle} value={ipv6Policy} disabled={disabled} options={[{ value: "node", label: T.ipv6ProxyNode }, { value: "ipv4", label: T.ipv6ProxyIpv4 }, { value: "bypass", label: T.ipv6ProxyBypass }]} onValueChange={ipv6Proxy => void persist({ ipv6Proxy })} /></div>}</div>
      <div className="backend-section backend-tun"><h2>{T.tunOptionsTitle}</h2><p className="backend-hint">{T.tunOptionsDescription}</p><div className="backend-tun-row"><span>{T.tunStackLabel}</span><SelectControl label={T.tunStackLabel} value={profile.tun?.stack || "mixed"} disabled={disabled} options={["mixed", "gvisor", "system"].map(value => ({ value, label: value }))} onValueChange={stack => void persist({ tun: { stack } })} /></div><p className="backend-hint">{T.tunStackNote}</p>{([ ["mtu", "MTU", T.tunMtuNote, 65535], ["tcpMss", T.tunMssLabel, T.tunMssNote, 65495] ] as const).map(([field, label, hint, max]) => <div className="backend-tun-field" key={field}><div className="backend-tun-row"><span>{label}</span><BackendInput aria-label={label} type="number" min={0} max={max} value={profile.tun?.[field] || 0} disabled={disabled} normalize={value => String(Math.max(0, Number.parseInt(value, 10) || 0))} onCommit={value => persist({ tun: { [field]: Number(value) } })} /></div><p className="backend-hint">{hint}</p></div>)}</div>
      <div className="backend-section"><h2>{T.testUrlTitle}</h2><p className="backend-hint">{T.testUrlDescription}</p><div className="backend-test-urls">{([ ["testUrl", T.testUrlLabel, T.testUrlHint], ["directTestUrl", T.directTestUrl, T.directTestUrlHint] ] as const).map(([field, label, hint]) => <label key={field}><span>{label}</span><BackendInput aria-label={label} type="url" className="backend-mono" spellCheck={false} retainOnFailure value={profile[field] || normalizeTestUrl("", field === "directTestUrl")} disabled={disabled} normalize={value => normalizeTestUrl(value, field === "directTestUrl")} onCommit={value => persist({ [field]: value }, "")} /><small className="backend-hint">{hint}</small></label>)}</div></div>
      <div className="backend-section"><h2>{T.trafficRetentionTitle}</h2><p className="backend-hint">{T.trafficRetentionDescription}</p><div className="backend-retention"><BackendInput aria-label={T.trafficRetentionTitle} type="number" min={1} max={36} value={profile.traffic?.keepMonths ?? 3} disabled={disabled} normalize={value => String(retentionMonths(value))} onCommit={value => persist({ traffic: { ...profile.traffic, keepMonths: Number(value) } })} /><span>{T.trafficRetentionUnit}</span><span className="backend-hint">{T.trafficRetentionRange}</span></div><p className="backend-hint">{usage?.days ? <>{backendMessage("trafficRetentionUsage", { days: usage.days, size: storageSize(usage.bytes), perDay: storageSize(usage.perDay) })} {usage.perDay ? backendMessage("trafficRetentionForecast", { size: storageSize(retentionForecast(usage, profile.traffic?.keepMonths ?? 3)) }) : ""}</> : T.trafficRetentionNoData}</p></div>
    </section>
    <BackendDataSettings profile={profile} onImported={load} onToast={onToast} />
    {resetOpen && <Modal title={T.factoryReset} className="backend-dialog backend-reset-dialog routing-dialog" onClose={() => { if (!resetBusy) setResetOpen(false); }} footer={<><button className="compact-button" disabled={resetBusy} onClick={() => setResetOpen(false)}>取消</button><button className="backend-danger-button" disabled={resetBusy || countdown > 0} onClick={() => void reset()}>{resetBusy && <span className="loading-spinner" />}{countdown ? backendMessage("confirmIn", { seconds: countdown }) : "确定"}</button></>}><p>{T.factoryResetConfirm}</p><p className="backend-hint">{T.factoryResetKeepHint}</p></Modal>}
  </div>;
}
