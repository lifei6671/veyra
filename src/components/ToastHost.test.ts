import { createElement, StrictMode } from "react";
import * as React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
vi.mock("react", async importOriginal => ({ ...await importOriginal<typeof import("react")>() }));
import source from "./ToastHost.tsx?raw";
import {
  SUCCESS_TOAST_DURATION_MS,
  ToastHost,
  ToastProvider,
  invokeToastAction,
  toastQueue,
  useToastOwner,
  type ToastItem,
} from "./ToastHost";

const item = (id: number, owner = "subscription", expiresAt: number | null = null): ToastItem =>
  ({ id, owner, expiresAt, message: `message ${id}`, kind: "success", style: "subscription" });
const html = (items: readonly ToastItem[]) => renderToStaticMarkup(createElement(ToastHost, { items, remove: vi.fn() }));

describe("shared Toast presentation", () => {
  afterEach(() => { vi.useRealTimers(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });

  it("uses the original enqueue time and expires every ordinary success at exactly 3000ms", () => {
    vi.useFakeTimers(); vi.setSystemTime(0);
    vi.stubGlobal("window", { setTimeout, clearTimeout });
    let state: readonly ToastItem[] = [];
    const refs = { current: { nextId: 0, items: [] as readonly ToastItem[], focusOrigins: new Map(), pendingFocus: null } };
    const effects: React.EffectCallback[] = [];
    vi.spyOn(React, "useState").mockImplementation((() => [state, (update: (current: readonly ToastItem[]) => readonly ToastItem[]) => {
      const first = update(state);
      expect(update(state)).toEqual(first);
      state = first;
    }]) as typeof React.useState);
    vi.spyOn(React, "useRef").mockReturnValue(refs);
    vi.spyOn(React, "useCallback").mockImplementation(callback => callback);
    vi.spyOn(React, "useMemo").mockImplementation(factory => factory());
    vi.spyOn(React, "useEffect").mockImplementation(next => { effects.push(next); });
    const api = ToastProvider({ children: null }).props.value;
    const first = api.enqueue("app", { message: "same", kind: "error" }, "failure", 6000);
    vi.advanceTimersByTime(1000);
    for (let i = 0; i < 3; i++) api.enqueue("subscription", { message: "same", kind: "success" }, "subscription", null);
    expect(state.map(entry => entry.id)).toEqual([1, 2, 3, 4]);
    expect(state.map(entry => entry.expiresAt)).toEqual([6000, 4000, 4000, 4000]);
    ToastProvider({ children: null });
    const cleanup = effects.at(-2)?.();
    vi.advanceTimersByTime(2999);
    expect(state.map(entry => entry.id)).toEqual([1, 2, 3, 4]);
    vi.advanceTimersByTime(1);
    expect(state.map(entry => entry.id)).toEqual([1]);
    api.remove(first, "app");
    expect(state).toEqual([]);
    if (typeof cleanup === "function") cleanup();
    ToastProvider({ children: null });
    const dispose = effects.at(-2)?.();
    expect(vi.getTimerCount()).toBe(0);
    if (typeof dispose === "function") dispose();
    expect(vi.getTimerCount()).toBe(0);
    expect(SUCCESS_TOAST_DURATION_MS).toBe(3000);
  });

  it("restores the enqueue focus origin when a focused failure toast expires", () => {
    vi.useFakeTimers(); vi.setSystemTime(0);
    class FakeElement {
      isConnected = true;
      focus = vi.fn();
      child: unknown = null;
      contains(node: unknown) { return node === this.child; }
    }
    const origin = new FakeElement();
    const close = new FakeElement();
    const toast = new FakeElement(); toast.child = close;
    let activeElement: unknown = origin;
    vi.stubGlobal("HTMLElement", FakeElement);
    vi.stubGlobal("document", {
      get activeElement() { return activeElement; },
      querySelector: vi.fn().mockReturnValue(toast),
    });
    vi.stubGlobal("window", { setTimeout, clearTimeout });
    let state: readonly ToastItem[] = [];
    const refs = { current: { nextId: 0, items: [] as readonly ToastItem[], focusOrigins: new Map(), pendingFocus: null } };
    const effects: React.EffectCallback[] = [];
    vi.spyOn(React, "useState").mockImplementation((() => [state, (update: (current: readonly ToastItem[]) => readonly ToastItem[]) => {
      const first = update(state);
      expect(update(state)).toEqual(first);
      state = first;
    }]) as typeof React.useState);
    vi.spyOn(React, "useRef").mockReturnValue(refs);
    vi.spyOn(React, "useCallback").mockImplementation(callback => callback);
    vi.spyOn(React, "useMemo").mockImplementation(factory => factory());
    vi.spyOn(React, "useEffect").mockImplementation(next => { effects.push(next); });

    const api = ToastProvider({ children: null }).props.value;
    const id = api.enqueue("app", { message: "failed", kind: "error" }, "failure", 6000);
    ToastProvider({ children: null });
    activeElement = close;
    api.remove(id, "another-owner");
    expect(state).toHaveLength(1);
    expect(refs.current.pendingFocus).toBeNull();

    const cleanup = effects.at(-2)?.();
    vi.advanceTimersByTime(5999);
    expect(state).toHaveLength(1);
    expect(origin.focus).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(state).toHaveLength(0);
    expect(refs.current.pendingFocus).toBe(origin);
    ToastProvider({ children: null });
    effects.at(-1)?.();
    expect(origin.focus).toHaveBeenCalledTimes(1);
    expect(refs.current.pendingFocus).toBeNull();
    expect(refs.current.focusOrigins.has(id)).toBe(false);
    if (typeof cleanup === "function") cleanup();
  });

  it("production owner keeps independent emission identities and clears only its latest one", () => {
    const enqueue = vi.fn().mockReturnValueOnce(11).mockReturnValueOnce(12);
    const replace = vi.fn().mockReturnValueOnce(13).mockReturnValueOnce(14);
    const remove = vi.fn();
    vi.spyOn(React, "useContext").mockReturnValue({ enqueue, replace, remove });
    vi.spyOn(React, "useId").mockReturnValue("owner-A");
    vi.spyOn(React, "useRef").mockReturnValue({ current: null });
    vi.spyOn(React, "useCallback").mockImplementation(callback => callback);
    vi.spyOn(React, "useMemo").mockImplementation(factory => factory());
    const present = useToastOwner("subscription", 6000);
    present({ message: "相同", kind: "success" }); present({ message: "相同", kind: "success" });
    expect(enqueue).toHaveBeenCalledTimes(2);
    expect(remove).not.toHaveBeenCalled();
    expect(present.replace({ message: "需处理", kind: "warning" })).toBe(13);
    expect(replace).toHaveBeenCalledExactlyOnceWith("owner-A", 12, { message: "需处理", kind: "warning" }, "subscription", 6000);
    present.clear(12); // stale identity cannot clear the replacement
    present.clear();
    expect(remove.mock.calls).toEqual([[12, "owner-A"], [13, "owner-A"]]);
    present(null);
    expect(remove).toHaveBeenCalledTimes(2);
  });

  it("renders one, two and three independent occurrences in stable FIFO order", () => {
    let queue: readonly ToastItem[] = [];
    for (const id of [1, 2, 3]) {
      queue = toastQueue(queue, { type: "append", item: item(id) });
      const output = html(queue);
      expect([...output.matchAll(/data-toast-id="(\d+)"/g)].map(match => Number(match[1])))
        .toEqual(Array.from({ length: id }, (_, index) => index + 1));
    }
  });

  it("keeps fourth item waiting and promotes it at the bottom after an explicit dismissal", () => {
    const queue = [item(1), item(2), item(3), item(4)];
    expect(html(queue)).not.toContain("message 4");
    const promoted = toastQueue(queue, { type: "remove", id: 2, owner: "subscription" });
    expect(promoted.map(entry => entry.id)).toEqual([1, 3, 4]);
    expect(html(promoted)).toContain("message 4");
    expect(queue.map(entry => entry.id)).toEqual([1, 2, 3, 4]);
  });

  it("charges waiting time to the original lifetime and promotion cannot extend it", () => {
    vi.useFakeTimers(); vi.setSystemTime(0);
    let queue: readonly ToastItem[] = [item(1), item(2), item(3), item(4, "subscription", Date.now() + 3000)];
    vi.advanceTimersByTime(2999);
    queue = toastQueue(queue, { type: "remove", id: 1, owner: "subscription" });
    expect(html(queue)).toContain("message 4");
    expect(queue[2].expiresAt).toBe(3000);
    vi.advanceTimersByTime(1);
    queue = toastQueue(queue, { type: "expire", now: Date.now() });
    expect(html(queue)).not.toContain("message 4");
    expect(queue.map(entry => entry.id)).toEqual([2, 3]);
  });

  it("expires a waiting item without displaying it and keeps persistent routing notices", () => {
    const queue = [item(1, "routing"), item(2), item(3), item(4, "app", 6000), item(5, "app", 7000)];
    const expired = toastQueue(queue, { type: "expire", now: 6000 });
    expect(expired.map(entry => entry.id)).toEqual([1, 2, 3, 5]);
    expect(html(expired)).not.toContain("message 5");
    expect(toastQueue(expired, { type: "remove", id: 2, owner: "subscription" }).map(entry => entry.id)).toEqual([1, 3, 5]);
  });

  it("a late old-item clear or expiry cannot delete a newer same-owner item", () => {
    const queue = [item(1, "app", 6000), item(2, "app", 9000), item(3, "routing")];
    const firstDismiss = toastQueue(queue, { type: "remove", id: 1, owner: "app" });
    const lateDismiss = toastQueue(firstDismiss, { type: "remove", id: 1, owner: "app" });
    const lateTimer = toastQueue(lateDismiss, { type: "expire", now: 6000 });
    expect(lateTimer.map(entry => entry.id)).toEqual([2, 3]);
  });

  it("replaces only the exact owner item in place and appends when that identity is stale", () => {
    const replacement = { ...item(4, "app"), kind: "warning" as const, message: "new recovery" };
    const queue = [item(1, "app"), item(2, "subscription"), item(3, "routing")];
    const replaced = toastQueue(queue, { type: "replace", previousId: 1, owner: "app", item: replacement });
    expect(replaced.map(entry => entry.id)).toEqual([4, 2, 3]);
    expect(replaced[0].message).toBe("new recovery");
    expect(toastQueue(replaced, { type: "replace", previousId: 1, owner: "app", item: item(5, "app") })
      .map(entry => entry.id)).toEqual([4, 2, 3, 5]);
    expect(toastQueue(replaced, { type: "replace", previousId: 4, owner: "routing", item: item(6, "routing") })
      .map(entry => entry.id)).toEqual([4, 2, 3, 6]);
  });

  it("explicit clear only removes the specified owner's current identity", () => {
    const queue = [item(1, "app"), item(2, "subscription"), item(3, "routing"), item(4, "app")];
    expect(toastQueue(queue, { type: "remove", id: 4, owner: "subscription" })).toEqual(queue);
    expect(toastQueue(queue, { type: "remove", id: 4, owner: "app" }).map(entry => entry.id)).toEqual([1, 2, 3]);
  });

  it("repeated equal messages remain separate presentations, including one batch", () => {
    const repeated = { message: "相同提示", kind: "success" as const };
    const queue = [1, 2, 3, 4].reduce<readonly ToastItem[]>((current, id) =>
      toastQueue(current, { type: "append", item: { ...item(id), ...repeated } }), []);
    expect(queue).toHaveLength(4);
    expect(html(queue).match(/相同提示/g)).toHaveLength(3);
  });

  it("invokes a typed recovery action exactly once and never invokes a disabled action", () => {
    const invoke = vi.fn();
    const invoked = { current: false };
    const action = { label: "重试应用", onInvoke: invoke };
    expect(invokeToastAction(action, invoked)).toBe(true);
    expect(invokeToastAction(action, invoked)).toBe(false);
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invokeToastAction({ ...action, disabled: true }, { current: false })).toBe(false);
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("renders persistent warning and error recovery controls without a timer deadline", () => {
    const action = { label: "重试应用", onInvoke: vi.fn() };
    const output = html([
      { ...item(1, "routing"), kind: "warning", style: "proxy-routing", action },
      { ...item(2, "app"), kind: "error", style: "failure", action: { ...action, disabled: true } },
    ]);
    expect(output.match(/重试应用/g)).toHaveLength(2);
    expect(output).toContain("disabled");
    expect(output.match(/class="toast-close"/g)).toHaveLength(2);
    const stillPersistent = toastQueue([
      { ...item(1), kind: "warning" },
      { ...item(2), kind: "error" },
    ], { type: "expire", now: Number.MAX_SAFE_INTEGER });
    expect(stillPersistent).toHaveLength(2);
  });

  it("preserves message roles, non-success close affordances and escaped text", () => {
    const output = html([
      { ...item(1, "app"), kind: "error", style: "failure", message: "<failure>" },
      { ...item(2, "routing"), kind: "warning", style: "proxy-routing" },
      { ...item(3, "routing"), kind: "info", style: "proxy-routing" },
    ]);
    expect(output).toContain('role="alert"');
    expect(output.match(/role="status"/g)).toHaveLength(2);
    expect(output).toContain('aria-label="关闭失败提示"');
    expect(output).toContain('aria-label="关闭提示"');
    expect(output).toContain("notice-info");
    expect(output).toContain("&lt;failure&gt;");
    expect(html([item(1)])).not.toContain("<button");
    expect(html([{ ...item(1, "app"), style: "failure" }])).not.toContain("<button");
  });

  it("empty StrictMode provider renders no toast and scheduling uses emission deadlines", () => {
    const output = renderToStaticMarkup(createElement(StrictMode, null, createElement(ToastProvider, { children: "page content" })));
    expect(output).toContain("page content");
    expect(output).not.toContain("data-toast-id");
    expect(source).toContain("Date.now() + lifetime");
    expect(source).toContain("window.clearTimeout(timer)");
    expect(source).toContain("target?.isConnected");
    expect(source.slice(source.indexOf("  useEffect("), source.indexOf("  const value ="))).not.toContain("enqueue(");
  });
});
