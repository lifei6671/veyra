/// <reference types="vite/client" />
import source from "./SubscriptionPage.tsx?raw";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { RuntimeObservation } from "../../lib/observability";
import type { SubscriptionResult, SubscriptionSettings, SubscriptionSummary } from "../../lib/subscriptions";
import { activationCompletion, isSubscriptionVisualActive, isSubscriptionCardActive, presentSubscriptionNotice, SubscriptionCard, type PendingActivation } from "./SubscriptionPage";

const observation = (patch: Partial<RuntimeObservation> = {}): RuntimeObservation => ({
  source: "mock", revision: 1, observedAtMs: 1, trafficHistory: [], captureMode: "off", sidecarLifecycle: "ready",
  uploadRateBps: 0, downloadRateBps: 0, uploadTotalBytes: 0, downloadTotalBytes: 0, connectionCount: 0,
  logSummary: [], appliedSubscriptionId: "A", appliedConfigurationGeneration: 1, subscriptionSwitch: null,
  managedProxyAvailable: false, coreMemoryBytes: null, ...patch,
});
const operation = (id = "B"): PendingActivation => ({ id, completion: "operation", operationId: "switch" });
const ready = (id = "B"): RuntimeObservation => observation({ appliedSubscriptionId: id, appliedConfigurationGeneration: 2,
  subscriptionSwitch: { operationId: "switch", status: "ready", errorCode: null } });
const already: PendingActivation = { id: "B", completion: "alreadyCurrent", operationId: "direct", expectedGeneration: 2 };
const card = (id: string, persistedActive: boolean): SubscriptionSummary => ({
  id, name: id, description: "订阅说明", sourceKind: "manual", nodeCount: 1, skippedNodeCount: 0,
  lastSuccessAtMs: null, traffic: null, allowAutoUpdate: false, updateIntervalMinutes: null,
  active: persistedActive, activeConfigurationGeneration: persistedActive ? 99 : null, shareable: false,
});
function markup(id: string, obs: RuntimeObservation | null, pending: PendingActivation | null, persistedActive = false, description = "订阅说明") {
  return renderToStaticMarkup(createElement(SubscriptionCard, {
    subscription: { ...card(id, persistedActive), description }, visualActive: isSubscriptionCardActive(obs, id, pending), activating: pending?.id === id,
    updating: false, selectionMode: false, checked: false, disabled: pending !== null,
    onUse() {}, onUpdate() {}, onContextMenu() {}, onMenuKeyDown() {},
  }));
}

