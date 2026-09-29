import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { api } from "./api/client";
import type { AuthStatus, RouteKey, ServiceStatus, StorageResponse } from "./api/types";
import { AppShell, type ShellStats } from "./components/AppShell";
import { AuthScreen } from "./components/AuthScreen";
import { ErrorState, LoadingView } from "./components/shared";
import { useLiveMetrics } from "./hooks/useLiveMetrics";
import { OverviewPage } from "./pages/OverviewPage";
import { PROXY_VIEW_SETTINGS_KEY, ProxiesPage, proxyViewSettings } from "./pages/ProxiesPage";
import { ConnectionsPage } from "./pages/ConnectionsPage";
import { LogsPage } from "./pages/LogsPage";
import { RulesPage } from "./pages/RulesPage";
import { SettingsPage } from "./pages/SettingsPage";
import { Toaster } from "./ui/sonner";

const routeKeys: RouteKey[] = ["overview", "proxies", "connections", "logs", "rules", "settings"];

function routeFromHash(): RouteKey {
  const candidate = window.location.hash.replace(/^#\/?/, "").split("/")[0] as RouteKey;
  return routeKeys.includes(candidate) ? candidate : "overview";
}

export function OpenBoxApp() {
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [route, setRoute] = useState<RouteKey>(routeFromHash);
  const [storage, setStorage] = useState<StorageResponse | null>(null);
  const [backgroundImage, setBackgroundImage] = useState("");
  const [service, setService] = useState<ServiceStatus | null>(null);
  const [theme, setTheme] = useState<"light" | "dark">("light");
  const [collapsed, setCollapsed] = useState(false);
  const [proxyNodeCardMinWidth, setProxyNodeCardMinWidth] = useState(() => proxyViewSettings(window.localStorage.getItem(PROXY_VIEW_SETTINGS_KEY)).nodeCardMinWidth);
  const [busy, setBusy] = useState(false);
  const [authError, setAuthError] = useState<unknown>(null);

  const authenticated = Boolean(auth && (!auth.enabled || auth.authenticated));
  const live = useLiveMetrics(authenticated);

  const showToast = useCallback((message: string) => {
    toast(message);
  }, []);

  const loadStorage = useCallback(async () => {
    const value = await api.storage();
    setStorage(value);
    const language = value.entries["config/language"] ?? "zh-CN";
    localStorage.setItem("config/language", language);
    document.documentElement.lang = language;
    const background = value.entries["config/custom-background-image"] ?? "";
    if (background.startsWith("local-image-")) {
      setBackgroundImage(await api.backgroundImage());
    } else {
      setBackgroundImage("");
    }
    const themeMode = value.entries["config/theme-mode"];
    setTheme(themeMode === "dark" || (themeMode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches) ? "dark" : "light");
    setCollapsed(value.entries["config/is-sidebar-collapsed"] === "true");
  }, []);

  const patchStorage = useCallback(async (entries: Record<string, string>, removed: string[] = []) => {
    await api.patchStorage(entries, removed);
    setStorage(current => {
      if (!current) return current;
      const nextEntries = { ...current.entries, ...entries };
      removed.forEach(key => delete nextEntries[key]);
      return { entries: nextEntries };
    });
  }, []);

  const loadService = useCallback(async () => setService(await api.serviceStatus()), []);

  const loadAuth = useCallback(async () => {
    setAuthError(null);
    try {
      setAuth(await api.authStatus());
    } catch (reason) {
      setAuthError(reason);
    }
  }, []);

  useEffect(() => {
    void loadAuth();
    const unauthorized = () => setAuth(current => ({ enabled: true, authenticated: false, passwordSet: current?.passwordSet ?? true }));
    window.addEventListener("openbox:unauthorized", unauthorized);
    return () => window.removeEventListener("openbox:unauthorized", unauthorized);
  }, [loadAuth]);

  useEffect(() => {
    const hashChange = () => setRoute(routeFromHash());
    window.addEventListener("hashchange", hashChange);
    if (!window.location.hash) window.location.hash = "#/overview";
    return () => window.removeEventListener("hashchange", hashChange);
  }, []);

  useEffect(() => {
    if (!authenticated) return;
    void Promise.all([loadStorage(), loadService()]).catch(reason => showToast(reason instanceof Error ? reason.message : "后端初始化失败"));
    const timer = window.setInterval(() => void loadService().catch(() => undefined), 15_000);
    return () => window.clearInterval(timer);
  }, [authenticated, loadService, loadStorage, showToast]);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.lang = "zh-CN";
    document.body.dataset.theme = theme;
    document.body.classList.toggle("theme-dark", theme === "dark");
  }, [theme]);

  const navigate = (next: RouteKey) => { window.location.hash = `#/${next}`; };
  const toggleCollapse = () => {
    const next = !collapsed;
    setCollapsed(next);
    void api.patchStorage({ "config/is-sidebar-collapsed": String(next) }).catch(reason => showToast(reason instanceof Error ? reason.message : "侧栏设置保存失败"));
  };
  const refresh = async () => {
    setBusy(true);
    try { await Promise.all([loadStorage(), loadService()]); showToast("数据已刷新"); }
    catch (reason) { showToast(reason instanceof Error ? reason.message : "刷新失败"); }
    finally { setBusy(false); }
  };
  const serviceAction = async (action: "start" | "stop") => {
    if (action === "stop" && !window.confirm("确定停止 Open-Box 内核吗？现有连接会中断。")) return;
    setBusy(true);
    try { await api.serviceAction(action); await loadService(); showToast(action === "start" ? "内核启动命令已执行" : "内核已停止"); }
    catch (reason) { showToast(reason instanceof Error ? reason.message : "内核操作失败"); }
    finally { setBusy(false); }
  };

  const stats = useMemo<ShellStats>(() => ({
    connections: live.connections.connections.length,
    memory: live.memory,
    inbound: live.traffic.down,
    inboundRate: live.traffic.downRate,
    outbound: live.traffic.up,
    outboundRate: live.traffic.upRate,
  }), [live]);

  if (authError) return <main className="auth-page"><ErrorState title="无法连接 Open-Box 后端" error={authError} onRetry={() => void loadAuth()} /></main>;
  if (!auth) return <LoadingView label="正在检查后端登录状态" />;
  if (!authenticated) return <AuthScreen status={auth} onAuthenticated={setAuth} />;
  if (!storage) return <LoadingView />;

  return <>
    <AppShell route={route} onNavigate={navigate} service={service} stats={stats} collapsed={collapsed} onCollapse={toggleCollapse} onServiceAction={serviceAction} onRefresh={() => void refresh()} busy={busy} appearance={{ radius: Number(storage.entries["config/global-radius"] ?? 15.8), background: (storage.entries["config/custom-background-image"] ?? "").startsWith("local-image-") ? backgroundImage : storage.entries["config/custom-background-image"] ?? "", backgroundOpacity: Number(storage.entries["config/dashboard-transparent"] ?? 90), backgroundBlur: Number(storage.entries["config/blur-intensity"] ?? 10), nodeCardMinWidth: proxyNodeCardMinWidth }}>
      {route === "overview" && <OverviewPage stats={stats} storage={storage} onToast={showToast} />}
      {route === "proxies" && <ProxiesPage storage={storage} onToast={showToast} onNodeCardMinWidthChange={setProxyNodeCardMinWidth} />}
      {route === "connections" && <ConnectionsPage frame={live.connections} storage={storage} onPatchStorage={patchStorage} onToast={showToast} />}
      {route === "logs" && <LogsPage />}
      {route === "rules" && <RulesPage storage={storage} onToast={showToast} />}
      {route === "settings" && <SettingsPage storage={storage} theme={theme} setTheme={setTheme} onToast={message => { showToast(message); void loadStorage().catch(() => undefined); }} />}
    </AppShell>
    <Toaster theme={theme} />
  </>;
}
