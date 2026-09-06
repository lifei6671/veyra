import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const commands = {
  list: "list_subscriptions", import: "import_subscription", update: "update_subscription",
  settings: "get_subscription_settings", edit: "edit_subscription", activate: "activate_subscription",
  share: "get_subscription_share_url", delete: "delete_subscription",
} as const;
const STATE_CHANGED_EVENT = "subscription-state-changed";

export const MAX_SUBSCRIPTION_NAME_LENGTH = 80;
export const MAX_SUBSCRIPTION_DESCRIPTION_LENGTH = 280;
export const MAX_USER_AGENT_LENGTH = 256;
export const MAX_REMOTE_URL_BYTES = 8192;
export const MAX_MANUAL_CONTENT_BYTES = 4 * 1024 * 1024;

export type SubscriptionSourceKind = "remote" | "manual";
export type ProxyMode = "direct" | "system" | "managedCore";
export type SubscriptionTrafficSummary = { upload: number | null; download: number | null; total: number | null; expireAtMs: number | null };
export type SubscriptionSummary = {
  id: string; name: string; description: string; sourceKind: SubscriptionSourceKind; nodeCount: number; skippedNodeCount: number;
  lastSuccessAtMs: number | null; traffic: SubscriptionTrafficSummary | null;
  allowAutoUpdate: boolean; updateIntervalMinutes: number | null;
  active: boolean; activeConfigurationGeneration: number | null; shareable: boolean;
};
export type SubscriptionErrorCode =
  | "invalidInput" | "busy" | "fetchFailed" | "parseFailed" | "normalizationFailed"
  | "validationFailed" | "saveFailed" | "stateUnavailable" | "notFound" | "cacheUnavailable"
  | "identityFailed" | "invalidOptions" | "unsupportedClashProviders" | "systemProxyUnavailable"
  | "proxyUnavailable" | "referenceConflict" | "notShareable";
export type ActivationErrorCode =
  | "invalidInput" | "identityFailed" | "busy" | "notFound" | "stateUnavailable"
  | "selectionConflict" | "configurationFailed" | "saveFailed" | "stopFailed"
  | "startFailed" | "recoveryRequired";
export type SubscriptionResult<T> = ({ status: "ok" } & T) | { status: "error"; error: SubscriptionErrorCode };
export type RemoteImportOptions = {
  userAgent?: string; timeoutSeconds: number; proxyMode: ProxyMode; verifyTls: boolean;
  allowAutoUpdate: boolean; intervalMinutes: number | null;
};
export type ImportSubscriptionRequest = {
  name: string; description: string;
  source: { kind: "remote"; url: string; options: RemoteImportOptions } | { kind: "manual"; content: string };
};
export type SubscriptionSettings = {
  id: string; name: string; description: string; sourceKind: SubscriptionSourceKind;
  urlPreview: string | null; hasCustomUserAgent: boolean;
  remoteRequest: { timeoutSeconds: number; proxyMode: ProxyMode; verifyTls: boolean } | null;
  updatePolicy: { allowAutoUpdate: boolean; intervalMinutes: number | null }; shareable: boolean;
};
export type EditSubscriptionRequest = {
  id: string; name?: string; description?: string; urlReplacement?: string;
  remoteRequest?: { userAgent?: string; timeoutSeconds?: number; proxyMode?: ProxyMode; verifyTls?: boolean };
  updatePolicy?: { allowAutoUpdate?: boolean; intervalMinutes?: number | null };
};
export type UpdateSubscriptionRequest = { id: string; content?: string; routeOverride?: "managedCore" };
export type ActivateSubscriptionResult =
  | { status: "ok"; outcome: "activated" | "alreadyCurrent" | "reactivated"; operationId: string; subscription: SubscriptionSummary; activeConfigurationGeneration: number }
  | { status: "pending"; operationId: string }
  | { status: "error"; operationId: string | null; error: ActivationErrorCode };
export type SubscriptionStateChanged = { id: string; change: "updated" | "edited" | "activated" | "deleted" };

