import { lazy, Suspense, useEffect, useRef, useState, type ChangeEvent, type FormEvent, type KeyboardEvent, type MouseEvent } from "react";
import { QRCodeSVG } from "qrcode.react";
import { formatTraffic } from "../../lib/traffic-trend";
import type { RuntimeObservation } from "../../lib/observability";
import {
  MAX_MANUAL_CONTENT_BYTES, MAX_REMOTE_URL_BYTES, MAX_SUBSCRIPTION_DESCRIPTION_LENGTH,
  MAX_SUBSCRIPTION_NAME_LENGTH, MAX_USER_AGENT_LENGTH, activateSubscription, deleteSubscription,
  editSubscription, formatRelativeTime, getSubscriptionSettings, getSubscriptionShareUrl,
  importSubscription, listSubscriptions, normalizedSubscriptionName, safeFileDisplayName,
  getSubscriptionDocument, formatSubscriptionDocument, saveSubscriptionDocument, getRunningConfiguration, documentErrorMessage, type SubscriptionDocument,
  subscribeSubscriptionStateChanged, subscriptionErrorMessage, unicodeScalarLength,
  updateSubscription, utf8Length, type ProxyMode, type SubscriptionSettings,
  type SubscriptionSourceKind, type SubscriptionSummary,
} from "../../lib/subscriptions";

const DocumentEditor = lazy(() => import("./DocumentEditor"));

type Props = { active: boolean; observation: RuntimeObservation | null };
type DialogMode = "create" | "edit" | "replace" | "qr" | "delete" | "batchDelete";
type Notice = { message: string; kind: "error" | "success" };
type MenuState = { subscription: SubscriptionSummary; x: number; y: number };
type UserAgentMode = "clash" | "singBox" | "clashMeta" | "custom" | "preserve";
type FormState = {
  sourceKind: SubscriptionSourceKind; name: string; description: string; remoteUrl: string;
  manualContent: string; manualInputMode: "text" | "file"; userAgentMode: UserAgentMode; userAgent: string; timeoutSeconds: string;
  proxyMode: ProxyMode; verifyTls: boolean; allowAutoUpdate: boolean; intervalMinutes: string;
};

const defaultForm = (): FormState => ({
  sourceKind: "remote", name: "", description: "", remoteUrl: "", manualContent: "", manualInputMode: "text",
  userAgentMode: "clash", userAgent: "", timeoutSeconds: "30", proxyMode: "direct",
  verifyTls: true, allowAutoUpdate: true, intervalMinutes: "",
});
const pendingSwitchStatuses = new Set(["queued", "checking", "prepared", "persisted", "applying"]);
const MAX_QR_SHARE_BYTES = 2048;