describe("SubscriptionCard runtime-active and pending completion", () => {
  it("pending suppresses A presentation without changing runtime truth or promoting persisted B", () => {
    expect(isSubscriptionVisualActive(observation(), "A")).toBe(true);
    expect(markup("A", observation(), operation(), false)).not.toContain("subscription-card-active");
    const b = markup("B", observation(), operation(), true);
    expect(b).not.toContain("subscription-card-active");
    expect(b).toContain("subscription-card-pending");
    expect(b).toContain('aria-busy="true"');
    expect(b).toContain("切换中…");
  });

  it.each(["activated", "reactivated", "asynchronous pending"])("%s: response before observation keeps target pending until confirmed", () => {
    const pending = operation();
    expect(activationCompletion(pending, observation())).toBeNull();
    expect(markup("B", observation(), pending)).toContain("subscription-card-pending");
    const completed = activationCompletion(pending, ready());
    expect(completed).toEqual({ kind: "success", message: "订阅已应用，服务正在使用最新配置" });
    expect(markup("B", ready(), pending)).toContain("subscription-card-active subscription-card-pending");
    const terminal = markup("B", ready(), null);
    expect(terminal).toContain("subscription-card-active");
    expect(terminal).not.toContain("subscription-card-pending");
    expect(terminal).not.toContain("切换中…");
    expect(activationCompletion(null, ready())).toBeNull();
  });

  it.each(["activated", "reactivated", "asynchronous pending"])("%s: observation before response settles as soon as operation metadata arrives", () => {
    const initial: PendingActivation = { id: "B", completion: "operation", operationId: null };
    expect(activationCompletion(initial, ready())).toBeNull();
    expect(markup("B", ready(), initial)).toContain("subscription-card-active subscription-card-pending");
    expect(activationCompletion(operation(), ready())?.kind).toBe("success");
    expect(markup("B", ready(), null)).toContain("subscription-card-active");
  });

  it.each(["queued", "checking", "prepared", "persisted", "applying"] as const)("%s preserves pending", status => {
    expect(activationCompletion(operation(), observation({ subscriptionSwitch: { operationId: "switch", status, errorCode: null } }))).toBeNull();
  });

  it("AlreadyCurrent uses the delivered exact tuple without a new switch event", () => {
    const arrived = observation({ appliedSubscriptionId: "B", appliedConfigurationGeneration: 2 });
    expect(activationCompletion(already, arrived)).toEqual({ kind: "success", message: "订阅已是当前运行配置" });
    expect(activationCompletion(already, observation())).toBeNull();
    expect(markup("B", observation(), already, true)).not.toContain("subscription-card-active");
    expect(activationCompletion(already, arrived)?.kind).toBe("success");
  });

  it("AlreadyCurrent rejects the wrong generation and ignores unrelated old terminal events", () => {
    const applied = ready();
    expect(activationCompletion(already, { ...applied, appliedConfigurationGeneration: 3 })).toBeNull();
    expect(activationCompletion(already, { ...applied, subscriptionSwitch: { operationId: "old", status: "failed", errorCode: "startFailed" } })?.kind).toBe("success");
    expect(activationCompletion(already, { ...applied, appliedConfigurationGeneration: 3, subscriptionSwitch: { operationId: "direct", status: "ready", errorCode: null } })?.kind).toBe("error");
  });

  it.each([
    ["cancelled", "cancelled", "ready"], ["failed", "configurationFailed", "ready"],
    ["failed", "stopFailed", "recoveryRequired"], ["failed", "startFailed", "stopped"],
    ["failed", "recoveryRequired", "recoveryRequired"], ["failed", null, "stopped"],
  ] as const)("terminal %s/%s/%s clears pending without false active", (status, errorCode, sidecarLifecycle) => {
    const obs = observation({ sidecarLifecycle, subscriptionSwitch: { operationId: "switch", status, errorCode } });
    expect(activationCompletion(operation(), obs)?.kind).toBe("error");
    expect(markup("B", obs, null, true)).not.toContain("subscription-card-active");
    expect(markup("B", obs, null, true)).not.toContain("subscription-card-pending");
    expect(isSubscriptionVisualActive(obs, "A")).toBe(sidecarLifecycle === "ready");
  });

  it("terminal delivered before response waits for identity, then closes; unrelated terminal never closes", () => {
    const obs = observation({ subscriptionSwitch: { operationId: "switch", status: "cancelled", errorCode: "cancelled" } });
    expect(activationCompletion({ id: "B", completion: "operation", operationId: null }, obs)).toBeNull();
    expect(activationCompletion(operation(), obs)?.message).toBe("订阅切换已取消");
    expect(activationCompletion(operation(), { ...obs, subscriptionSwitch: { ...obs.subscriptionSwitch!, operationId: "unrelated" } })).toBeNull();
  });

  it.each([{ appliedSubscriptionId: "A" }, { appliedConfigurationGeneration: null }, { sidecarLifecycle: "stopped" as const }])("matching ready mismatch clears with error %j", patch => {
    expect(activationCompletion(operation(), { ...ready(), ...patch })?.kind).toBe("error");
    expect(isSubscriptionVisualActive({ ...ready(), ...patch }, "B")).toBe(false);
  });

  it("same-card reactivation retains active and waits for the new matching operation", () => {
    const pending = operation("A");
    const old = observation({ subscriptionSwitch: { operationId: "old", status: "ready", errorCode: null } });
    expect(activationCompletion(pending, old)).toBeNull();
    expect(markup("A", old, pending)).toContain("subscription-card-active subscription-card-pending");
    expect(activationCompletion(pending, ready("A"))?.kind).toBe("success");
  });

  it("missing observation never manufactures active or success", () => {
    expect(isSubscriptionVisualActive(null, "B")).toBe(false);
    expect(activationCompletion(operation(), null)).toBeNull();
    expect(activationCompletion(already, null)).toBeNull();
  });

  it("empty descriptions reserve the same existing line-height during pending", async () => {
    const { readFileSync } = await vi.importActual<{ readFileSync(path: URL, encoding: "utf8"): string }>("node:fs");
    const styles = readFileSync(new URL("../../styles.css", import.meta.url), "utf8");
    const before = markup("B", observation(), null, false, "");
    const pending = markup("B", observation(), operation(), false, "");
    const after = markup("B", ready(), null, false, "");
    expect(before).toContain('<p class="subscription-description"></p>');
    expect(after).toContain('<p class="subscription-description"></p>');
    expect(pending).toContain('class="subscription-pending-label"');
    expect(styles).toMatch(/\.subscription-description\s*\{\s*min-height: var\(--font-supporting-line-height\);/);
  });

  it("production wiring has one pre-paint completion site and preserves immediate-error cleanup", () => {
    const activation = source.slice(source.indexOf("  async function activate("), source.indexOf("  async function confirmDelete("));
    const success = activation.slice(activation.indexOf("      upsert(result.subscription)"), activation.indexOf("    } catch"));
    expect(success).toContain('completion: "alreadyCurrent"');
    expect(success).toContain('completion: "operation"');
    expect(success).not.toContain("setPendingActivation(null)");
    expect(success).not.toContain("success(");
    expect(activation).toContain('if (result.status === "error") { setPendingActivation(null); setPendingOperation(null); fail(result.error); return; }');
    expect(activation).toContain("catch { setPendingActivation(null); setPendingOperation(null); failResponse(); }");
    expect(source.match(/useLayoutEffect\(\(\) =>/g)).toHaveLength(1);
    expect(source).toContain("activationCompletion(pendingActivation, observation)");
    expect(source).toContain('useToastOwner("subscription", 6000)');
    expect(source).toContain("window.setTimeout(() => setLocalNotice(current => current === notice ? null : current), 6000)");
    expect(source).toContain("visualActive={isSubscriptionCardActive(observation, subscription.id, pendingActivation)}");
    expect(source).toContain("const applied = isSubscriptionVisualActive(observation, pending.id)");
  });
});

function expectPair(obs: RuntimeObservation | null, pending: PendingActivation | null, expected: string[]) {
  const states = ["A", "B"].map(id => {
    const html = markup(id, obs, pending, id === "B");
    const active = html.includes("subscription-card-active");
    const busy = html.includes("subscription-card-pending");
    expect(html.includes('aria-busy="true"')).toBe(busy);
    return active ? (busy ? "active+pending" : "active") : busy ? "pending" : "normal";
  });
  expect(states.filter(state => state !== "normal").length).toBeLessThanOrEqual(1);
  expect(states).toEqual(expected);
}

describe("SubscriptionCard single emphasis", () => {
  it.each(["response-first", "observation-first"])("%s keeps only the pending target emphasized from its first render", order => {
    const initial: PendingActivation = { id: "B", completion: "operation", operationId: null };
    expectPair(observation(), null, ["active", "normal"]);
    expectPair(observation(), initial, ["normal", "pending"]);
    if (order === "response-first") {
      expect(activationCompletion(operation(), observation())).toBeNull();
      expectPair(observation(), operation(), ["normal", "pending"]);
    } else {
      expect(activationCompletion(initial, ready())).toBeNull();
      expectPair(ready(), initial, ["normal", "active+pending"]);
    }
    expectPair(ready(), operation(), ["normal", "active+pending"]);
    expect(activationCompletion(operation(), ready())?.kind).toBe("success");
    expectPair(ready(), null, ["normal", "active"]);
  });

  it.each(["failed", "cancelled"] as const)("%s restores only the still-Ready applied Card after pending clears", status => {
    const obs = observation({ subscriptionSwitch: { operationId: "switch", status, errorCode: status === "cancelled" ? "cancelled" : "configurationFailed" } });
    expectPair(obs, operation(), ["normal", "pending"]);
    expect(activationCompletion(operation(), obs)?.kind).toBe("error");
    expectPair(obs, null, ["active", "normal"]);
  });

  it.each([
    { sidecarLifecycle: "stopped" },
    { sidecarLifecycle: "recoveryRequired" },
    { appliedSubscriptionId: null },
    { appliedConfigurationGeneration: null },
  ] satisfies Partial<RuntimeObservation>[])("failure with invalid applied tuple %j restores no Card", patch => {
    const obs = observation({ ...patch, subscriptionSwitch: { operationId: "switch", status: "failed", errorCode: "startFailed" } });
    expectPair(obs, operation(), ["normal", "pending"]);
    expect(activationCompletion(operation(), obs)?.kind).toBe("error");
    expectPair(obs, null, ["normal", "normal"]);
  });

  it("AlreadyCurrent retains target-only emphasis until the delivered tuple confirms it", () => {
    expectPair(observation(), already, ["normal", "pending"]);
    expect(activationCompletion(already, observation())).toBeNull();
    const delivered = observation({ appliedSubscriptionId: "B", appliedConfigurationGeneration: 2 });
    expectPair(delivered, already, ["normal", "active+pending"]);
    expect(activationCompletion(already, delivered)?.kind).toBe("success");
    expectPair(delivered, null, ["normal", "active"]);
  });

  it("same-card reactivation keeps one border without manufacturing active while stopped", () => {
    const pending = operation("A");
    expectPair(observation(), null, ["active", "normal"]);
    expectPair(observation(), pending, ["active+pending", "normal"]);
    expectPair(observation({ sidecarLifecycle: "stopped" }), pending, ["pending", "normal"]);
    expectPair(ready("A"), pending, ["active+pending", "normal"]);
    expect(activationCompletion(pending, ready("A"))?.kind).toBe("success");
    expectPair(ready("A"), null, ["active", "normal"]);
  });
});

describe("Subscription notice presentation", () => {
  it("renders a validation alert as a direct Dialog child without global feedback", async () => {
    const harness = await subscriptionPageHarness();
    const button = findElement(harness.render(), element => element.type === "button" && element.props.children === "新建");
    if (button === null) throw new Error("New subscription callback was not found");
    (button.props.onClick as () => void)();
    const form = findElement(harness.render(), element => element.type === "form");
    if (form === null) throw new Error("Create form callback was not found");
    (form.props.onSubmit as (event: unknown) => void)({ preventDefault() {} });
    const dialog = findElement(harness.render(), element => element.props.role === "dialog");
    const alert = findElement(dialog, element => element.props.role === "alert");
    expect(dialog?.props["aria-modal"]).toBe("true");
    expect(alert?.props.children).toBe("请输入有效的订阅文件链接");
    expect(alert?.props.className).toBe("subscription-notice subscription-notice-error");
    expect(dialog?.props.children).toContain(alert);
    expect(harness.globalNotices).toEqual([]);
  });

  it("scopes normal-flow Notice layout to Dialog children and preserves viewport Toast rules", async () => {
    const { readFileSync } = await vi.importActual<{ readFileSync(path: URL, encoding: "utf8"): string }>("node:fs");
    const styles = readFileSync(new URL("../../styles.css", import.meta.url), "utf8");
    const localRules = [...styles.matchAll(/\.subscription-dialog\s*>\s*\.subscription-notice\s*\{([^}]+)\}/g)];
    expect(localRules).toHaveLength(1);
    expect(localRules[0][1].trim().split(/;\s*/).filter(Boolean)).toEqual([
      "position: static", "inset: auto", "z-index: auto", "display: block", "width: auto",
      "max-width: none", "max-height: none", "overflow: visible", "overflow-wrap: anywhere",
      "margin: 0 20px 20px", "padding: var(--component-gap) var(--surface-padding)",
    ]);
    expect(styles).toMatch(/\.subscription-dialog\s*\{[^}]*max-height: 80dvh;[^}]*overflow: auto;/);
    expect(styles).toMatch(/\.subscription-notice\s*\{[^}]*position: fixed;[^}]*z-index: 40;/);
    expect(styles).toMatch(/\.subscription-notice\s*\{[^}]*left: var\(--notice-edge\);[^}]*bottom: var\(--notice-edge\);/);
    expect(styles).toMatch(/\.toast-host\s*\{[^}]*position: fixed;/);
    expect(styles).toMatch(/\.toast-host > \.subscription-notice,[^{]+\{[^}]*position: relative;[^}]*inset: auto;/);
    expect(styles).toMatch(/\.toast-host > \.subscription-notice-success\s*\{\s*max-width: min\(300px, 100%\);/);
  });

  it("dispatches non-Dialog notices globally and Dialog notices locally without duplication", () => {
    const local = vi.fn();
    const global = vi.fn();
    const pageNotice = { kind: "success", message: "订阅已更新，运行配置未切换" } as const;
    const dialogError = { kind: "error", message: "订阅响应不可用，请重试" } as const;

    presentSubscriptionNotice(pageNotice, false, local, global);
    expect(global).toHaveBeenCalledExactlyOnceWith(pageNotice);
    expect(local).not.toHaveBeenCalled();

    local.mockClear(); global.mockClear();
    presentSubscriptionNotice(dialogError, true, local, global);
    expect(local).toHaveBeenCalledExactlyOnceWith(dialogError);
    expect(global).not.toHaveBeenCalled();
  });

  it("retains Subscription Dialog and DocumentEditor contextual error ownership", () => {
    expect(source).toContain("presentSubscriptionNotice(next, dialogIdentityRef.current !== null, setLocalNotice, presentGlobalNotice)");
    expect(source).toContain("{noticeElement}");
    expect(source).toContain('className={`subscription-notice subscription-notice-${notice.kind}`}');
    expect(source).toContain("<DocumentEditor");
    expect(source).toContain("if (result.status === \"error\") throw new Error(documentErrorMessage(result))");
    expect(source).not.toContain("<ToastHost");
  });
});