export async function listSubscriptions(): Promise<SubscriptionResult<{ subscriptions: SubscriptionSummary[] }>> {
  return parseListResult(await invoke<unknown>(commands.list));
}
export async function importSubscription(request: ImportSubscriptionRequest): Promise<SubscriptionResult<{ subscription: SubscriptionSummary }>> {
  return parseSummaryResult(await invoke<unknown>(commands.import, { request }), false);
}
export async function updateSubscription(request: UpdateSubscriptionRequest): Promise<SubscriptionResult<{ subscription: SubscriptionSummary; contentChanged: boolean }>> {
  return parseSummaryResult(await invoke<unknown>(commands.update, { request }), true);
}
export async function getSubscriptionSettings(id: string): Promise<SubscriptionResult<{ settings: SubscriptionSettings }>> {
  const value = await invoke<unknown>(commands.settings, { request: { id } });
  if (isErrorResult(value)) return value;
  if (!isRecord(value) || !hasExactKeys(value, ["status", "settings"]) || value.status !== "ok") throw invalidResponse();
  return { status: "ok", settings: parseSettings(value.settings) };
}
export async function editSubscription(request: EditSubscriptionRequest): Promise<SubscriptionResult<{ subscription: SubscriptionSummary; contentChanged: boolean }>> {
  return parseSummaryResult(await invoke<unknown>(commands.edit, { request }), true);
}
export async function activateSubscription(id: string, force = false): Promise<ActivateSubscriptionResult> {
  return parseActivateResult(await invoke<unknown>(commands.activate, { request: { id, ...(force ? { force: true } : {}) } }));
}
export async function getSubscriptionShareUrl(id: string): Promise<SubscriptionResult<{ url: string }>> {
  const value = await invoke<unknown>(commands.share, { request: { id } });
  if (isErrorResult(value)) return value;
  if (!isRecord(value) || !hasExactKeys(value, ["status", "url"]) || value.status !== "ok" ||
    typeof value.url !== "string" || !value.url || utf8Length(value.url) > MAX_REMOTE_URL_BYTES) throw invalidResponse();
  return { status: "ok", url: value.url };
}
export async function deleteSubscription(id: string): Promise<SubscriptionResult<{ deletedId: string }>> {
  const value = await invoke<unknown>(commands.delete, { request: { id } });
  if (isErrorResult(value)) return value;
  if (!isRecord(value) || !hasExactKeys(value, ["status", "deletedId"]) || value.status !== "ok" || typeof value.deletedId !== "string" || !value.deletedId) throw invalidResponse();
  return { status: "ok", deletedId: value.deletedId };
}
export async function subscribeSubscriptionStateChanged(onChange: (event: SubscriptionStateChanged) => void): Promise<UnlistenFn> {
  return listen<unknown>(STATE_CHANGED_EVENT, ({ payload }) => onChange(parseStateChanged(payload)));
}

export function subscriptionErrorMessage(error: SubscriptionErrorCode | ActivationErrorCode | "operationTimedOut"): string {
  return error === "operationTimedOut" ? "应用操作超时，请检查当前运行状态后重试" : errorMessages[error];
}
export function normalizedSubscriptionName(name: string, sourceKind: SubscriptionSourceKind): string {
  return name.trim() || (sourceKind === "remote" ? "远程订阅" : "本地订阅");
}
export function safeFileDisplayName(fileName: string): string {
  const normalized = fileName.replace(/[\u0000-\u001f\u007f]/g, "").trim();
  return normalized ? Array.from(normalized).slice(0, MAX_SUBSCRIPTION_NAME_LENGTH).join("") : "本地订阅";
}
export const unicodeScalarLength = (value: string) => Array.from(value).length;
export const utf8Length = (value: string) => new TextEncoder().encode(value).byteLength;
export function formatRelativeTime(timestamp: number, now = Date.now()): string {
  const elapsed = Math.max(0, now - timestamp);
  if (elapsed < 60_000) return "刚刚";
  if (elapsed < 3_600_000) return `${Math.floor(elapsed / 60_000)} 分钟前`;
  if (elapsed < 86_400_000) return `${Math.floor(elapsed / 3_600_000)} 小时前`;
  return `${Math.floor(elapsed / 86_400_000)} 天前`;
}