export function SubscriptionPage({ active, observation }: Props) {
  const [loadState, setLoadState] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [subscriptions, setSubscriptions] = useState<SubscriptionSummary[]>([]);
  const [documentSession, setDocumentSession] = useState<{ kind: "subscription"; document: SubscriptionDocument } | { kind: "runtime"; content: string; id: string; generation: number; operationId: string | null } | null>(null);
  const documentRevisionRef = useRef("");
  const documentEpochRef = useRef(0);
  const observationRef = useRef(observation);
  useEffect(() => { observationRef.current = observation; }, [observation]);
  useEffect(() => {
    if (!active) { documentEpochRef.current += 1; setDocumentSession(null); documentRevisionRef.current = ""; }
  }, [active]);
  useEffect(() => {
    if (documentSession?.kind !== "runtime") return;
    if (observation?.sidecarLifecycle !== "ready" || observation.appliedSubscriptionId !== documentSession.id || observation.appliedConfigurationGeneration !== documentSession.generation || (observation.subscriptionSwitch?.operationId ?? null) !== documentSession.operationId) setDocumentSession(null);
  }, [observation, documentSession]);
  const [selectionMode, setSelectionMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [quickUrl, setQuickUrl] = useState("");
  const [pendingOperation, setPendingOperation] = useState<string | null>(null);
  const [pendingActivation, setPendingActivation] = useState<{ id: string; operationId: string | null } | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [dialogMode, setDialogMode] = useState<DialogMode | null>(null);
  const [dialogTarget, setDialogTarget] = useState<SubscriptionSummary | null>(null);
  const [settings, setSettings] = useState<SubscriptionSettings | null>(null);
  const [settingsLoading, setSettingsLoading] = useState(false);
  const [form, setForm] = useState<FormState>(defaultForm);
  const [selectedFileName, setSelectedFileName] = useState<string | null>(null);
  const [tlsConfirmation, setTlsConfirmation] = useState(false);
  const [shareUrl, setShareUrl] = useState<string | null>(null);
  const [shareUnavailable, setShareUnavailable] = useState(false);
  const [contextMenu, setContextMenu] = useState<MenuState | null>(null);
  const hasSubscriptions = loadState === "ready" && subscriptions.length > 0;
  const fileInputRef = useRef<HTMLInputElement>(null);
  const originalUrlRef = useRef("");
  const fileReadEpochRef = useRef(0);
  const [fileReading, setFileReading] = useState(false);
  const dialogRef = useRef<HTMLElement>(null);
  const dialogReturnFocusRef = useRef<HTMLElement | null>(null);
  const dialogWasOpenRef = useRef(false);
  const dialogWasPendingRef = useRef(false);
  const menuItemRef = useRef<HTMLButtonElement>(null);
  const menuReturnFocusRef = useRef<HTMLElement | null>(null);
  const menuWasOpenRef = useRef(false);
  const dialogEpochRef = useRef(0);
  const dialogIdentityRef = useRef<{ epoch: number; mode: DialogMode; targetId: string | null } | null>(null);

  useEffect(() => { if (active && loadState === "idle") void load(); }, [active, loadState]);
  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;
    void subscribeSubscriptionStateChanged(() => {
      if (!mounted) return;
      void refreshList();
    }).then((stop) => { unlisten = stop; }).catch(() => undefined);
    return () => { mounted = false; unlisten?.(); };
  }, []);
  useEffect(() => {
    const current = observation?.subscriptionSwitch;
    if (pendingActivation === null || current?.operationId !== pendingActivation.operationId) return;
    if (pendingSwitchStatuses.has(current.status)) return;
    setPendingActivation(null);
    setPendingOperation(null);
    void refreshList();
    if (current.status === "ready" && observation?.sidecarLifecycle === "ready" &&
      observation.appliedSubscriptionId === pendingActivation.id &&
      observation.appliedConfigurationGeneration !== null) {
      setNotice({ message: "订阅已应用，服务正在使用最新配置", kind: "success" });
    } else if (current.status === "ready") {
      setNotice({ message: "订阅切换结果与运行状态不一致，请刷新后重试", kind: "error" });
    }
    else if (current.status === "cancelled") setNotice({ message: "订阅切换已取消", kind: "error" });
    else if (current.status === "failed" && current.errorCode) setNotice({ message: current.errorCode === "cancelled" ? "订阅切换已取消" : subscriptionErrorMessage(current.errorCode), kind: "error" });
  }, [observation, pendingActivation]);
  useEffect(() => {
    if (notice === null) return;
    const timer = window.setTimeout(dismissNotice, 6000);
    return () => window.clearTimeout(timer);
  }, [dialogMode, notice]);
  useEffect(() => {
    if (dialogMode !== null) {
      dialogWasOpenRef.current = true;
      window.queueMicrotask(focusFirstDialogControl);
      return;
    }
    if (dialogWasOpenRef.current) {
      dialogWasOpenRef.current = false;
      dialogReturnFocusRef.current?.focus();
    }
  }, [dialogMode, settingsLoading]);
  useEffect(() => {
    if (dialogMode === null) { dialogWasPendingRef.current = false; return; }
    if (pendingOperation !== null) { dialogWasPendingRef.current = true; dialogRef.current?.focus(); return; }
    if (dialogWasPendingRef.current) { dialogWasPendingRef.current = false; focusFirstDialogControl(); }
  }, [dialogMode, pendingOperation]);
  useEffect(() => {
    if (contextMenu !== null) { menuWasOpenRef.current = true; menuItemRef.current?.focus(); return; }
    if (menuWasOpenRef.current) { menuWasOpenRef.current = false; menuReturnFocusRef.current?.focus(); }
  }, [contextMenu]);
  useEffect(() => {
    if (contextMenu === null) return;
    const close = () => setContextMenu(null);
    const escape = (event: globalThis.KeyboardEvent) => { if (event.key === "Escape") close(); };
    window.addEventListener("click", close); window.addEventListener("keydown", escape);
    return () => { window.removeEventListener("click", close); window.removeEventListener("keydown", escape); };
  }, [contextMenu]);

  async function load() {
    setLoadState("loading");
    try {
      const result = await listSubscriptions();
      if (result.status === "error") { setLoadState("error"); fail(result.error); return; }
      setSubscriptions(result.subscriptions); setLoadState("ready");
    } catch { setLoadState("error"); failResponse(); }
  }
  async function refreshList() {
    try {
      const result = await listSubscriptions();
      if (result.status === "ok") { setSubscriptions(result.subscriptions); setLoadState("ready"); }
    } catch { /* The visible state remains authoritative until a successful event readback. */ }
  }
  function upsert(next: SubscriptionSummary) {
    setSubscriptions((current) => current.some((item) => item.id === next.id)
      ? current.map((item) => item.id === next.id ? next : item) : [...current, next]);
  }
  function fail(error: Parameters<typeof subscriptionErrorMessage>[0]) { setNotice({ message: subscriptionErrorMessage(error), kind: "error" }); }
  function failResponse() { setNotice({ message: "订阅响应不可用，请重试", kind: "error" }); }
  function dismissNotice() { setNotice(null); }
  function focusFirstDialogControl() {
    const next = dialogRef.current?.querySelector<HTMLElement>("select:not(:disabled), textarea:not(:disabled), input:not(:disabled), button:not(:disabled)");
    (next ?? dialogRef.current)?.focus();
  }
  function resetDialog() {
    dialogEpochRef.current += 1;
    dialogIdentityRef.current = null;
    setDialogMode(null); setDialogTarget(null); setSettings(null); setSettingsLoading(false);
    originalUrlRef.current = ""; fileReadEpochRef.current += 1; setFileReading(false);
    setForm(defaultForm()); setSelectedFileName(null); setTlsConfirmation(false); setShareUrl(null); setShareUnavailable(false);
    setPendingOperation((current) => current?.startsWith("share-") ? null : current);
    if (fileInputRef.current) fileInputRef.current.value = "";
  }
  function beginDialog(mode: DialogMode, target: SubscriptionSummary | null, returnFocus?: HTMLElement | null): number {
    resetDialog();
    const epoch = dialogEpochRef.current;
    dialogIdentityRef.current = { epoch, mode, targetId: target?.id ?? null };
    dialogReturnFocusRef.current = returnFocus ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    setDialogTarget(target); setDialogMode(mode);
    return epoch;
  }
  function isCurrentDialog(epoch: number, mode: DialogMode, targetId: string | null) {
    const current = dialogIdentityRef.current;
    return current?.epoch === epoch && current.mode === mode && current.targetId === targetId;
  }
  function openCreateDialog() { beginDialog("create", null); }
  async function openSettingsDialog(target: SubscriptionSummary) {
    handoffMenuFocus();
    const epoch = beginDialog("edit", target, menuReturnFocusRef.current);
    setSettingsLoading(true);
    try {
      const result = await getSubscriptionSettings(target.id);
      if (!isCurrentDialog(epoch, "edit", target.id)) return;
      if (result.status === "error") { fail(result.error); resetDialog(); return; }
      const next = result.settings;
      let remoteUrl = "";
      if (next.sourceKind === "remote") {
        const source = await getSubscriptionShareUrl(target.id);
        if (!isCurrentDialog(epoch, "edit", target.id)) return;
        if (source.status === "error") { fail(source.error); resetDialog(); return; }
        remoteUrl = source.url;
      }
      originalUrlRef.current = remoteUrl;
      setSettings(next);
      setForm({
        ...defaultForm(), sourceKind: next.sourceKind, name: next.name, description: next.description, remoteUrl,
        userAgentMode: next.hasCustomUserAgent ? "preserve" : "clash",
        timeoutSeconds: String(next.remoteRequest?.timeoutSeconds ?? 30),
        proxyMode: next.remoteRequest?.proxyMode ?? "direct", verifyTls: next.remoteRequest?.verifyTls ?? true,
        allowAutoUpdate: next.updatePolicy.allowAutoUpdate,
        intervalMinutes: next.updatePolicy.intervalMinutes === null ? "" : String(next.updatePolicy.intervalMinutes),
      });
    } catch {
      if (isCurrentDialog(epoch, "edit", target.id)) { failResponse(); resetDialog(); }
    } finally {
      if (isCurrentDialog(epoch, "edit", target.id)) setSettingsLoading(false);
    }
  }
  function openReplaceDialog(target: SubscriptionSummary) {
    handoffMenuFocus(); beginDialog("replace", target, menuReturnFocusRef.current);
    setForm({ ...defaultForm(), sourceKind: target.sourceKind, name: target.name, description: target.description });
  }
  async function openQrDialog(target: SubscriptionSummary) {
    handoffMenuFocus(); const epoch = beginDialog("qr", target, menuReturnFocusRef.current); setPendingOperation(`share-${target.id}`);
    try {
      const result = await getSubscriptionShareUrl(target.id);
      if (!isCurrentDialog(epoch, "qr", target.id)) return;
      if (result.status === "error") { fail(result.error); resetDialog(); return; }
      if (utf8Length(result.url) > MAX_QR_SHARE_BYTES) {
        setShareUrl(null); setShareUnavailable(true);
        setNotice({ message: "订阅链接过长，无法生成二维码", kind: "error" });
        return;
      }
      setShareUrl(result.url);
    } catch {
      if (isCurrentDialog(epoch, "qr", target.id)) { failResponse(); resetDialog(); }
    } finally {
      if (isCurrentDialog(epoch, "qr", target.id)) setPendingOperation(null);
    }
  }
  function openDeleteDialog(target: SubscriptionSummary) {
    handoffMenuFocus(); beginDialog("delete", target, menuReturnFocusRef.current);
  }
  function handoffMenuFocus() { if (contextMenu !== null) menuWasOpenRef.current = false; setContextMenu(null); }

  async function submitQuickImport() {
    if (pendingOperation !== null) return;
    const url = quickUrl.trim();
    if (!url || utf8Length(url) > MAX_REMOTE_URL_BYTES) { setNotice({ message: "请输入有效的订阅文件链接", kind: "error" }); return; }
    setPendingOperation("quick-import"); setNotice(null);
    try {
      const result = await importSubscription({ name: "远程订阅", description: "", source: { kind: "remote", url, options: defaultRemoteOptions() } });
      if (result.status === "error") { fail(result.error); return; }
      upsert(result.subscription); setQuickUrl(""); setLoadState("ready"); success("订阅已导入，运行配置未切换", result.subscription);
    } catch { failResponse(); } finally { setPendingOperation(null); }
  }
  function handleQuickKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key !== "Enter" || event.nativeEvent.isComposing || event.keyCode === 229) return;
    event.preventDefault(); void submitQuickImport();
  }
  async function pasteQuickUrl() {
    try {
      if (!navigator.clipboard?.readText) throw new Error("clipboard unavailable");
      setQuickUrl((await navigator.clipboard.readText()).trim());
    } catch { setNotice({ message: "无法读取剪贴板，请手动粘贴", kind: "error" }); }
  }
  function changeManualInputMode(mode: "text" | "file") {
    fileReadEpochRef.current += 1; setFileReading(false);
    patchForm({ manualInputMode: mode, manualContent: "" }); setSelectedFileName(null);
    if (fileInputRef.current) fileInputRef.current.value = "";
  }
  async function selectFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]; if (!file) return;
    // 文件对象已取得，清空控件让失败后重选同一文件也能触发 change。
    event.target.value = "";
    const epoch = ++fileReadEpochRef.current;
    patchForm({ manualContent: "" }); setSelectedFileName(null);
    if (file.size > MAX_MANUAL_CONTENT_BYTES) { failInput("订阅文件过大，请选择不超过 4 MiB 的文件"); return; }
    setFileReading(true);
    try {
      const content = await file.text();
      if (fileReadEpochRef.current !== epoch) return;
      if (utf8Length(content) > MAX_MANUAL_CONTENT_BYTES) { failInput("订阅文件过大，请选择不超过 4 MiB 的文件"); return; }
      setForm((current) => ({ ...current, manualContent: content, name: current.name.trim() || (dialogMode === "create" ? safeFileDisplayName(file.name) : current.name) }));
      setSelectedFileName(safeFileDisplayName(file.name));
    } catch { if (fileReadEpochRef.current === epoch) failInput("无法读取订阅文件，请重新选择"); }
    finally { if (fileReadEpochRef.current === epoch) setFileReading(false); }
  }
  function validatedCommon(): { name: string; description: string } | null {
    const name = normalizedSubscriptionName(form.name, form.sourceKind);
    const description = form.description.trim();
    if (unicodeScalarLength(name) > MAX_SUBSCRIPTION_NAME_LENGTH) { failInput("订阅名称不能超过 80 个字符"); return null; }
    if (unicodeScalarLength(description) > MAX_SUBSCRIPTION_DESCRIPTION_LENGTH || /[\u0000-\u001f\u007f]/.test(description)) { failInput("订阅描述不能超过 280 个字符且不能包含控制字符"); return null; }
    return { name, description };
  }
  function validatedRemote() {
    const url = form.remoteUrl.trim();
    const timeout = Number(form.timeoutSeconds);
    const interval = form.intervalMinutes.trim() === "" ? null : Number(form.intervalMinutes);
    if (!url || utf8Length(url) > MAX_REMOTE_URL_BYTES) { failInput("请输入有效的订阅文件链接"); return null; }
    if (!Number.isInteger(timeout) || timeout < 5 || timeout > 120) { failInput("总超时必须为 5 到 120 秒"); return null; }
    if (interval !== null && (!Number.isSafeInteger(interval) || interval < 1440)) { failInput("自动更新周期至少为 1440 分钟"); return null; }
    if (form.userAgentMode === "custom" && (!form.userAgent || unicodeScalarLength(form.userAgent) > MAX_USER_AGENT_LENGTH || !/^[\x20-\x7e]+$/.test(form.userAgent))) { failInput("User-Agent 必须是 1 到 256 个可见 ASCII 字符"); return null; }
    if (!form.verifyTls && !tlsConfirmation) { failInput("关闭 TLS 校验前请再次确认"); return null; }
    return { url, timeout, interval };
  }
  async function submitDialog(event: FormEvent) {
    event.preventDefault();
    if (pendingOperation !== null || fileReading || settingsLoading || dialogMode === null || dialogMode === "qr" || dialogMode === "delete") return;
    const common = validatedCommon(); if (!common) return;
    const remote = form.sourceKind === "remote" ? validatedRemote() : null; if (form.sourceKind === "remote" && !remote) return;
    const userAgent = selectedUserAgent(form.userAgentMode, form.userAgent);
    if ((dialogMode === "create" || dialogMode === "replace") && form.sourceKind === "manual" && (!form.manualContent || utf8Length(form.manualContent) > MAX_MANUAL_CONTENT_BYTES)) { failInput("请输入订阅文本或选择不超过 4 MiB 的文件"); return; }
    setPendingOperation(`${dialogMode}-${dialogTarget?.id ?? "new"}`); setNotice(null);
    try {
      let result;
      if (dialogMode === "create") {
        result = await importSubscription({ ...common, source: form.sourceKind === "remote"
          ? { kind: "remote", url: remote!.url, options: { ...defaultRemoteOptions(), timeoutSeconds: remote!.timeout, proxyMode: form.proxyMode, verifyTls: form.verifyTls, allowAutoUpdate: form.allowAutoUpdate, intervalMinutes: remote!.interval, ...(userAgent ? { userAgent } : {}) } }
          : { kind: "manual", content: form.manualContent } });
      } else if (dialogMode === "edit" && dialogTarget !== null) {
        result = await editSubscription({ id: dialogTarget.id, ...common, ...(form.sourceKind === "remote" ? {
          ...(remote!.url !== originalUrlRef.current ? { urlReplacement: remote!.url } : {}),
          remoteRequest: { timeoutSeconds: remote!.timeout, proxyMode: form.proxyMode, verifyTls: form.verifyTls, ...(form.userAgentMode === "preserve" ? {} : { userAgent: userAgent ?? "" }) },
          updatePolicy: { allowAutoUpdate: form.allowAutoUpdate, intervalMinutes: remote!.interval },
        } : {}) });
      } else if (dialogMode === "replace" && dialogTarget !== null) {
        result = form.sourceKind === "remote"
          ? await editSubscription({ id: dialogTarget.id, urlReplacement: remote!.url })
          : await updateSubscription({ id: dialogTarget.id, content: form.manualContent });
      } else return;
      if (result.status === "error") { fail(result.error); return; }
      upsert(result.subscription); const linkChanged = form.sourceKind === "remote" && remote?.url !== originalUrlRef.current; resetDialog(); success(dialogMode === "edit" ? (linkChanged ? "订阅信息已保存并刷新" : "订阅信息已保存") : dialogMode === "replace" ? "订阅源已替换，运行配置未切换" : "订阅已导入，运行配置未切换", result.subscription);
    } catch { failResponse(); } finally { setPendingOperation(null); }
  }
  function closeDocument() { documentEpochRef.current += 1; setDocumentSession(null); documentRevisionRef.current = ""; }
  async function openDocument(target: SubscriptionSummary) {
    handoffMenuFocus();
    if (pendingOperation !== null) return;
    const epoch = ++documentEpochRef.current;
    setPendingOperation(`read-document-${target.id}`); setNotice(null);
    try {
      const result = await getSubscriptionDocument(target.id);
      if (documentEpochRef.current !== epoch) return;
      if (result.status === "error") { setNotice({ message: documentErrorMessage(result), kind: "error" }); return; }
      documentRevisionRef.current = result.document.revision;
      setDocumentSession({ kind: "subscription", document: result.document });
    } catch { if (documentEpochRef.current === epoch) failResponse(); }
    finally { setPendingOperation(null); }
  }
  async function openRunningDocument() {
    if (pendingOperation !== null || observation?.sidecarLifecycle !== "ready") return;
    const epoch = ++documentEpochRef.current;
    setPendingOperation("read-running"); setNotice(null);
    try {
      const result = await getRunningConfiguration();
      if (epoch !== documentEpochRef.current) return;
      if (result.status === "error") { setNotice({ message: documentErrorMessage(result), kind: "error" }); return; }
      const current = observationRef.current;
      if (current?.sidecarLifecycle !== "ready" || result.appliedSubscriptionId !== current.appliedSubscriptionId || result.appliedConfigurationGeneration !== current.appliedConfigurationGeneration) { setNotice({ message: "运行配置已变化，请重新打开", kind: "error" }); return; }
      setDocumentSession({ kind: "runtime", content: result.content, id: result.appliedSubscriptionId, generation: result.appliedConfigurationGeneration, operationId: current.subscriptionSwitch?.operationId ?? null });
    } catch { if (epoch === documentEpochRef.current) failResponse(); }
    finally { setPendingOperation(null); }
  }
  async function saveDocument(content: string) {
    if (documentSession?.kind !== "subscription") return;
    const result = await saveSubscriptionDocument({ id: documentSession.document.id, content, format: documentSession.document.format, expectedRevision: documentRevisionRef.current });
    if (result.status === "error") throw new Error(documentErrorMessage(result));
    documentRevisionRef.current = result.documentRevision;
    upsert(result.subscription);
    if (result.apply.status === "failed") throw new Error(`文件已保存，但应用失败：${documentErrorMessage({ status: "error", error: result.apply.error })}`);
    success(result.apply.status === "notRequired" ? "文件已保存" : "文件已保存并启用");
  }
  function toggleSelected(id: string) {
    setSelectedIds(current => { const next = new Set(current); if (next.has(id)) next.delete(id); else next.add(id); return next; });
  }
  async function updateAll() {
    if (pendingOperation !== null) return;
    const targets = subscriptions.filter(item => item.sourceKind === "remote");
    setPendingOperation("update-all"); setNotice(null);
    let updated = 0;
    const failures: string[] = [];
    try {
      for (const target of targets) {
        try {
          const result = await updateSubscription({ id: target.id });
          if (result.status === "error") failures.push(`${target.name}：${subscriptionErrorMessage(result.error)}`);
          else { upsert(result.subscription); updated += 1; }
        } catch { failures.push(`${target.name}：更新响应异常`); }
      }
      setNotice({ message: `已检查 ${updated} 个远程订阅${failures.length ? `，${failures.length} 个更新失败：${failures.join("；")}` : ""}`, kind: failures.length ? "error" : "success" });
    } finally { setPendingOperation(null); }
  }
  async function deleteSelected() {
    if (pendingOperation !== null) return;
    setPendingOperation("delete-selected"); setNotice(null);
    const removed = new Set<string>();
    try {
      for (const id of selectedIds) {
        try { const result = await deleteSubscription(id); if (result.status === "ok") removed.add(id); }
        catch { /* 其余项继续删除；失败项保留选择，最终明确报告。 */ }
      }
      setSubscriptions(current => current.filter(item => !removed.has(item.id)));
      const remaining = new Set([...selectedIds].filter(id => !removed.has(id)));
      setSelectedIds(remaining); setSelectionMode(remaining.size > 0); resetDialog();
      setNotice({ message: `已删除 ${removed.size} 个订阅${remaining.size ? `；${remaining.size} 个未删除，请确认是否正在使用或仍被引用` : ""}`, kind: remaining.size ? "error" : "success" });
    } finally { setPendingOperation(null); }
  }
  async function updateOne(subscription: SubscriptionSummary, managed = false) {
    handoffMenuFocus();
    if (subscription.sourceKind === "manual") { openReplaceDialog(subscription); return; }
    if (managed && !observation?.managedProxyAvailable) { fail("proxyUnavailable"); return; }
    if (pendingOperation !== null) return;
    setPendingOperation(`update-${subscription.id}`); setNotice(null);
    try {
      const result = await updateSubscription({ id: subscription.id, ...(managed ? { routeOverride: "managedCore" as const } : {}) });
      if (result.status === "error") { fail(result.error); return; }
      upsert(result.subscription); success(result.contentChanged ? "订阅已更新，运行配置未切换" : "检查成功，内容未变化，运行配置未切换", result.subscription);
    } catch { failResponse(); } finally { setPendingOperation(null); }
  }
  async function activate(target: SubscriptionSummary, force = false) {
    handoffMenuFocus(); if (pendingOperation !== null) return;
    setPendingOperation(`activate-${target.id}`); setPendingActivation({ id: target.id, operationId: null }); setNotice(null);
    try {
      const result = await activateSubscription(target.id, force);
      if (result.status === "error") { setPendingActivation(null); setPendingOperation(null); fail(result.error); return; }
      if (result.status === "pending") { setPendingActivation({ id: target.id, operationId: result.operationId }); return; }
      upsert(result.subscription); setPendingActivation(null); setPendingOperation(null);
      success(result.outcome === "alreadyCurrent" ? "订阅已是当前运行配置" : "订阅已应用，服务正在使用最新配置");
      void refreshList();
    } catch { setPendingActivation(null); setPendingOperation(null); failResponse(); }
  }
  async function confirmDelete() {
    if (!dialogTarget || pendingOperation !== null) return;
    setPendingOperation(`delete-${dialogTarget.id}`); setNotice(null);
    try {
      const result = await deleteSubscription(dialogTarget.id);
      if (result.status === "error") { fail(result.error); return; }
      setSubscriptions((current) => current.filter((item) => item.id !== result.deletedId));
      resetDialog(); success("订阅已删除");
    } catch { failResponse(); } finally { setPendingOperation(null); }
  }
  function success(message: string, subscription?: SubscriptionSummary) {
    const count = subscription?.skippedNodeCount ?? 0;
    setNotice({ message: count > 0 ? `${message}；已过滤 ${count} 个不支持的节点` : message, kind: "success" });
  }
  function failInput(message: string) { setNotice({ message, kind: "error" }); }
  function patchForm(patch: Partial<FormState>) { setForm((current) => ({ ...current, ...patch })); }
  function openContextMenu(event: MouseEvent, subscription: SubscriptionSummary) {
    event.preventDefault(); event.stopPropagation();
    menuReturnFocusRef.current = event.currentTarget.querySelector<HTMLElement>(".refresh-button") ?? event.currentTarget as HTMLElement;
    setContextMenu({ subscription, x: event.clientX, y: event.clientY });
  }
  function openKeyboardMenu(event: KeyboardEvent<HTMLElement>, subscription: SubscriptionSummary) {
    if (event.key !== "ContextMenu" && !(event.shiftKey && event.key === "F10")) return;
    event.preventDefault(); const bounds = event.currentTarget.getBoundingClientRect();
    menuReturnFocusRef.current = event.currentTarget as HTMLElement;
    setContextMenu({ subscription, x: bounds.left, y: bounds.bottom });
  }
  function handleDialogKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (event.key === "Escape" && (pendingOperation === null || dialogMode === "qr")) { event.preventDefault(); resetDialog(); return; }
    if (event.key !== "Tab") return;
    const controls = [...(dialogRef.current?.querySelectorAll<HTMLElement>("button:not(:disabled), select:not(:disabled), input:not(:disabled), textarea:not(:disabled)") ?? [])];
    if (!controls.length) { event.preventDefault(); dialogRef.current?.focus(); return; }
    const first = controls[0], last = controls[controls.length - 1];
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
  }

  const noticeElement = notice ? <div className={`subscription-notice subscription-notice-${notice.kind}`} role={notice.kind === "error" ? "alert" : "status"}>{notice.message}</div> : null;
  return <main className="home-content subscription-page" id="page-subscriptions" hidden={!active} aria-labelledby="subscriptions-title">
    <header className="home-header"><h1 id="subscriptions-title">订阅</h1><div className="subscription-header-actions">
      <button type="button" title="选择订阅" aria-label="选择订阅" aria-pressed={selectionMode} disabled={pendingOperation !== null} onClick={() => { setSelectionMode(value => !value); setSelectedIds(new Set()); }}><svg viewBox="0 0 24 24"><rect x="4" y="4" width="16" height="16" rx="1" /></svg></button>
      <button type="button" title="刷新全部订阅" aria-label="刷新全部订阅" disabled={pendingOperation !== null || !subscriptions.some(item => item.sourceKind === "remote")} onClick={() => void updateAll()}><RefreshIcon spinning={pendingOperation === "update-all"} /></button>
      <button type="button" title="查看运行时配置" aria-label="查看运行时配置" disabled={pendingOperation !== null || observation?.sidecarLifecycle !== "ready"} onClick={() => void openRunningDocument()}><svg viewBox="0 0 24 24"><path d="M5 3h10l4 4v14H5zM14 3v5h5M8 12h8M8 16h8" /></svg></button>
      <button type="button" title="重新激活订阅" aria-label="重新激活订阅" disabled={pendingOperation !== null || !subscriptions.some(item => item.active)} onClick={() => { const current = subscriptions.find(item => item.active); if (current) void activate(current, true); }}><svg viewBox="0 0 24 24"><path d="M13 2c2 6 7 7 7 13a8 8 0 0 1-16 0c0-4 3-7 5-10 0 4 1 5 2 6 2-3 3-5 2-9Z" /></svg></button>
    </div></header>
    <div className="page-scroll subscription-scroll">
      <section className="subscription-toolbar" aria-label="快捷导入订阅">
        <div className="subscription-url-field"><input type="url" value={quickUrl} placeholder="订阅文件链接" aria-label="订阅文件链接" disabled={pendingOperation !== null} onChange={(e) => setQuickUrl(e.target.value)} onKeyDown={handleQuickKeyDown} />
          {quickUrl ? <button type="button" className="field-action" aria-label="清除订阅文件链接" onClick={() => setQuickUrl("")}>×</button>
            : <button type="button" className="field-action" aria-label="从剪贴板粘贴订阅文件链接" onClick={() => void pasteQuickUrl()}><ClipboardIcon /></button>}</div>
        <button type="button" className="primary-button" disabled={pendingOperation !== null} onClick={() => void submitQuickImport()}>{pendingOperation === "quick-import" ? "导入中" : "导入"}</button>
        <button type="button" className="secondary-button" disabled={pendingOperation !== null} onClick={openCreateDialog}>新建</button>
      </section>
      {selectionMode && <section className="subscription-batch-actions" aria-label="批量操作"><label><input type="checkbox" checked={subscriptions.length > 0 && subscriptions.every(item => selectedIds.has(item.id))} disabled={pendingOperation !== null} onChange={event => setSelectedIds(event.target.checked ? new Set(subscriptions.map(item => item.id)) : new Set())} />全选</label><span>已选择 {selectedIds.size} 项</span><button type="button" className="danger-button" disabled={pendingOperation !== null || selectedIds.size === 0} onClick={() => beginDialog("batchDelete", null)}>删除所选</button></section>}
      {(loadState === "loading" || loadState === "idle") && <section className="subscription-state" role="status">正在加载订阅</section>}
      {loadState === "error" && <section className="subscription-state"><p>订阅列表暂时不可用</p><button type="button" className="secondary-button" onClick={() => void load()}>重试</button></section>}
      {loadState === "ready" && subscriptions.length === 0 && <section className="subscription-state"><p>暂无订阅，可通过上方链接导入或新建订阅。</p><button type="button" className="secondary-button" onClick={openCreateDialog}>新建订阅</button></section>}
      {hasSubscriptions && <section className="subscription-grid" aria-label="订阅列表">{subscriptions.map((subscription) =>
        <SubscriptionCard key={subscription.id} subscription={subscription}
          updating={pendingOperation === `update-${subscription.id}`} selectionMode={selectionMode} checked={selectedIds.has(subscription.id)}
          disabled={pendingOperation !== null} onUse={() => selectionMode ? toggleSelected(subscription.id) : void activate(subscription)} onUpdate={() => void updateOne(subscription)}
          onContextMenu={(event) => openContextMenu(event, subscription)} onMenuKeyDown={(event) => openKeyboardMenu(event, subscription)} />)}</section>}
    </div>
    {dialogMode && <div className="dialog-backdrop" role="presentation" onClick={(event) => { if (event.target === event.currentTarget && (pendingOperation === null || dialogMode === "qr")) resetDialog(); }}><section ref={dialogRef} className={`subscription-dialog${dialogMode === "qr" ? "" : " subscription-dialog-expanded"}`} role="dialog" aria-modal="true" aria-labelledby="subscription-dialog-title" aria-busy={pendingOperation !== null || settingsLoading} tabIndex={-1} onKeyDown={handleDialogKeyDown}>
      {renderDialog()}
      {noticeElement}
    </section></div>}
    {contextMenu && <SubscriptionMenu menu={contextMenu} disabled={pendingOperation !== null} managedAvailable={observation?.managedProxyAvailable === true} firstRef={menuItemRef}
      onUse={() => void activate(contextMenu.subscription)} onShare={() => void openQrDialog(contextMenu.subscription)}
      onEditFile={() => void openDocument(contextMenu.subscription)} onEdit={() => void openSettingsDialog(contextMenu.subscription)} onReplace={() => openReplaceDialog(contextMenu.subscription)}
      onUpdate={() => void updateOne(contextMenu.subscription)} onManagedUpdate={() => void updateOne(contextMenu.subscription, true)}
      onDelete={() => openDeleteDialog(contextMenu.subscription)} />}
    {!dialogMode && noticeElement}
    {documentSession && <Suspense fallback={<div className="dialog-backdrop" role="status">正在打开编辑器</div>}><DocumentEditor title={documentSession.kind === "runtime" ? "运行时配置" : "编辑文件"} content={documentSession.kind === "runtime" ? documentSession.content : documentSession.document.content} format={documentSession.kind === "runtime" ? "json" : documentSession.document.format} readOnly={documentSession.kind === "runtime"} onClose={closeDocument} onSave={saveDocument} onFormat={async content => { const result = await formatSubscriptionDocument(content, documentSession.kind === "runtime" ? "json" : documentSession.document.format); if (result.status === "error") throw new Error(documentErrorMessage(result)); return result.content; }} /></Suspense>}
  </main>;

  function renderDialog() {
    const title = dialogMode === "create" ? "新建订阅" : dialogMode === "edit" ? "编辑信息" : dialogMode === "replace" ? "替换订阅源" : dialogMode === "qr" ? "分享二维码" : "删除订阅";
    if (dialogMode === "qr") return <><header><h2 id="subscription-dialog-title">{title}</h2></header><div className="qr-panel">{shareUrl ? <QRCodeSVG value={shareUrl} size={220} level="M" aria-label="订阅分享二维码" /> : shareUnavailable ? <p>二维码不可用</p> : <p role="status">正在生成二维码</p>}<p>点击空白处或按 Esc 关闭，关闭后将清除此二维码</p></div></>;
    if (dialogMode === "batchDelete") return <><header><h2 id="subscription-dialog-title">删除所选订阅</h2></header><p>确定删除所选的 {selectedIds.size} 个订阅吗？正在使用或仍被引用的订阅会保留。</p><footer><button type="button" className="secondary-button" disabled={pendingOperation !== null} onClick={resetDialog}>取消</button><button type="button" className="danger-button" disabled={pendingOperation !== null} onClick={() => void deleteSelected()}>删除</button></footer></>;
    if (dialogMode === "delete") return <><header><h2 id="subscription-dialog-title">{title}</h2></header><p>确定删除“{dialogTarget?.name}”吗？正在使用或仍被引用的订阅会被安全拒绝。</p><footer><button type="button" className="secondary-button" disabled={pendingOperation !== null} onClick={resetDialog}>取消</button><button type="button" className="danger-button" disabled={pendingOperation !== null} onClick={() => void confirmDelete()}>{pendingOperation ? "删除中" : "删除"}</button></footer></>;
    if (settingsLoading) return <><header><h2 id="subscription-dialog-title">{title}</h2></header><p role="status">正在读取订阅设置</p></>;
    const showCommon = dialogMode === "create" || dialogMode === "edit";
    return <form onSubmit={(event) => void submitDialog(event)}>
      <header><h2 id="subscription-dialog-title">{title}</h2></header>
      {dialogMode === "create" && <label className="outlined-field"><span className="field-label">类型</span><select value={form.sourceKind} disabled={pendingOperation !== null} onChange={(e) => { changeManualInputMode("text"); patchForm({ sourceKind: e.target.value as SubscriptionSourceKind, allowAutoUpdate: e.target.value === "remote" }); }}><option value="remote">远程</option><option value="manual">本地</option></select></label>}
      {showCommon && <><label className="outlined-field"><span className="field-label">名称（可选）</span><input value={form.name} disabled={pendingOperation !== null} onChange={(e) => patchForm({ name: e.target.value })} /></label><label className="outlined-field"><span className="field-label">描述（可选）</span><textarea className="description-field" value={form.description} disabled={pendingOperation !== null} onChange={(e) => patchForm({ description: e.target.value })} /></label></>}
      {form.sourceKind === "remote" ? <>
        {<label className="outlined-field"><span className="field-label">订阅文件链接</span><input type="url" value={form.remoteUrl} placeholder={settings?.urlPreview ?? "https://"} disabled={pendingOperation !== null} onChange={(e) => patchForm({ remoteUrl: e.target.value })} /></label>}
        {dialogMode !== "replace" && <div className="subscription-options">
          <label className="outlined-field"><span className="field-label">User-Agent</span><select value={form.userAgentMode} disabled={pendingOperation !== null} onChange={(e) => patchForm({ userAgentMode: e.target.value as UserAgentMode })}>
            {dialogMode === "edit" && settings?.hasCustomUserAgent && <option value="preserve">保持现有</option>}
            <option value="clash">Clash（默认）</option><option value="singBox">sing-box</option><option value="clashMeta">Clash Meta</option><option value="custom">自定义</option>
          </select></label>
          {form.userAgentMode === "custom" && <label className="outlined-field"><span className="field-label">自定义 User-Agent</span><input value={form.userAgent} placeholder="输入 1 到 256 个可见 ASCII 字符" disabled={pendingOperation !== null} onChange={(e) => patchForm({ userAgent: e.target.value })} /></label>}
          {form.userAgentMode === "preserve" && <p className="field-help">保留当前已设置的 User-Agent</p>}
          <div className="option-grid"><label className="outlined-field"><span className="field-label">总超时（秒）</span><input type="number" min={5} max={120} value={form.timeoutSeconds} disabled={pendingOperation !== null} onChange={(e) => patchForm({ timeoutSeconds: e.target.value })} /></label><label className="outlined-field"><span className="field-label">代理模式</span><select value={form.proxyMode} disabled={pendingOperation !== null} onChange={(e) => patchForm({ proxyMode: e.target.value as ProxyMode })}><option value="direct">Direct</option><option value="system">System</option><option value="managedCore" disabled={!observation?.managedProxyAvailable}>ManagedCore</option></select></label></div>
          {!observation?.managedProxyAvailable && <p className="field-help">当前内核未提供代理端口，ManagedCore 暂不可用。</p>}
          <label className="checkbox-row"><input type="checkbox" checked={form.verifyTls} disabled={pendingOperation !== null} onChange={(e) => { patchForm({ verifyTls: e.target.checked }); setTlsConfirmation(false); }} />校验 TLS 证书</label>
          {!form.verifyTls && <label className="tls-warning"><input type="checkbox" checked={tlsConfirmation} disabled={pendingOperation !== null} onChange={(e) => setTlsConfirmation(e.target.checked)} />我确认关闭 TLS 校验会降低连接安全性</label>}
          <label className="checkbox-row"><input type="checkbox" checked={form.allowAutoUpdate} disabled={pendingOperation !== null} onChange={(e) => patchForm({ allowAutoUpdate: e.target.checked, intervalMinutes: e.target.checked ? form.intervalMinutes : "" })} />允许自动更新</label>
          <label className="outlined-field"><span className="field-label">更新周期（分钟，可选）</span><input type="number" min={1440} value={form.intervalMinutes} disabled={pendingOperation !== null || !form.allowAutoUpdate} onChange={(e) => patchForm({ intervalMinutes: e.target.value })} /></label>
        </div>}
      </> : dialogMode !== "edit" ? <>
        <fieldset className="manual-input-modes"><legend>导入方式</legend>
          <label><input type="radio" name="manual-input-mode" value="text" checked={form.manualInputMode === "text"} disabled={pendingOperation !== null} onChange={() => changeManualInputMode("text")} />粘贴文本</label>
          <label><input type="radio" name="manual-input-mode" value="file" checked={form.manualInputMode === "file"} disabled={pendingOperation !== null} onChange={() => changeManualInputMode("file")} />选择文件</label>
        </fieldset>
        {form.manualInputMode === "text" ? <label className="outlined-field"><span className="field-label">订阅文本</span><textarea value={form.manualContent} placeholder="粘贴 Clash、sing-box 或 URI 订阅文本" disabled={pendingOperation !== null} onChange={e => patchForm({ manualContent: e.target.value })} /></label>
          : <div className="local-file-picker"><input ref={fileInputRef} type="file" hidden accept=".json,.yaml,.yml,.txt" disabled={pendingOperation !== null || fileReading} onChange={e => void selectFile(e)} /><button type="button" className="secondary-button" disabled={pendingOperation !== null || fileReading} onClick={() => fileInputRef.current?.click()}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7V4h6l2 3h10v13H3z" /></svg>{fileReading ? "读取中" : "选择本地文件"}</button><span title={selectedFileName ?? undefined}>{selectedFileName ?? "支持 JSON、YAML 或文本配置"}</span></div>}
      </> : null}
      <footer><button type="button" className="secondary-button" disabled={pendingOperation !== null} onClick={resetDialog}>取消</button><button type="submit" className="primary-button" disabled={pendingOperation !== null || fileReading}>{pendingOperation ? "保存中" : "保存"}</button></footer>
    </form>;
  }
}

