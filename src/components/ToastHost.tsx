import { createContext, useCallback, useContext, useEffect, useId, useMemo, useRef, useState, type ReactNode } from "react";

export const SUCCESS_TOAST_DURATION_MS = 3000;

export type ToastRecoveryAction = {
  label: string;
  onInvoke: () => void | Promise<void>;
  disabled?: boolean;
};
export type ToastMessage = {
  message: string;
  kind: "error" | "success" | "warning" | "info";
  action?: ToastRecoveryAction;
};
type ToastStyle = "failure" | "subscription" | "proxy-routing";
export type ToastItem = ToastMessage & { id: number; owner: string; style: ToastStyle; expiresAt: number | null };
export type ToastAction =
  | { type: "append"; item: ToastItem }
  | { type: "replace"; previousId: number | null; owner: string; item: ToastItem }
  | { type: "remove"; id: number; owner: string }
  | { type: "expire"; now: number };

export function toastQueue(items: readonly ToastItem[], action: ToastAction): readonly ToastItem[] {
  if (action.type === "append") return [...items, action.item];
  if (action.type === "replace") {
    const index = items.findIndex(item => item.id === action.previousId && item.owner === action.owner);
    if (index === -1) return [...items, action.item];
    return items.map((item, itemIndex) => itemIndex === index ? action.item : item);
  }
  if (action.type === "remove") return items.filter(item => item.id !== action.id || item.owner !== action.owner);
  return items.filter(item => item.expiresAt === null || item.expiresAt > action.now);
}

type ToastContextValue = {
  enqueue(owner: string, message: ToastMessage, style: ToastStyle, duration: number | null): number;
  replace(owner: string, previousId: number | null, message: ToastMessage, style: ToastStyle, duration: number | null): number;
  remove(id: number, owner: string): void;
};
const ToastContext = createContext<ToastContextValue | null>(null);

export function invokeToastAction(action: ToastRecoveryAction, invoked: { current: boolean }): boolean {
  if (action.disabled || invoked.current) return false;
  invoked.current = true;
  void action.onInvoke();
  return true;
}

function ToastActionButton({ action }: { action: ToastRecoveryAction }) {
  const invoked = useRef(false);
  const [locallyDisabled, setLocallyDisabled] = useState(false);
  return <button type="button" className="toast-action" disabled={action.disabled || locallyDisabled} onClick={() => {
    if (!invokeToastAction(action, invoked)) return;
    setLocallyDisabled(true);
  }}>{action.label}</button>;
}

export function ToastHost({ items, remove }: { items: readonly ToastItem[]; remove: ToastContextValue["remove"] }) {
  return <div className="toast-host">{items.slice(0, 3).map(item => <div key={item.id} data-toast-id={item.id}
    className={item.style === "failure" ? "failure-toast" : item.style === "subscription" ? `subscription-notice subscription-notice-${item.kind}` : `proxy-routing-notice notice-${item.kind}`}
    role={item.kind === "error" ? "alert" : "status"}>
    <span className="toast-content">{item.message}</span>
    {(item.action || (item.kind !== "success" && item.style !== "subscription")) && <span className="toast-actions">
      {item.action && <ToastActionButton action={item.action} />}
      {item.kind !== "success" && item.style !== "subscription" && <button type="button" className="toast-close"
        aria-label={item.style === "failure" ? "关闭失败提示" : "关闭提示"} onClick={() => remove(item.id, item.owner)}>关闭</button>}
    </span>}
  </div>)}</div>;
}

type ToastProviderRefs = {
  nextId: number;
  items: readonly ToastItem[];
  focusOrigins: Map<number, HTMLElement>;
  pendingFocus: HTMLElement | null;
};

