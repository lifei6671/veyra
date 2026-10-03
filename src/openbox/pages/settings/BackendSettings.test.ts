import { afterEach, describe, expect, it, vi } from "vitest";
import { api } from "../../api/client";
import { backupParts, downloadBackendJson, kernelActionMessage, normalizeTestUrl, parseBackup, retentionForecast, retentionMonths, runKernelAction, storageSize, validBypassPorts } from "./BackendSettings.helpers";

afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers(); vi.unstubAllGlobals(); });
describe("backend settings contracts", () => {
  it("uses the original version, update, backup, import and diagnostic endpoints", async () => {
    const fetch = vi.fn().mockImplementation(async () => new Response('{"state":{"stage":"running","at":1}}', { headers: { "content-type": "text/plain" } }));
    vi.stubGlobal("fetch", fetch); vi.stubGlobal("localStorage", { getItem: () => "zh-CN" }); vi.stubGlobal("navigator", { language: "zh-CN" });
    const backup = parseBackup('{"format":"open-box-backup","profile":{},"nodes":[]}');
    await api.kernelVersion(); await api.updateStatus(); await api.checkUpdate(); await api.runUpdate("mirror"); await api.cancelUpdate(); await api.trafficUsage();
    await api.backup({ subscriptions: true, chainProxies: false, clientRoutes: true, servers: false }); await api.importBackup(backup, "append"); await api.diagnostics(); await api.factoryReset(); await api.deployState();
    expect(fetch.mock.calls.map(call => call[0])).toEqual(["/api/openbox/kernel/version", "/api/openbox/update/status", "/api/openbox/update/check", "/api/openbox/update/run", "/api/openbox/update/cancel", "/api/openbox/traffic/usage", "/api/openbox/backup?subscriptions=1&chainProxies=0&clientRoutes=1&servers=0", "/api/openbox/backup/import?subscriptions=append", "/api/openbox/diagnostics", "/api/openbox/factory-reset", "/api/openbox/deploy/state"]);
    expect(fetch.mock.calls[3][1]).toMatchObject({ method: "POST", body: '{"channel":"mirror"}' });
    expect(fetch.mock.calls[7][1]).toMatchObject({ method: "POST", body: JSON.stringify(backup) });
    expect(fetch.mock.calls[9][1]).toMatchObject({ method: "POST" });
  });
  it("saves only the changed TUN or update fields and renders the returned profile", async () => {
    const profile = { tun: { stack: "gvisor", mtu: 1280, tcpMss: 0, autoRedirect: true }, directBypass: true };
    const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify({ profile })));
    vi.stubGlobal("fetch", fetch); vi.stubGlobal("localStorage", { getItem: () => "zh-CN" }); vi.stubGlobal("navigator", { language: "zh-CN" });
    await expect(api.saveProfile({ tun: { stack: "gvisor" } })).resolves.toEqual(profile);
    expect(fetch.mock.calls[0]).toMatchObject(["/api/openbox/profile", { method: "PUT", body: '{"tun":{"stack":"gvisor"}}' }]);
  });
  it("rejects unrelated or malformed imports and counts nodes from the backup's top-level collection", () => {
    for (const text of ['{}', '[]', '{"format":"other","profile":{}}', '{"format":"open-box-backup","profile":null}', '{"format":"open-box-backup","profile":[]}', 'broken']) expect(() => parseBackup(text)).toThrow();
    const backup = parseBackup('{"format":"open-box-backup","profile":{"servers":[{}]},"subscriptions":[{"id":"sub"}],"nodes":[{},{}],"panelSettings":{},"backgroundImage":"fixture"}');
    expect(backupParts(backup)).toContain("1 条订阅、2 个节点"); expect(backupParts(backup)).toContain("1 台共享网络"); expect(backupParts(backup)).toContain("面板设置和背景图");
  });
  it("exports the returned data as a named JSON file and releases the temporary download URL", async () => {
    vi.useFakeTimers(); vi.setSystemTime(new Date(2026, 9, 3, 9, 5));
    const anchor = { href: "", download: "", click: vi.fn(), remove: vi.fn() };
    const appendChild = vi.fn(), createObjectURL = vi.fn().mockReturnValue("blob:backend-export"), revokeObjectURL = vi.fn();
    vi.stubGlobal("document", { createElement: () => anchor, body: { appendChild } });
    vi.stubGlobal("window", { setTimeout }); vi.stubGlobal("URL", { createObjectURL, revokeObjectURL });
    const data = { format: "open-box-backup", profile: {}, nodes: [{ name: "fixture" }] };
    expect(downloadBackendJson(data, "backup")).toBe("open-box-backup-20261003-0905.json");
    expect(anchor).toMatchObject({ href: "blob:backend-export", download: "open-box-backup-20261003-0905.json" });
    expect(appendChild).toHaveBeenCalledWith(anchor); expect(anchor.click).toHaveBeenCalledOnce(); expect(anchor.remove).toHaveBeenCalledOnce();
    const blob = createObjectURL.mock.calls[0][0] as Blob;
    expect(blob.type).toBe("application/json"); expect(await blob.text()).toBe(JSON.stringify(data, null, 2));
    await vi.advanceTimersByTimeAsync(1000); expect(revokeObjectURL).toHaveBeenCalledWith("blob:backend-export");
  });
  it("uses source port syntax and retention limits, and preserves explicitly entered test addresses", () => {
    expect(validBypassPorts("21114-21119, 2233")).toBe(true); expect(validBypassPorts("")).toBe(true); expect(validBypassPorts("53,abc")).toBe(false);
    expect([retentionMonths(""), retentionMonths("100"), retentionMonths("-3"), retentionMonths("4.9")]).toEqual([3, 36, 1, 4]);
    expect(normalizeTestUrl("  http://example.com/204  ")).toBe("http://example.com/204"); expect(normalizeTestUrl("", true)).toBe("http://connectivitycheck.platform.hicloud.com/generate_204");
    expect(retentionForecast({ days: 4, bytes: 40e6, perDay: 2e6, oldPerDay: 1e6, collapseAfterDays: 30, hourPerDay: 1e6, hourKeepDays: 7 }, 3)).toBe(127e6);
    expect(storageSize(0)).toBe("0 MB"); expect(storageSize(12e6)).toBe("12 MB");
  });
  it("reports the backend's action failure instead of treating HTTP success as kernel success", () => {
    expect(kernelActionMessage("restart", { ok: false, code: 2, stderr: "invalid config" })).toContain("重启失败:invalid config");
    expect(kernelActionMessage("stop", { ok: false, code: 7 })).toContain("退出码 7");
    expect(kernelActionMessage("start", { ok: true, code: 0, durationMs: 1200, warning: "warning" })).toContain("启动成功,耗时 1.2 秒。 warning");
  });
  it("recovers a disconnected restart only from a deployment started by this action", async () => {
    vi.useFakeTimers(); vi.stubGlobal("window", { setTimeout });
    const started = Date.now(); vi.spyOn(api, "serviceAction").mockRejectedValue(new Error("disconnected"));
    const state = vi.spyOn(api, "deployState").mockResolvedValueOnce({ at: started - 1000, stage: "running" }).mockResolvedValueOnce({ at: started, stage: "running", warning: "recovered" });
    const action = runKernelAction("restart"); await vi.advanceTimersByTimeAsync(3000);
    await expect(action).resolves.toMatchObject({ ok: true, warning: "recovered", durationMs: 3000 }); expect(state).toHaveBeenCalledTimes(2);
  });
  it("does not poll deployment state when stopping failed", async () => {
    vi.spyOn(api, "serviceAction").mockRejectedValue(new Error("unreachable")); const state = vi.spyOn(api, "deployState");
    await expect(runKernelAction("stop")).rejects.toThrow("unreachable"); expect(state).not.toHaveBeenCalled();
  });
});