function defaultRemoteOptions() {
  return { timeoutSeconds: 30, proxyMode: "direct" as const, verifyTls: true, allowAutoUpdate: true, intervalMinutes: null };
}

function selectedUserAgent(mode: UserAgentMode, custom: string): string | undefined {
  if (mode === "singBox") return "sing-box/1.14.0";
  if (mode === "clashMeta") return "Clash.Meta";
  if (mode === "custom") return custom;
  return undefined;
}

function SubscriptionMenu({ menu, disabled, managedAvailable, firstRef, onUse, onShare, onEdit, onEditFile, onReplace, onUpdate, onManagedUpdate, onDelete }: {
  menu: MenuState; disabled: boolean; managedAvailable: boolean; firstRef: React.RefObject<HTMLButtonElement | null>;
  onUse: () => void; onShare: () => void; onEdit: () => void; onEditFile: () => void; onReplace: () => void; onUpdate: () => void; onManagedUpdate: () => void; onDelete: () => void;
}) {
  const remote = menu.subscription.sourceKind === "remote";
  return <div className="subscription-menu" role="menu" style={{ left: `clamp(8px, ${menu.x}px, calc(100vw - 160px))`, top: `clamp(8px, ${menu.y}px, calc(100vh - 300px))` }} onClick={(e) => e.stopPropagation()}>
    <button ref={firstRef} type="button" role="menuitem" disabled={disabled} onClick={onUse}>使用</button>
    {remote && menu.subscription.shareable && <button type="button" role="menuitem" disabled={disabled} onClick={onShare}>分享二维码</button>}
    <button type="button" role="menuitem" disabled={disabled} onClick={onEdit}>编辑信息</button>
    <button type="button" role="menuitem" disabled={disabled} onClick={onEditFile}>编辑文件</button>
    <button type="button" role="menuitem" disabled={disabled} onClick={onReplace}>替换订阅源</button>
    <button type="button" role="menuitem" disabled={disabled} onClick={onUpdate}>更新</button>
    {remote && <button type="button" role="menuitem" disabled={disabled || !managedAvailable} title={managedAvailable ? undefined : "当前内核未提供代理端口"} onClick={onManagedUpdate}>使用 Veyra 代理更新</button>}
    <button type="button" role="menuitem" className="danger-item" disabled={disabled} onClick={onDelete}>删除</button>
  </div>;
}