describe("Subscription dialog Contract sizing", () => {
  it.each([
    ["create", true],
    ["edit", true],
    ["qr", false],
    ["replace", false],
    ["delete", false],
    ["batchDelete", false],
  ] as const)("emits the %s dialog with the expected width class", async (mode, expanded) => {
    const harness = await subscriptionPageHarness();
    harness.render();
    harness.states[9] = mode;
    const dialog = findElement(harness.render(), element => element.props.role === "dialog");
    expect(dialog?.props.className).toBe(expanded ? "subscription-dialog subscription-dialog-expanded" : "subscription-dialog");
  });

  it("binds ordinary, complex and DocumentEditor popup borders and widths to Contract tokens", async () => {
    const { readFileSync } = await vi.importActual<{ readFileSync(path: URL, encoding: "utf8"): string }>("node:fs");
    const styles = readFileSync(new URL("../../styles.css", import.meta.url), "utf8");
    const editorStyles = readFileSync(new URL("./DocumentEditor.css", import.meta.url), "utf8");
    expect(styles).toMatch(/\.subscription-dialog\s*\{[\s\S]*?width: min\(460px, calc\(100vw - 24px\)\);/);
    expect(styles).toMatch(/\.subscription-dialog-expanded\s*\{\s*width: min\(520px, calc\(100vw - 24px\)\);\s*\}/);
    expect(editorStyles).toMatch(/\.document-dialog\s*\{[^}]*border: 1px solid var\(--divider\);/);
  });
});

type ElementLike = { type: unknown; props: Record<string, unknown> };
type Deferred<T> = { promise: Promise<T>; resolve: (value: T) => void; reject: (reason?: unknown) => void };

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((accept, decline) => { resolve = accept; reject = decline; });
  return { promise, resolve, reject };
}