function parseListResult(value: unknown): SubscriptionResult<{ subscriptions: SubscriptionSummary[] }> {
  if (isErrorResult(value)) return value;
  if (!isRecord(value) || !hasExactKeys(value, ["status", "subscriptions"]) || value.status !== "ok" || !Array.isArray(value.subscriptions)) throw invalidResponse();
  return { status: "ok", subscriptions: value.subscriptions.map(parseSummary) };
}
function parseSummaryResult(value: unknown, withChanged: false): SubscriptionResult<{ subscription: SubscriptionSummary }>;
function parseSummaryResult(value: unknown, withChanged: true): SubscriptionResult<{ subscription: SubscriptionSummary; contentChanged: boolean }>;
function parseSummaryResult(value: unknown, withChanged: boolean): SubscriptionResult<{ subscription: SubscriptionSummary; contentChanged?: boolean }> {
  if (isErrorResult(value)) return value;
  const keys = withChanged ? ["status", "subscription", "contentChanged"] : ["status", "subscription"];
  if (!isRecord(value) || !hasExactKeys(value, keys) || value.status !== "ok" || (withChanged && typeof value.contentChanged !== "boolean")) throw invalidResponse();
  return { status: "ok", subscription: parseSummary(value.subscription), ...(withChanged ? { contentChanged: value.contentChanged as boolean } : {}) };
}
function parseSummary(value: unknown): SubscriptionSummary {
  const keys = ["id", "name", "description", "sourceKind", "nodeCount", "skippedNodeCount", "lastSuccessAtMs", "traffic", "allowAutoUpdate", "updateIntervalMinutes", "active", "activeConfigurationGeneration", "shareable"];
  if (!isRecord(value) || !hasExactKeys(value, keys) || typeof value.id !== "string" || !value.id ||
    typeof value.name !== "string" || !isSafeText(value.name, 80, false) ||
    typeof value.description !== "string" || !isSafeText(value.description, 280, true) ||
    !isSourceKind(value.sourceKind) || !isNonNegativeInteger(value.nodeCount) || !isNonNegativeInteger(value.skippedNodeCount) || value.skippedNodeCount > 0xffff_ffff ||
    !isNullableNonNegativeInteger(value.lastSuccessAtMs) || typeof value.allowAutoUpdate !== "boolean" ||
    !isNullableInterval(value.updateIntervalMinutes) || typeof value.active !== "boolean" ||
    !isNullableNonNegativeInteger(value.activeConfigurationGeneration) ||
    value.active !== (value.activeConfigurationGeneration !== null) || typeof value.shareable !== "boolean" ||
    (!value.allowAutoUpdate && value.updateIntervalMinutes !== null) ||
    (value.sourceKind === "manual" && (value.allowAutoUpdate || value.updateIntervalMinutes !== null || value.shareable))) throw invalidResponse();
  return { ...value, traffic: value.traffic === null ? null : parseTraffic(value.traffic) } as SubscriptionSummary;
}
function parseSettings(value: unknown): SubscriptionSettings {
  if (!isRecord(value) || !hasExactKeys(value, ["id", "name", "description", "sourceKind", "urlPreview", "hasCustomUserAgent", "remoteRequest", "updatePolicy", "shareable"]) ||
    typeof value.id !== "string" || !value.id || typeof value.name !== "string" || !isSafeText(value.name, 80, false) ||
    typeof value.description !== "string" || !isSafeText(value.description, 280, true) ||
    !isSourceKind(value.sourceKind) || !(value.urlPreview === null || (typeof value.urlPreview === "string" && isSafeUrlPreview(value.urlPreview))) ||
    typeof value.hasCustomUserAgent !== "boolean" || typeof value.shareable !== "boolean" ||
    !isRecord(value.updatePolicy) || !hasExactKeys(value.updatePolicy, ["allowAutoUpdate", "intervalMinutes"]) ||
    typeof value.updatePolicy.allowAutoUpdate !== "boolean" || !isNullableInterval(value.updatePolicy.intervalMinutes)) throw invalidResponse();
  let remoteRequest: SubscriptionSettings["remoteRequest"] = null;
  if (value.remoteRequest !== null) {
    if (!isRecord(value.remoteRequest) || !hasExactKeys(value.remoteRequest, ["timeoutSeconds", "proxyMode", "verifyTls"]) ||
      !isTimeout(value.remoteRequest.timeoutSeconds) || !isProxyMode(value.remoteRequest.proxyMode) || typeof value.remoteRequest.verifyTls !== "boolean") throw invalidResponse();
    remoteRequest = value.remoteRequest as NonNullable<SubscriptionSettings["remoteRequest"]>;
  }
  if ((value.sourceKind === "remote") !== (remoteRequest !== null) ||
    (!value.updatePolicy.allowAutoUpdate && value.updatePolicy.intervalMinutes !== null) ||
    (value.sourceKind === "manual" && (value.urlPreview !== null || value.hasCustomUserAgent || value.updatePolicy.allowAutoUpdate || value.updatePolicy.intervalMinutes !== null || value.shareable))) throw invalidResponse();
  return { ...value, remoteRequest } as SubscriptionSettings;
}
function parseActivateResult(value: unknown): ActivateSubscriptionResult {
  if (!isRecord(value) || typeof value.status !== "string") throw invalidResponse();
  if (value.status === "pending" && hasExactKeys(value, ["status", "operationId"]) && typeof value.operationId === "string" && value.operationId) return value as ActivateSubscriptionResult;
  if (value.status === "error" && hasExactKeys(value, ["status", "operationId", "error"]) &&
    isActivationError(value.error) &&
    (value.error === "identityFailed" ? value.operationId === null : typeof value.operationId === "string" && value.operationId.length > 0)) return value as ActivateSubscriptionResult;
  if (value.status === "ok" && hasExactKeys(value, ["status", "outcome", "operationId", "subscription", "activeConfigurationGeneration"]) &&
    (value.outcome === "activated" || value.outcome === "alreadyCurrent" || value.outcome === "reactivated") && typeof value.operationId === "string" && value.operationId &&
    isNonNegativeInteger(value.activeConfigurationGeneration)) {
    const subscription = parseSummary(value.subscription);
    if (!subscription.active || subscription.activeConfigurationGeneration !== value.activeConfigurationGeneration) throw invalidResponse();
    return { ...value, subscription } as ActivateSubscriptionResult;
  }
  throw invalidResponse();
}
function parseStateChanged(value: unknown): SubscriptionStateChanged {
  if (!isRecord(value) || !hasExactKeys(value, ["id", "change"]) || typeof value.id !== "string" || !value.id ||
    (value.change !== "updated" && value.change !== "edited" && value.change !== "activated" && value.change !== "deleted")) throw invalidResponse();
  return value as SubscriptionStateChanged;
}
function parseTraffic(value: unknown): SubscriptionTrafficSummary {
  if (!isRecord(value) || !hasExactKeys(value, ["upload", "download", "total", "expireAtMs"]) ||
    !isNullableNonNegativeInteger(value.upload) || !isNullableNonNegativeInteger(value.download) ||
    !isNullableNonNegativeInteger(value.total) || !isNullableNonNegativeInteger(value.expireAtMs)) throw invalidResponse();
  return value as SubscriptionTrafficSummary;
}
function isErrorResult(value: unknown): value is { status: "error"; error: SubscriptionErrorCode } {
  return isRecord(value) && hasExactKeys(value, ["status", "error"]) && value.status === "error" &&
    typeof value.error === "string" && subscriptionErrors.includes(value.error as SubscriptionErrorCode);
}
const subscriptionErrors: SubscriptionErrorCode[] = ["invalidInput", "busy", "fetchFailed", "parseFailed", "normalizationFailed", "validationFailed", "saveFailed", "stateUnavailable", "notFound", "cacheUnavailable", "identityFailed", "invalidOptions", "unsupportedClashProviders", "systemProxyUnavailable", "proxyUnavailable", "referenceConflict", "notShareable"];
const activationErrors: ActivationErrorCode[] = ["invalidInput", "identityFailed", "busy", "notFound", "stateUnavailable", "selectionConflict", "configurationFailed", "saveFailed", "stopFailed", "startFailed", "recoveryRequired"];
const isActivationError = (value: unknown): value is ActivationErrorCode => typeof value === "string" && activationErrors.includes(value as ActivationErrorCode);
const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);
const hasExactKeys = (value: Record<string, unknown>, expected: readonly string[]) => { const keys = Object.keys(value); return keys.length === expected.length && expected.every((key) => key in value); };
const isSourceKind = (value: unknown): value is SubscriptionSourceKind => value === "remote" || value === "manual";
const isProxyMode = (value: unknown): value is ProxyMode => value === "direct" || value === "system" || value === "managedCore";
const isNonNegativeInteger = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const isPositiveInteger = (value: unknown): value is number => isNonNegativeInteger(value) && value > 0;
const isNullableNonNegativeInteger = (value: unknown): value is number | null => value === null || isNonNegativeInteger(value);
const isTimeout = (value: unknown): value is number => isPositiveInteger(value) && value >= 5 && value <= 120;
const isNullableInterval = (value: unknown): value is number | null => value === null || (isPositiveInteger(value) && value >= 1440);
const isSafeText = (value: string, maximum: number, allowEmpty: boolean) => (allowEmpty || value.trim().length > 0) && unicodeScalarLength(value) <= maximum && !/[\u0000-\u001f\u007f]/.test(value);
function isSafeUrlPreview(value: string): boolean {
  try {
    const url = new URL(value);
    return !url.username && !url.password && !url.search && !url.hash;
  } catch { return false; }
}
const invalidResponse = () => new Error("invalid subscription response");

