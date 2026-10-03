import { useEffect, useRef, useState } from "react";
import { api } from "../../api/client";
import type { OpenBoxProfile, ProfilePatch, UpdateCheck, UpdateStatus } from "../../api/types";
import { Modal, Tooltip } from "../../components/shared";
import { SelectControl, SwitchControl } from "../../ui/controls";
import { BACKEND_TEXT as T } from "./BackendSettings.messages";
import { backendError, backendMessage } from "./BackendSettings.helpers";

const activeKey = "openbox/update-active";
const channels = [{ value: "auto", label: T.obUpdateChannelAuto }, { value: "direct", label: T.obUpdateChannelDirect }, { value: "mirror", label: T.obUpdateChannelMirror }];
const cancellableStages = new Set(["starting", "probing", "downloading", "verifying", "extracting"]);
export function BackendUpdateControls({ profile, status, setStatus, persist, saving, onToast }: { profile: OpenBoxProfile; status: UpdateStatus | null; setStatus: (value: UpdateStatus) => void; persist: (patch: ProfilePatch, message?: string) => Promise<boolean>; saving: boolean; onToast: (message: string) => void }) {
  const [check, setCheck] = useState<UpdateCheck | null>(null), [busy, setBusy] = useState(false), [dialog, setDialog] = useState(false), [cancelBusy, setCancelBusy] = useState(false);
  const [pollRevision, setPollRevision] = useState(0);
  const running = status?.status.running === true, wasRunning = useRef(false), closed = useRef(false);
  const callbacks = useRef({ setStatus, onToast }); callbacks.current = { setStatus, onToast };
  const plan = { auto: profile.updates?.openbox?.auto === true, hour: profile.updates?.openbox?.hour ?? 4, days: profile.updates?.openbox?.days ?? 1, channel: profile.updates?.openbox?.channel ?? "auto", checkChannel: profile.updates?.openbox?.checkChannel ?? "auto" };
  useEffect(() => {
    let active = true, timer = 0, failures = 0;
    wasRunning.current = localStorage.getItem(activeKey) === "1" || running;
    const poll = async () => {
      let delay = 30_000;
      try {
        const next = await api.updateStatus(); if (!active) return;
        callbacks.current.setStatus(next); failures = 0;
        if (next.status.running) {
          delay = 1500; localStorage.setItem(activeKey, "1");
          if (!wasRunning.current) closed.current = false;
          if (!closed.current) setDialog(true);
        } else if (wasRunning.current) {
          localStorage.removeItem(activeKey);
          if (next.status.stage === "done") { callbacks.current.onToast(T.obUpdateDone); window.setTimeout(() => location.reload(), 1500); }
          else if (next.status.stage === "failed") callbacks.current.onToast(backendMessage("obUpdateFailed", { message: next.status.message || "" }));
          else if (next.status.stage === "cancelled") callbacks.current.onToast(T.obUpdateCancelled);
        }
        wasRunning.current = next.status.running;
      } catch {
        if (!active) return;
        if (wasRunning.current && ++failures < 45) delay = 2000;
        else { wasRunning.current = false; localStorage.removeItem(activeKey); }
      }
      if (active) timer = window.setTimeout(() => void poll(), delay);
    };
    if (pollRevision) timer = window.setTimeout(() => void poll(), 800); else void poll();
    return () => { active = false; window.clearTimeout(timer); };
  }, [running, pollRevision]);
  useEffect(() => {
    if (!dialog) return; const close = (event: KeyboardEvent) => { if (event.key === "Escape" && !event.defaultPrevented) { closed.current = true; setDialog(false); } };
    window.addEventListener("keydown", close); return () => window.removeEventListener("keydown", close);
  }, [dialog]);
  const checkUpdate = async () => { setBusy(true); try { const result = await api.checkUpdate(); setCheck(result); onToast(backendMessage(result.hasUpdate ? "obUpdateAvailableToast" : "obUpdateUpToDateToast", { latest: result.latest })); } catch (reason) { onToast(backendMessage("obUpdateCheckFailed", { message: backendError(reason) })); } finally { setBusy(false); } };
  const run = async () => {
    setBusy(true); try {
      await api.runUpdate(plan.checkChannel); localStorage.setItem(activeKey, "1"); wasRunning.current = true; closed.current = false; setDialog(true); onToast(T.obUpdateStarted);
      setPollRevision(value => value + 1);
    } catch (reason) { onToast(backendMessage("obUpdateStartFailed", { message: backendError(reason) })); } finally { setBusy(false); }
  };
  const cancel = async () => { setCancelBusy(true); try { await api.cancelUpdate(); setPollRevision(value => value + 1); } catch (reason) { onToast(backendMessage("obUpdateStartFailed", { message: backendError(reason) })); } finally { setCancelBusy(false); } };
  const savePlan = (patch: Partial<typeof plan>) => persist({ updates: { openbox: { ...plan, ...patch } } }, T.obUpdatePlanSaved);
  const stage = status?.status.stage || "", stageLabel = T[`obUpdateStage_${stage}` as keyof typeof T] || stage;
  const progress = status?.status.total && status.status.bytes != null ? Math.min(100, Math.round(status.status.bytes / status.status.total * 100)) : undefined;
  return <>
    <div className="backend-divider" /><div className="backend-update-row">
      <Tooltip label={T.obUpdateDescription}><SelectControl label="更新下载通道" value={plan.checkChannel} disabled={busy || running || saving} options={channels} onValueChange={checkChannel => void persist({ updates: { openbox: { ...plan, checkChannel } } }, "")} /></Tooltip>
      {running ? <button className="compact-button" onClick={() => { closed.current = false; setDialog(true); }}><span className="loading-spinner" />{T.obUpdateViewProgress}</button> : check?.hasUpdate ? <button className="primary-button" disabled={busy} onClick={() => void run()}>{busy && <span className="loading-spinner" />}{T.obUpdateNow}</button> : <button className="compact-button" disabled={busy} onClick={() => void checkUpdate()}>{busy && <span className="loading-spinner" />}{T.obUpdateCheck}</button>}
      {check && <><span className="backend-hint">{T.obUpdateLatest}: <span className="backend-mono">{check.latest}</span></span><span className={`backend-badge ${check.hasUpdate ? "off" : "on"}`}>{check.hasUpdate ? T.obUpdateAvailable : T.obUpdateUpToDate}</span></>}
      <span className="backend-hint">{backendMessage("obUpdateInstalledChannel", { channel: status?.channel?.mode === "mirror" ? T.obUpdateChannelMirror : T.obUpdateChannelDirect })}</span>
    </div><div className="backend-divider" /><div className="backend-update-row backend-auto-update"><span>{T.obUpdateAuto}</span><SwitchControl label={T.obUpdateAuto} checked={plan.auto} disabled={saving} onCheckedChange={auto => void savePlan({ auto })} />
      {plan.auto && <><span className="backend-hint">{T.geoUpdateEvery}</span><SelectControl label="自动更新间隔" value={String(plan.days)} disabled={saving} options={[1, 3, 7, 14, 30].map(days => ({ value: String(days), label: backendMessage("geoUpdateDays", { days }) }))} onValueChange={days => void savePlan({ days: Number(days) })} /><span className="backend-hint">{T.obUpdateAutoAt}</span><SelectControl label="自动更新时间" value={String(plan.hour)} disabled={saving} options={Array.from({ length: 24 }, (_, hour) => ({ value: String(hour), label: `${String(hour).padStart(2, "0")}:00` }))} onValueChange={hour => void savePlan({ hour: Number(hour) })} /><SelectControl label="自动更新通道" value={plan.channel} options={channels} disabled={saving} onValueChange={channel => void savePlan({ channel })} /></>}
      <span className="backend-hint">{T.obUpdateAutoHint}</span></div>
    {dialog && <Modal title={T.obUpdateDialogTitle} className="backend-dialog backend-update-dialog routing-dialog" onClose={() => { closed.current = true; setDialog(false); }} footer={<>{running && <button className="compact-button" disabled={cancelBusy || !cancellableStages.has(stage)} onClick={() => void cancel()}>{cancelBusy && <span className="loading-spinner" />}取消</button>}<button className="compact-button" onClick={() => { closed.current = true; setDialog(false); }}>关闭</button></>}><div className="backend-update-stage">{running && <span className="loading-spinner" />}<span>{stageLabel}</span>{status?.status.message && <small className="backend-hint">{status.status.message}</small>}</div><progress value={progress} max={100} /><p className="backend-hint">{T.obUpdateDialogHint}</p>{status?.logTail && <pre>{status.logTail}</pre>}</Modal>}
  </>;
}