function findElement(root: unknown, predicate: (element: ElementLike) => boolean): ElementLike | null {
  if (Array.isArray(root)) {
    for (const child of root) {
      const found = findElement(child, predicate);
      if (found !== null) return found;
    }
    return null;
  }
  if (typeof root !== "object" || root === null || !("props" in root) || !("type" in root)) return null;
  const element = root as ElementLike;
  if (predicate(element)) return element;
  return findElement(element.props?.children, predicate);
}

async function flushPromises() {
  await Promise.resolve();
  await Promise.resolve();
}

async function subscriptionPageHarness() {
  vi.resetModules();
  if (!("HTMLElement" in globalThis)) vi.stubGlobal("HTMLElement", class HTMLElement {});
  if (!("document" in globalThis)) vi.stubGlobal("document", { activeElement: null });
  const target = card("subscription-a", false);
  target.name = "远程订阅 A";
  target.sourceKind = "remote";
  target.shareable = true;
  const states: unknown[] = [];
  const refs: Array<{ current: unknown }> = [];
  const stateChanges: Array<{ index: number; value: unknown }> = [];
  const globalNotices: Array<{ notice: unknown; dialogMode: unknown; localNotice: unknown }> = [];
  const requestOrder: string[] = [];
  let stateCursor = 0;
  let refCursor = 0;

  const getSettings = vi.fn<(id: string) => Promise<SubscriptionResult<{ settings: SubscriptionSettings }>>>((id: string) => {
    requestOrder.push(`settings:${id}`);
    return Promise.resolve({ status: "ok" as const, settings: manualSettings });
  });
  const getShare = vi.fn<(id: string) => Promise<SubscriptionResult<{ url: string }>>>((id: string) => {
    requestOrder.push(`share:${id}`);
    return Promise.resolve({ status: "ok" as const, url: "https://example.test/subscription" });
  });
  const edit = vi.fn(() => Promise.resolve({ status: "error" as const, error: "saveFailed" as const }));
  const getDocument = vi.fn(() => Promise.resolve({
    status: "ok" as const,
    document: { id: target.id, content: "{}", format: "json" as const, revision: "revision-a", sourceKind: "remote" as const, localOverride: false },
  }));
  const saveDocument = vi.fn(() => Promise.resolve({ status: "error" as const, error: "validationFailed" as const }));

  vi.doMock("react", async (importOriginal) => ({
    ...await importOriginal<typeof import("react")>(),
    useEffect: () => undefined,
    useLayoutEffect: () => undefined,
    useRef: <T,>(initial: T) => {
      const index = refCursor++;
      if (refs[index] === undefined) refs[index] = { current: initial };
      return refs[index];
    },
    useState: <T,>(initial: T | (() => T)) => {
      const index = stateCursor++;
      if (!(index in states)) states[index] = typeof initial === "function" ? (initial as () => T)() : initial;
      const setState = (next: T | ((current: T) => T)) => {
        const value = typeof next === "function" ? (next as (current: T) => T)(states[index] as T) : next;
        states[index] = value;
        stateChanges.push({ index, value });
      };
      return [states[index] as T, setState] as const;
    },
  }));
  vi.doMock("../ToastHost", () => ({
    useToastOwner: () => (notice: unknown) => {
      globalNotices.push({ notice, dialogMode: states[9], localNotice: states[8] });
    },
  }));
  vi.doMock("../../lib/subscriptions", async (importOriginal) => ({
    ...await importOriginal<typeof import("../../lib/subscriptions")>(),
    listSubscriptions: vi.fn(() => Promise.resolve({ status: "ok", subscriptions: [target] })),
    subscribeSubscriptionStateChanged: vi.fn(() => Promise.resolve(() => undefined)),
    getSubscriptionSettings: getSettings,
    getSubscriptionShareUrl: getShare,
    editSubscription: edit,
    getSubscriptionDocument: getDocument,
    saveSubscriptionDocument: saveDocument,
  }));

  const pageModule = await import("./SubscriptionPage");
  states[0] = "ready";
  states[1] = [target];

  function render() {
    stateCursor = 0;
    refCursor = 0;
    return pageModule.SubscriptionPage({ active: true, observation: observation() });
  }
  function openMenu() {
    const page = render();
    const subscriptionCard = findElement(page, (element) =>
      typeof element.type === "function" && element.type.name === "SubscriptionCard");
    if (subscriptionCard === null) throw new Error("SubscriptionCard callback was not found");
    const currentTarget = { querySelector: () => null };
    (subscriptionCard.props.onContextMenu as (event: unknown) => void)({
      preventDefault() {}, stopPropagation() {}, currentTarget, clientX: 10, clientY: 20,
    });
    const menu = findElement(render(), (element) =>
      typeof element.type === "function" && element.type.name === "SubscriptionMenu");
    if (menu === null) throw new Error("SubscriptionMenu callbacks were not found");
    return menu.props;
  }
  function openEdit() { (openMenu().onEdit as () => void)(); }
  function openQr() { (openMenu().onShare as () => void)(); }
  function openDocumentEditor() { (openMenu().onEditFile as () => void)(); }

  return {
    states, stateChanges, globalNotices, requestOrder, target, getSettings, getShare, edit,
    getDocument, saveDocument, render, openEdit, openQr, openDocumentEditor,
  };
}