const errorMessages: Record<SubscriptionErrorCode | ActivationErrorCode, string> = {
  invalidInput: "订阅输入无效，请检查后重试", busy: "订阅操作进行中，请稍后重试",
  fetchFailed: "获取订阅失败，请稍后重试", parseFailed: "订阅内容无法解析，或没有可导入的节点",
  normalizationFailed: "订阅内容无法转换", validationFailed: "订阅内容未通过校验",
  saveFailed: "订阅保存失败，原有内容未更改", stateUnavailable: "订阅状态不可用",
  notFound: "订阅不存在，请刷新列表", cacheUnavailable: "订阅缓存不可用，请稍后重试",
  identityFailed: "无法创建订阅身份，请重试", systemProxyUnavailable: "系统代理不可用，请检查系统设置",
  invalidOptions: "订阅请求设置无效，请检查后重试", unsupportedClashProviders: "此订阅仅包含暂不支持的外部 Provider",
  proxyUnavailable: "当前内核未提供代理端口", referenceConflict: "订阅正在使用或仍被其它设置引用，无法删除",
  notShareable: "此订阅不能分享二维码", recoveryRequired: "运行状态需要恢复后再试",
  selectionConflict: "当前分流设置与该订阅不兼容", configurationFailed: "订阅配置检查失败，当前服务未切换",
  stopFailed: "旧服务停止失败，切换未完成", startFailed: "新服务启动失败，当前服务已停止",
};
export type DocumentFormat = "json" | "yaml";
export type SubscriptionDocument = { id: string; format: DocumentFormat; content: string; revision: string; sourceKind: SubscriptionSourceKind; localOverride: boolean };
export type DocumentErrorCode = SubscriptionErrorCode | ActivationErrorCode | "documentUnavailable" | "documentConflict" | "contentTooLarge" | "formatFailed" | "unsupportedNodes" | "notRunning" | "operationTimedOut";
export type DocumentFailure = { status: "error"; error: DocumentErrorCode; operationId?: string; location?: { line: number; column: number } };
export type DocumentApply = { status: "notRequired" } | { status: "ready"; operationId: string; generation: number } | { status: "failed"; operationId: string; generation: number; error: "stopFailed" | "startFailed" | "recoveryRequired" | "operationTimedOut" };
export type DocumentSaveResult = DocumentFailure | { status: "ok"; subscription: SubscriptionSummary; documentRevision: string; apply: DocumentApply };
export type RunningConfiguration = { status: "ok"; format: "json"; content: string; appliedSubscriptionId: string; appliedConfigurationGeneration: number };

