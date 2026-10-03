import { useEffect, useRef, useState } from "react";
import { ArrowDownTrayIcon, ArrowUpTrayIcon, PlusIcon } from "@heroicons/react/24/outline";
import { api } from "../../api/client";
import type { BackupOptions, OpenBoxBackup, OpenBoxProfile } from "../../api/types";
import { Modal } from "../../components/shared";
import { BACKEND_TEXT as T } from "./BackendSettings.messages";
import { backendError, backendMessage, backupParts, downloadBackendJson, parseBackup } from "./BackendSettings.helpers";

const importedKey = "openbox/backup-imported";
const optionLabels = { subscriptions: T.backupIncludeSubscriptions, chainProxies: T.backupIncludeChainProxies, clientRoutes: T.backupIncludeClientRoutes, servers: T.backupIncludeServers };
export function BackendDataSettings({ profile, onImported, onToast }: { profile: OpenBoxProfile; onImported: () => Promise<void>; onToast: (message: string) => void }) {
  const [hasSubscriptions, setHasSubscriptions] = useState(false), [include, setInclude] = useState<BackupOptions>({ subscriptions: false, chainProxies: false, clientRoutes: false, servers: false });
  const [exportBusy, setExportBusy] = useState(false), [importBusy, setImportBusy] = useState(false), [diagnosticBusy, setDiagnosticBusy] = useState(false), [dragging, setDragging] = useState(false);
  const [preview, setPreview] = useState<{ file: string; backup: OpenBoxBackup } | null>(null), [mode, setMode] = useState<"replace" | "append">("replace"), [progress, setProgress] = useState("");
  const input = useRef<HTMLInputElement>(null), dragDepth = useRef(0);
  const callbacks = useRef({ onToast }); callbacks.current = { onToast };
  useEffect(() => {
    let active = true; void api.subscriptions().then(value => { if (active) setHasSubscriptions(value.length > 0); }, reason => { if (active) callbacks.current.onToast(backendError(reason)); });
    const imported = sessionStorage.getItem(importedKey); if (imported) { sessionStorage.removeItem(importedKey); callbacks.current.onToast(imported); }
    return () => { active = false; };
  }, []);
  useEffect(() => {
    if (!preview) return; const close = (event: KeyboardEvent) => { if (event.key === "Escape" && !event.defaultPrevented && !importBusy) setPreview(null); };
    window.addEventListener("keydown", close); return () => window.removeEventListener("keydown", close);
  }, [preview, importBusy]);
  const available = { subscriptions: hasSubscriptions, chainProxies: !!profile.chainProxies?.length, clientRoutes: !!profile.clientRoutes?.length, servers: !!profile.servers?.length };
  useEffect(() => { setInclude({ subscriptions: hasSubscriptions, chainProxies: !!profile.chainProxies?.length, clientRoutes: !!profile.clientRoutes?.length, servers: !!profile.servers?.length }); }, [hasSubscriptions, profile.chainProxies?.length, profile.clientRoutes?.length, profile.servers?.length]);
  const read = async (file: File) => {
    if (importBusy) return;
    try { const backup = parseBackup(await file.text()); setPreview({ file: file.name, backup }); setMode("replace"); setProgress(""); } catch { onToast(backendMessage("backupBadFile", { file: file.name })); }
  };
  const exportBackup = async () => {
    setExportBusy(true); try { const value = await api.backup(include); onToast(backendMessage("backupExported", { file: downloadBackendJson(value, "backup") })); } catch (reason) { onToast(backendMessage("backupExportFailed", { message: backendError(reason) })); } finally { setExportBusy(false); }
  };
  const importBackup = async () => {
    if (!preview) return;
    setImportBusy(true); let message: string = T.backupImportedNeedRestart;
    try {
      const result = await api.importBackup(preview.backup, mode);
      if ((result.imported.subscriptions || 0) > 0) {
        const subscriptions = preview.backup.subscriptions?.filter(value => typeof value.id === "string" && (value.url || value.urls?.length)) || [];
        let done = 0, failed = 0;
        for (const subscription of subscriptions) {
          const text = backendMessage("backupRefreshingSubscriptions", { done: done + failed, total: subscriptions.length }); setProgress(text);
          try { await api.refreshSubscription(subscription.id!); done++; } catch { failed++; }
        }
        if (subscriptions.length) message = backendMessage(failed ? "backupImportedRefreshFailed" : "backupImportedRefreshed", { n: done, failed });
      }
      setPreview(null); await onImported();
      if (result.imported.panelSettings || result.imported.backgroundImage) { sessionStorage.setItem(importedKey, message); window.setTimeout(() => location.reload(), 300); }
      else onToast(message);
      setHasSubscriptions((await api.subscriptions()).length > 0);
    } catch (reason) { onToast(backendMessage("backupImportFailed", { message: backendError(reason) })); } finally { setImportBusy(false); }
  };
  const diagnostics = async () => { setDiagnosticBusy(true); try { const value = await api.diagnostics(); onToast(backendMessage("diagnosticsExported", { file: downloadBackendJson(value, "diagnostics") })); } catch (reason) { onToast(backendMessage("diagnosticsExportFailed", { message: backendError(reason) })); } finally { setDiagnosticBusy(false); } };
  const exportedAt = preview?.backup.exportedAt ? new Date(preview.backup.exportedAt) : null;
  const dateText = exportedAt && !Number.isNaN(exportedAt.getTime()) ? `${exportedAt.getFullYear()}-${String(exportedAt.getMonth() + 1).padStart(2, "0")}-${String(exportedAt.getDate()).padStart(2, "0")} ${String(exportedAt.getHours()).padStart(2, "0")}:${String(exportedAt.getMinutes()).padStart(2, "0")}` : "—";
  return <section className="surface backend-card backend-grouped">
    <div className="backend-section backend-backup" onDragEnter={event => { event.preventDefault(); if (event.dataTransfer.types.includes("Files")) { dragDepth.current++; setDragging(true); } }} onDragOver={event => { event.preventDefault(); event.dataTransfer.dropEffect = "copy"; }} onDragLeave={event => { event.preventDefault(); dragDepth.current = Math.max(0, dragDepth.current - 1); if (!dragDepth.current) setDragging(false); }} onDrop={event => { event.preventDefault(); dragDepth.current = 0; setDragging(false); const file = event.dataTransfer.files[0]; if (file) void read(file); }}>
      <h2>{T.backupTitle}</h2><p className="backend-hint">{T.backupDescription}</p>
      <div className="backend-data-actions"><button className="primary-button" disabled={exportBusy || importBusy} onClick={() => void exportBackup()}>{exportBusy ? <span className="loading-spinner" /> : <ArrowDownTrayIcon />}{T.backupExport}</button>{(Object.keys(optionLabels) as Array<keyof BackupOptions>).map(key => <label className="backend-checkbox" data-disabled={!available[key]} key={key}><input type="checkbox" checked={include[key]} disabled={!available[key] || exportBusy || importBusy} onChange={event => setInclude(value => ({ ...value, [key]: event.target.checked }))} />{optionLabels[key]}</label>)}<span className="backend-hint">{T.backupExportHint}</span></div>
      <div className="backend-data-actions"><button className="compact-button" disabled={importBusy} onClick={() => input.current?.click()}><ArrowUpTrayIcon />{T.backupImport}</button><span className="backend-hint">{T.backupImportHint}</span><input ref={input} type="file" accept=".json,application/json" hidden onChange={event => { const file = event.target.files?.[0]; if (file) void read(file); event.target.value = ""; }} /></div>
      {dragging && <div className="backend-drop"><ArrowUpTrayIcon /><span>{T.backupDropHint}</span></div>}
    </div><div className="backend-divider" /><div className="backend-section"><div className="backend-title"><h2>{T.clientConfigTitle}</h2><span className="backend-badge">即将开放</span></div><p className="backend-hint">{T.clientConfigDescription}</p><div className="backend-data-actions"><button className="compact-button" disabled><PlusIcon />{T.clientConfigAddDevice}</button><button className="compact-button" disabled><ArrowDownTrayIcon />{T.clientConfigExport}</button></div><p className="backend-hint">{T.clientConfigExportHint}</p></div>
    <div className="backend-divider" /><div className="backend-section"><h2>{T.diagnosticsTitle}</h2><p className="backend-hint">{T.diagnosticsDescription.split("{link}")[0]}<a href="https://github.com/liandu2024/Open-Box/issues" target="_blank" rel="noreferrer">GitHub issue</a>{T.diagnosticsDescription.split("{link}")[1]}</p><div className="backend-data-actions"><button className="primary-button" disabled={diagnosticBusy} onClick={() => void diagnostics()}>{diagnosticBusy ? <span className="loading-spinner" /> : <ArrowDownTrayIcon />}{T.diagnosticsExport}</button><span className="backend-hint">{T.diagnosticsHint}</span></div></div>
    {preview && <Modal title={T.backupImportConfirmTitle} className="backend-dialog backend-import-dialog routing-dialog" onClose={() => { if (!importBusy) setPreview(null); }} footer={<><button className="compact-button" disabled={importBusy} onClick={() => setPreview(null)}>取消</button><button className="primary-button" disabled={importBusy} onClick={() => void importBackup()}>{importBusy && <span className="loading-spinner" />}确定</button></>}><p>{backendMessage("backupImportConfirm", { file: preview.file, exportedAt: dateText, parts: backupParts(preview.backup) })}</p>{Array.isArray(preview.backup.subscriptions) && <div className="backend-import-modes"><strong>{T.backupSubscriptionsModeLabel}</strong>{(["replace", "append"] as const).map(value => <label className="backend-radio" key={value}><input type="radio" name="backend-import-mode" checked={mode === value} disabled={importBusy} onChange={() => setMode(value)} /><span>{T[value === "replace" ? "backupModeReplace" : "backupModeAppend"]}<small className="backend-hint">{T[value === "replace" ? "backupModeReplaceHint" : "backupModeAppendHint"]}</small></span></label>)}<p className="backend-hint">{T.backupModeOthersHint}</p></div>}{progress && <p className="backend-hint" role="status">{progress}</p>}</Modal>}
  </section>;
}