const manualSettings: SubscriptionSettings = {
  id: "subscription-a", name: "远程订阅 A", description: "订阅说明", sourceKind: "manual" as const,
  urlPreview: null, hasCustomUserAgent: false, remoteRequest: null,
  updatePolicy: { allowAutoUpdate: false, intervalMinutes: null }, shareable: false,
};
const remoteSettings: SubscriptionSettings = {
  ...manualSettings, sourceKind: "remote" as const, urlPreview: "https://example.test/…", shareable: true,
  remoteRequest: { timeoutSeconds: 30, proxyMode: "direct" as const, verifyTls: true },
  updatePolicy: { allowAutoUpdate: true, intervalMinutes: 1440 },
};

function expectClosedGlobalError(harness: Awaited<ReturnType<typeof subscriptionPageHarness>>, message: string) {
  expect(harness.states[9]).toBeNull();
  expect(harness.states[10]).toBeNull();
  expect(harness.states[8]).toBeNull();
  expect(harness.globalNotices).toEqual([{
    notice: { kind: "error", message }, dialogMode: null, localNotice: null,
  }]);
}

describe("Subscription production preload presentation", () => {
  it.each([
    ["settings result-error", "result"],
    ["settings exception", "throw"],
  ] as const)("closes edit and emits one global error after %s", async (_name, outcome) => {
    const harness = await subscriptionPageHarness();
    const pending = deferred<{ status: "error"; error: "notFound" }>();
    harness.getSettings.mockImplementationOnce((id: string) => {
      harness.requestOrder.push(`settings:${id}`);
      return pending.promise;
    });

    harness.openEdit();
    expect(harness.states[9]).toBe("edit");
    outcome === "result" ? pending.resolve({ status: "error", error: "notFound" }) : pending.reject(new Error("settings failed"));
    await flushPromises();

    expectClosedGlobalError(harness, outcome === "result" ? "订阅不存在，请刷新列表" : "订阅响应不可用，请重试");
    expect(harness.requestOrder).toEqual([`settings:${harness.target.id}`]);
    expect(harness.getShare).not.toHaveBeenCalled();
  });

  it.each([
    ["remote share result-error", "result"],
    ["remote share exception", "throw"],
  ] as const)("closes edit and emits one global error after %s", async (_name, outcome) => {
    const harness = await subscriptionPageHarness();
    const pending = deferred<{ status: "error"; error: "notFound" }>();
    harness.getSettings.mockImplementationOnce((id: string) => {
      harness.requestOrder.push(`settings:${id}`);
      return Promise.resolve({ status: "ok", settings: remoteSettings });
    });
    harness.getShare.mockImplementationOnce((id: string) => {
      harness.requestOrder.push(`share:${id}`);
      return pending.promise;
    });

    harness.openEdit();
    expect(harness.states[9]).toBe("edit");
    await flushPromises();
    outcome === "result" ? pending.resolve({ status: "error", error: "notFound" }) : pending.reject(new Error("share failed"));
    await flushPromises();

    expectClosedGlobalError(harness, outcome === "result" ? "订阅不存在，请刷新列表" : "订阅响应不可用，请重试");
    expect(harness.requestOrder).toEqual([`settings:${harness.target.id}`, `share:${harness.target.id}`]);
  });

  it.each([
    ["QR result-error", "result"],
    ["QR exception", "throw"],
  ] as const)("closes QR and emits one global error after %s", async (_name, outcome) => {
    const harness = await subscriptionPageHarness();
    const pending = deferred<{ status: "error"; error: "notFound" }>();
    harness.getShare.mockImplementationOnce((id: string) => {
      harness.requestOrder.push(`share:${id}`);
      return pending.promise;
    });

    harness.openQr();
    expect(harness.states[9]).toBe("qr");
    outcome === "result" ? pending.resolve({ status: "error", error: "notFound" }) : pending.reject(new Error("share failed"));
    await flushPromises();

    expectClosedGlobalError(harness, outcome === "result" ? "订阅不存在，请刷新列表" : "订阅响应不可用，请重试");
    expect(harness.requestOrder).toEqual([`share:${harness.target.id}`]);
  });

  it("ignores stale edit completion at both settings and remote-share awaits", async () => {
    const settingsHarness = await subscriptionPageHarness();
    const settingsPending = deferred<{ status: "error"; error: "notFound" }>();
    settingsHarness.getSettings.mockReturnValueOnce(settingsPending.promise);
    settingsHarness.openEdit();
    settingsHarness.openQr();
    settingsPending.reject(new Error("stale settings"));
    await flushPromises();
    expect(settingsHarness.states[9]).toBe("qr");
    expect(settingsHarness.globalNotices).toEqual([]);

    const shareHarness = await subscriptionPageHarness();
    const editSharePending = deferred<{ status: "error"; error: "notFound" }>();
    const currentQrPending = deferred<{ status: "ok"; url: string }>();
    shareHarness.getSettings.mockResolvedValueOnce({ status: "ok", settings: remoteSettings });
    shareHarness.getShare.mockReturnValueOnce(editSharePending.promise).mockReturnValueOnce(currentQrPending.promise);
    shareHarness.openEdit();
    await flushPromises();
    shareHarness.openQr();
    editSharePending.resolve({ status: "error", error: "notFound" });
    await flushPromises();
    expect(shareHarness.states[9]).toBe("qr");
    expect(shareHarness.globalNotices).toEqual([]);
    currentQrPending.resolve({ status: "ok", url: "https://example.test/current" });
    await flushPromises();
  });

  it("ignores a stale rejected QR completion without closing the newer edit Dialog", async () => {
    const harness = await subscriptionPageHarness();
    const staleQr = deferred<{ status: "ok"; url: string }>();
    harness.getShare.mockReturnValueOnce(staleQr.promise);
    harness.getSettings.mockResolvedValueOnce({ status: "ok", settings: manualSettings });
    harness.openQr();
    harness.openEdit();
    staleQr.reject(new Error("stale QR"));
    await flushPromises();
    expect(harness.states[9]).toBe("edit");
    expect(harness.globalNotices).toEqual([]);
  });

  it("keeps validation and submit failures in an open Dialog", async () => {
    const validation = await subscriptionPageHarness();
    const newButton = findElement(validation.render(), (element) => element.type === "button" && element.props.children === "新建");
    if (newButton === null) throw new Error("New subscription callback was not found");
    (newButton.props.onClick as () => void)();
    const invalidForm = findElement(validation.render(), (element) => element.type === "form");
    if (invalidForm === null) throw new Error("Create form callback was not found");
    (invalidForm.props.onSubmit as (event: unknown) => void)({ preventDefault() {} });
    expect(validation.states[9]).toBe("create");
    expect(validation.states[8]).toEqual({ kind: "error", message: "请输入有效的订阅文件链接" });
    expect(validation.globalNotices).toEqual([]);

    const submit = await subscriptionPageHarness();
    submit.getSettings.mockResolvedValueOnce({ status: "ok", settings: manualSettings });
    submit.openEdit();
    await flushPromises();
    const editForm = findElement(submit.render(), (element) => element.type === "form");
    if (editForm === null) throw new Error("Edit form callback was not found");
    (editForm.props.onSubmit as (event: unknown) => void)({ preventDefault() {} });
    await flushPromises();
    expect(submit.edit).toHaveBeenCalledOnce();
    expect(submit.states[9]).toBe("edit");
    expect(submit.states[8]).toEqual({ kind: "error", message: "订阅保存失败，原有内容未更改" });
    expect(submit.globalNotices).toEqual([]);
  });

  it("keeps overlong QR local and preserves successful manual, remote and QR preloads", async () => {
    const overlong = await subscriptionPageHarness();
    overlong.getShare.mockResolvedValueOnce({ status: "ok", url: "x".repeat(2049) });
    overlong.openQr();
    await flushPromises();
    expect(overlong.states[9]).toBe("qr");
    expect(overlong.states[8]).toEqual({ kind: "error", message: "订阅链接过长，无法生成二维码" });
    expect(overlong.globalNotices).toEqual([]);

    const manual = await subscriptionPageHarness();
    manual.getSettings.mockResolvedValueOnce({ status: "ok", settings: manualSettings });
    manual.openEdit();
    await flushPromises();
    expect(manual.states[9]).toBe("edit");
    expect(manual.states[11]).toEqual(manualSettings);
    expect(manual.getShare).not.toHaveBeenCalled();

    const remote = await subscriptionPageHarness();
    remote.getSettings.mockResolvedValueOnce({ status: "ok", settings: remoteSettings });
    remote.getShare.mockResolvedValueOnce({ status: "ok", url: "https://example.test/remote" });
    remote.openEdit();
    await flushPromises();
    expect(remote.states[9]).toBe("edit");
    expect(remote.states[11]).toEqual(remoteSettings);
    expect(remote.getSettings).toHaveBeenCalledExactlyOnceWith(remote.target.id);
    expect(remote.getShare).toHaveBeenCalledExactlyOnceWith(remote.target.id);
    expect(remote.getSettings.mock.invocationCallOrder[0]).toBeLessThan(remote.getShare.mock.invocationCallOrder[0]);

    const qr = await subscriptionPageHarness();
    qr.getShare.mockResolvedValueOnce({ status: "ok", url: "https://example.test/qr" });
    qr.openQr();
    await flushPromises();
    expect(qr.states[9]).toBe("qr");
    expect(qr.states[16]).toBe("https://example.test/qr");
    expect(qr.globalNotices).toEqual([]);
  });

  it("passes save failures to DocumentEditor without presenting a page or global Notice", async () => {
    const harness = await subscriptionPageHarness();
    harness.openDocumentEditor();
    await flushPromises();
    const editor = findElement(harness.render(), (element) => typeof element.props.onSave === "function" && element.props.title === "编辑文件");
    if (editor === null) throw new Error("DocumentEditor save callback was not found");
    await expect((editor.props.onSave as (content: string) => Promise<void>)("invalid")).rejects.toThrow("订阅内容未通过校验");
    expect(harness.states[8]).toBeNull();
    expect(harness.globalNotices.filter(({ notice }) => notice !== null)).toEqual([]);
    expect(harness.saveDocument).toHaveBeenCalledWith({
      id: harness.target.id, content: "invalid", format: "json", expectedRevision: "revision-a",
    });
  });
});