const documentMessages = {
  documentUnavailable: "当前订阅尚未保存 JSON/YAML 原文件，请刷新远程订阅或重新导入本地文件后编辑",
  documentConflict: "订阅文件已被更新，请关闭后重新打开编辑，避免覆盖新内容",
  contentTooLarge: "文件不能超过 4 MiB",
  formatFailed: "文件格式错误，请检查 JSON/YAML 语法",
  unsupportedNodes: "文件包含无法转换为 sing-box 配置的节点，请修改后重试",
  notRunning: "内核未运行，无法查看运行时配置",
  operationTimedOut: "操作超时，请检查当前运行状态后重试",
};
export function documentErrorMessage(result: DocumentFailure): string {
  const message = Object.hasOwn(documentMessages, result.error) ? documentMessages[result.error as keyof typeof documentMessages] : subscriptionErrorMessage(result.error as SubscriptionErrorCode | ActivationErrorCode);
  return result.location ? `${message}（第 ${result.location.line} 行，第 ${result.location.column} 列）` : message;
}
function parseDocumentFailure(value: unknown): DocumentFailure | null {
  if (!isRecord(value) || value.status !== "error") return null;
  if (typeof value.error !== "string" || !(Object.hasOwn(documentMessages, value.error) || Object.hasOwn(errorMessages, value.error)) ||
    Object.keys(value).some(key => !["status", "error", "operationId", "location"].includes(key)) ||
    (value.operationId !== undefined && (typeof value.operationId !== "string" || !value.operationId)) ||
    (value.location !== undefined && (!isRecord(value.location) || !hasExactKeys(value.location, ["line", "column"]) || !isPositiveInteger(value.location.line) || !isPositiveInteger(value.location.column)))) throw invalidResponse();
  return value as DocumentFailure;
}
const isDocumentContent = (value: unknown): value is string => typeof value === "string" && utf8Length(value) > 0 && utf8Length(value) <= MAX_MANUAL_CONTENT_BYTES;
const isDocumentRevision = (value: unknown): value is string => typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
export async function getSubscriptionDocument(id: string): Promise<DocumentFailure | { status: "ok"; document: SubscriptionDocument }> {
  const value = await invoke<unknown>("get_subscription_document", { request: { id } });
  const failure = parseDocumentFailure(value); if (failure) return failure;
  if (!isRecord(value) || !hasExactKeys(value, ["status", "document"]) || value.status !== "ok" || !isRecord(value.document)) throw invalidResponse();
  const doc = value.document;
  if (!hasExactKeys(doc, ["id", "format", "content", "revision", "sourceKind", "localOverride"]) || doc.id !== id ||
    (doc.format !== "json" && doc.format !== "yaml") || !isDocumentContent(doc.content) || !isDocumentRevision(doc.revision) || !isSourceKind(doc.sourceKind) || typeof doc.localOverride !== "boolean" || (doc.sourceKind === "manual" && doc.localOverride)) throw invalidResponse();
  return { status: "ok", document: doc as SubscriptionDocument };
}
export async function formatSubscriptionDocument(content: string, format: DocumentFormat): Promise<DocumentFailure | { status: "ok"; content: string }> {
  const value = await invoke<unknown>("format_subscription_document", { request: { content, format } });
  const failure = parseDocumentFailure(value); if (failure) return failure;
  if (!isRecord(value) || !hasExactKeys(value, ["status", "content"]) || value.status !== "ok" || !isDocumentContent(value.content)) throw invalidResponse();
  return { status: "ok", content: value.content };
}
export async function getRunningConfiguration(): Promise<DocumentFailure | RunningConfiguration> {
  const value = await invoke<unknown>("get_running_configuration");
  const failure = parseDocumentFailure(value); if (failure) return failure;
  if (!isRecord(value) || !hasExactKeys(value, ["status", "format", "content", "appliedSubscriptionId", "appliedConfigurationGeneration"]) || value.status !== "ok" || value.format !== "json" || !isDocumentContent(value.content) || typeof value.appliedSubscriptionId !== "string" || !value.appliedSubscriptionId || !isNonNegativeInteger(value.appliedConfigurationGeneration)) throw invalidResponse();
  return value as RunningConfiguration;
}
export async function saveSubscriptionDocument(request: { id: string; content: string; format: DocumentFormat; expectedRevision: string }): Promise<DocumentSaveResult> {
  const value = await invoke<unknown>("save_subscription_document", { request });
  const failure = parseDocumentFailure(value); if (failure) return failure;
  if (!isRecord(value)) throw invalidResponse();
  if (value.status !== "ok" || !hasExactKeys(value, ["status", "subscription", "documentRevision", "apply"]) || !isDocumentRevision(value.documentRevision) || !isRecord(value.apply)) throw invalidResponse();
  const apply = value.apply;
  if (apply.status === "notRequired") { if (!hasExactKeys(apply, ["status"])) throw invalidResponse(); }
  else {
    if (!["ready", "failed"].includes(String(apply.status)) || !hasExactKeys(apply, apply.status === "failed" ? ["status", "operationId", "generation", "error"] : ["status", "operationId", "generation"]) || typeof apply.operationId !== "string" || !apply.operationId || !isNonNegativeInteger(apply.generation) || (apply.status === "failed" && !["stopFailed", "startFailed", "recoveryRequired", "operationTimedOut"].includes(String(apply.error)))) throw invalidResponse();
  }
  return { ...value, subscription: parseSummary(value.subscription), apply: apply as DocumentApply } as DocumentSaveResult;
}