function currentFocusOrigin(): HTMLElement | null {
  if (typeof document === "undefined") return null;
  return document.activeElement instanceof HTMLElement ? document.activeElement : null;
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<readonly ToastItem[]>([]);
  const refs = useRef<ToastProviderRefs>({ nextId: 0, items: [], focusOrigins: new Map(), pendingFocus: null });
  refs.current.items = items;
  const prepareFocusReturn = useCallback((id: number) => {
    if (typeof document === "undefined") return;
    const toast = document.querySelector<HTMLElement>(`[data-toast-id="${id}"]`);
    if (toast?.contains(document.activeElement)) refs.current.pendingFocus = refs.current.focusOrigins.get(id) ?? null;
  }, []);
  const remove = useCallback((id: number, owner: string) => {
    if (refs.current.items.some(item => item.id === id && item.owner === owner)) prepareFocusReturn(id);
    setItems(current => toastQueue(current, { type: "remove", id, owner }));
  }, [prepareFocusReturn]);
  const makeItem = useCallback((owner: string, message: ToastMessage, style: ToastStyle, duration: number | null) => {
    const id = ++refs.current.nextId;
    const lifetime = message.kind === "success" ? SUCCESS_TOAST_DURATION_MS : duration;
    const item = { ...message, id, owner, style, expiresAt: lifetime === null ? null : Date.now() + lifetime };
    const focusOrigin = currentFocusOrigin();
    if (focusOrigin) refs.current.focusOrigins.set(id, focusOrigin);
    return item;
  }, []);
  const enqueue = useCallback((owner: string, message: ToastMessage, style: ToastStyle, duration: number | null) => {
    const item = makeItem(owner, message, style, duration);
    setItems(current => toastQueue(current, { type: "append", item }));
    return item.id;
  }, [makeItem]);
  const replace = useCallback((owner: string, previousId: number | null, message: ToastMessage, style: ToastStyle, duration: number | null) => {
    const item = makeItem(owner, message, style, duration);
    if (previousId !== null && refs.current.items.some(current => current.id === previousId && current.owner === owner)) {
      prepareFocusReturn(previousId);
      const previousOrigin = refs.current.focusOrigins.get(previousId);
      if (previousOrigin) refs.current.focusOrigins.set(item.id, previousOrigin);
    }
    setItems(current => toastQueue(current, { type: "replace", previousId, owner, item }));
    return item.id;
  }, [makeItem, prepareFocusReturn]);
  useEffect(() => {
    const deadlines = items.flatMap(item => item.expiresAt === null ? [] : [item.expiresAt]);
    if (!deadlines.length) return;
    const timer = window.setTimeout(() => {
      const now = Date.now();
      for (const item of items) {
        if (item.expiresAt !== null && item.expiresAt <= now) prepareFocusReturn(item.id);
      }
      setItems(current => toastQueue(current, { type: "expire", now }));
    }, Math.max(0, Math.min(...deadlines) - Date.now()));
    return () => window.clearTimeout(timer);
  }, [items, prepareFocusReturn]);
  useEffect(() => {
    const currentIds = new Set(items.map(item => item.id));
    for (const id of refs.current.focusOrigins.keys()) {
      if (!currentIds.has(id)) refs.current.focusOrigins.delete(id);
    }
    const target = refs.current.pendingFocus;
    refs.current.pendingFocus = null;
    if (target?.isConnected) target.focus();
  }, [items]);
  const value = useMemo(() => ({ enqueue, replace, remove }), [enqueue, replace, remove]);
  return <ToastContext.Provider value={value}>{children}<ToastHost items={items} remove={remove} /></ToastContext.Provider>;
}

export type ToastOwnerPresenter = {
  (message: ToastMessage | null): number | null;
  replace(message: ToastMessage): number;
  clear(expectedId?: number): void;
};

// The callable form preserves append behavior. replace/clear provide exact owner-item lifecycle control.
export function useToastOwner(style: ToastStyle, duration: number | null): ToastOwnerPresenter {
  const context = useContext(ToastContext);
  if (!context) throw new Error("ToastProvider is required");
  const { enqueue, replace, remove } = context;
  const owner = useId();
  const currentId = useRef<number | null>(null);
  const present = useCallback((message: ToastMessage | null) => {
    if (message === null) {
      if (currentId.current !== null) remove(currentId.current, owner);
      currentId.current = null;
      return null;
    }
    const id = enqueue(owner, message, style, duration);
    currentId.current = id;
    return id;
  }, [enqueue, remove, owner, style, duration]);
  return useMemo(() => Object.assign(present, {
    replace(message: ToastMessage) {
      const id = replace(owner, currentId.current, message, style, duration);
      currentId.current = id;
      return id;
    },
    clear(expectedId = currentId.current ?? undefined) {
      if (expectedId === undefined) return;
      remove(expectedId, owner);
      if (currentId.current === expectedId) currentId.current = null;
    },
  }), [duration, owner, present, remove, replace, style]);
}