function SubscriptionCard({ subscription, updating, selectionMode, checked, disabled, onUse, onUpdate, onContextMenu, onMenuKeyDown }: {
  subscription: SubscriptionSummary; updating: boolean; selectionMode: boolean; checked: boolean; disabled: boolean;
  onUse: () => void; onUpdate: () => void; onContextMenu: (event: MouseEvent) => void; onMenuKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
}) {
  const traffic = subscription.traffic;
  const hasTraffic = traffic !== null && traffic.upload !== null && traffic.download !== null && traffic.total !== null;
  const used = hasTraffic ? traffic.upload! + traffic.download! : null;
  const progress = hasTraffic ? (traffic.total === 0 ? (used === 0 ? 0 : 100) : Math.min(100, (used! / traffic.total!) * 100)) : null;
  const exactTime = subscription.lastSuccessAtMs === null ? null : new Date(subscription.lastSuccessAtMs).toLocaleString();
  const dateMs = traffic?.expireAtMs ?? subscription.lastSuccessAtMs;
  const expireTime = dateMs === null ? null : new Date(dateMs).toLocaleDateString();
  return <article className={`subscription-card${subscription.active ? " subscription-card-active" : ""}${selectionMode ? " subscription-card-selection" : ""}`} role="button" tabIndex={disabled ? -1 : 0} aria-label={`${selectionMode ? "选择" : "使用"} ${subscription.name}`} aria-disabled={disabled} onClick={() => { if (!disabled) onUse(); }} onKeyDown={(event) => {
    if (event.target !== event.currentTarget) return;
    onMenuKeyDown(event); if ((event.key === "Enter" || event.key === " ") && !disabled) { event.preventDefault(); onUse(); }
  }} onContextMenu={onContextMenu}>
    {selectionMode && <input className="subscription-card-checkbox" type="checkbox" aria-label={`选中 ${subscription.name}`} checked={checked} disabled={disabled} onClick={event => event.stopPropagation()} onChange={onUse} />}
    <header><div><h2 title={subscription.name}>{subscription.name}</h2></div><button type="button" className="refresh-button" disabled={disabled} aria-label={`更新 ${subscription.name}`} aria-haspopup="menu" onKeyDown={(e) => { e.stopPropagation(); onMenuKeyDown(e); }} onClick={(e) => { e.stopPropagation(); onUpdate(); }}><RefreshIcon spinning={updating} /></button></header>
    <p className="subscription-description" title={subscription.description || undefined}>{subscription.description}</p>
    <dl className="subscription-meta"><div><dt>节点</dt><dd>{subscription.nodeCount}</dd></div><div><dd aria-label={`更新时间 ${subscription.lastSuccessAtMs === null ? "尚未成功更新" : formatRelativeTime(subscription.lastSuccessAtMs)}`} title={exactTime ?? undefined}>{subscription.lastSuccessAtMs === null ? "尚未成功更新" : formatRelativeTime(subscription.lastSuccessAtMs)}</dd></div></dl>
    <div className="subscription-traffic">{hasTraffic ? <><div><span>{formatTraffic(used!)} / {formatTraffic(traffic.total!)}</span>{expireTime && <span>{expireTime}</span>}</div><progress max={100} value={progress!} aria-label={`${subscription.name} 流量使用进度`} /></> : expireTime ? <div className="subscription-expiry-only"><span>{expireTime}</span></div> : null}</div>
  </article>;
}

function ClipboardIcon() { return <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5h6m-5-2h4a1 1 0 0 1 1 1v2H9V4a1 1 0 0 1 1-1ZM7 5H6a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-1" /></svg>; }
function RefreshIcon({ spinning }: { spinning: boolean }) { return <svg className={spinning ? "spin" : undefined} viewBox="0 0 24 24" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5m10.1-3A8 8 0 0 0 5.5 6M4.9 15A8 8 0 0 0 18.5 18" /></svg>; }
