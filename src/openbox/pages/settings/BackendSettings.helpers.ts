import { api } from "../../api/client";
import type { OpenBoxBackup, ServiceActionResult, TrafficUsage } from "../../api/types";
import { BACKEND_TEXT as T } from "./BackendSettings.messages";

export function backendMessage(key: keyof typeof T, params: Record<string, string | number> = {}) {
  return T[key].replace(/\{(\w+)\}/g, (token, name: string) => String(params[name] ?? token));
}
export const backendError = (reason: unknown) => reason instanceof Error ? reason.message : String(reason);
export const validBypassPorts = (value: string) => !value.trim() || /^\d{1,5}(-\d{1,5})?([,\s]+\d{1,5}(-\d{1,5})?)*[,\s]*$/.test(value.trim());
export const retentionMonths = (value: string | number) => Math.max(1, Math.min(36, Math.floor(Number(value)) || 3));
export function normalizeTestUrl(value: string, direct = false) {
  const fallback = direct ? "http://connectivitycheck.platform.hicloud.com/generate_204" : "http://www.gstatic.com/generate_204";
  return value.trim() || fallback;
}
export function retentionForecast(usage: TrafficUsage, months: number) {
  const days = 30 * months, recentDays = Math.min(days, usage.collapseAfterDays || days);
  return usage.perDay * recentDays + (usage.oldPerDay ?? usage.perDay) * (days - recentDays) + (usage.hourPerDay || 0) * (usage.hourKeepDays || 7);
}
export function storageSize(bytes: number) {
  const mb = bytes / 1e6;
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb ? mb.toFixed(mb >= 10 ? 0 : 1) : 0} MB`;
}
export function parseBackup(text: string): OpenBoxBackup {
  const value: unknown = JSON.parse(text);
  if (!value || typeof value !== "object" || !("format" in value) || value.format !== "open-box-backup" || !("profile" in value) || !value.profile || typeof value.profile !== "object" || Array.isArray(value.profile)) throw new Error("INVALID_BACKUP");
  return value as OpenBoxBackup;
}
export function backupParts(backup: OpenBoxBackup) {
  let parts: string = T.backupPartsBase;
  if (Array.isArray(backup.subscriptions)) parts += backendMessage("backupPartsSubscriptions", { subs: backup.subscriptions.length, nodes: Array.isArray(backup.nodes) ? backup.nodes.length : 0 });
  for (const [field, key] of [["chainProxies", "backupPartsChainProxies"], ["clientRoutes", "backupPartsClientRoutes"], ["servers", "backupPartsServers"]] as const) {
    const items = backup.profile[field]; if (Array.isArray(items)) parts += backendMessage(key, { n: items.length });
  }
  if (backup.panelSettings) parts += backup.backgroundImage ? T.backupPartsPanelWithBackground : T.backupPartsPanel;
  return parts;
}
export function downloadBackendJson(value: unknown, kind: "backup" | "diagnostics") {
  const now = new Date(), pad = (n: number) => String(n).padStart(2, "0");
  const file = `open-box-${kind}-${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}.json`;
  const url = URL.createObjectURL(new Blob([JSON.stringify(value, null, 2)], { type: "application/json" }));
  const anchor = document.createElement("a"); anchor.href = url; anchor.download = file; document.body.appendChild(anchor); anchor.click(); anchor.remove(); window.setTimeout(() => URL.revokeObjectURL(url), 1000);
  return file;
}
// Restarting the panel may disconnect the original request; deployment state is the backend's completion signal.
export async function runKernelAction(action: "start" | "stop" | "restart"): Promise<ServiceActionResult> {
  const started = Date.now();
  try { return await api.serviceAction(action); } catch (reason) {
    if (action === "stop") throw reason;
    while (Date.now() - started < 90_000) {
      await new Promise(resolve => window.setTimeout(resolve, 1500));
      try {
        const state = await api.deployState();
        if (state.at < started || state.stage === "idle") continue;
        return { ok: state.stage === "running", code: state.stage === "running" ? 0 : 1, stderr: state.message, warning: state.warning, durationMs: Date.now() - started };
      } catch { /* Panel is still restarting; keep waiting within the original bounded recovery window. */ }
    }
    throw reason;
  }
}
export function kernelActionMessage(action: "start" | "stop" | "restart", result: ServiceActionResult) {
  const label = T[action === "start" ? "kernelActionStart" : action === "stop" ? "kernelActionStop" : "kernelActionRestart"];
  if (!result.ok) return backendMessage("kernelActionFailed", { action: label, detail: result.stderr?.trim() || backendMessage("kernelActionNoDetail", { code: result.code }) });
  const message = result.durationMs !== undefined ? backendMessage("kernelActionSucceededIn", { action: label, seconds: (result.durationMs / 1000).toFixed(1) }) : backendMessage("kernelActionSucceeded", { action: label });
  return result.warning ? `${message} ${result.warning}` : message;
}
