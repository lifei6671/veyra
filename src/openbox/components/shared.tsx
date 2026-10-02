import { useEffect, useId, useLayoutEffect, useRef, useState, type ButtonHTMLAttributes, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { ExclamationTriangleIcon, XMarkIcon } from "@heroicons/react/24/outline";

export function Glyph({ children }: { children: ReactNode }) {
  return <span className="glyph" aria-hidden="true">{children}</span>;
}

export function Tooltip({ label, children, enabled = true }: { label: string; children: ReactNode; enabled?: boolean }) {
  const id = useId();
  const anchor = useRef<HTMLElement | null>(null), layer = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false), [position, setPosition] = useState({ left: 0, top: 0 });
  const show = (element: HTMLElement) => {
    const dialogs = document.querySelectorAll('[role="dialog"]');
    if (!enabled || dialogs.length && !dialogs[dialogs.length - 1].contains(element) && !element.closest("[role=listbox]")) return;
    anchor.current = element; setOpen(true);
  };
  useLayoutEffect(() => {
    if (!open || !enabled || !anchor.current || !layer.current) return;
    const target = anchor.current.getBoundingClientRect(), tip = layer.current.getBoundingClientRect();
    const above = target.top - tip.height - 8;
    setPosition({ left: Math.max(8, Math.min(window.innerWidth - tip.width - 8, target.left + (target.width - tip.width) / 2)), top: above >= 8 ? above : Math.min(window.innerHeight - tip.height - 8, target.bottom + 8) });
  }, [open, enabled, label]);
  useEffect(() => {
    if (!open) return;
    const close = () => setOpen(false);
    window.addEventListener("scroll", close, true); window.addEventListener("resize", close);
    return () => { window.removeEventListener("scroll", close, true); window.removeEventListener("resize", close); };
  }, [open]);
  return <span className="ob-tooltip-anchor" onPointerEnter={event => show(event.currentTarget.firstElementChild as HTMLElement)} onPointerLeave={() => setOpen(false)} onFocus={event => show(event.target as HTMLElement)} onBlur={() => setOpen(false)} onClickCapture={() => setOpen(false)} onKeyDown={event => { if (event.key === "Escape") setOpen(false); }} aria-describedby={open && enabled ? id : undefined}>
    {children}{open && enabled && createPortal(<div ref={layer} id={id} role="tooltip" className="ob-portal-tooltip" style={position}>{label}</div>, document.body)}
  </span>;
}

export function IconButton({ label, children, active, tooltip = false, portalTooltip = false, className = "", ...props }: ButtonHTMLAttributes<HTMLButtonElement> & {
  label: string;
  children: ReactNode;
  active?: boolean;
  tooltip?: boolean;
  portalTooltip?: boolean;
}) {
  const button = <button
    type="button"
    aria-label={label}
    title={tooltip ? undefined : label}
    data-tooltip={tooltip && !portalTooltip ? label : undefined}
    className={`icon-button${active ? " active" : ""} ${portalTooltip ? className.split(" ").filter(name => !name.startsWith("tooltip-")).join(" ") : className}`.trim()}
    {...props}
  >{children}</button>;
  return portalTooltip ? <Tooltip label={label}>{button}</Tooltip> : button;
}

export function PageToolbar({ title, subtitle, children }: { title: string; subtitle?: string; children?: ReactNode }) {
  return <header className="page-toolbar"><div><h1>{title}</h1>{subtitle && <p>{subtitle}</p>}</div>{children && <div className="toolbar-actions">{children}</div>}</header>;
}

export function EmptyState({ icon, title, text, compact = false }: { icon: ReactNode; title: string; text: string; compact?: boolean }) {
  return <div className={`empty-state${compact ? " compact" : ""}`}><i>{icon}</i><h2>{title}</h2><p>{text}</p></div>;
}

export function LoadingView({ label = "正在连接 Open-Box 后端" }: { label?: string }) {
  return <main className="page loading-page" aria-label={label}><span className="loading-spinner" /><strong>{label}</strong><small>https://openbox.disign.me</small></main>;
}

export function ErrorState({ title = "加载失败", error, onRetry }: { title?: string; error: unknown; onRetry?: () => void }) {
  const message = error instanceof Error ? error.message : String(error);
  return <section className="surface error-state" role="alert"><ExclamationTriangleIcon aria-hidden="true" /><div><h2>{title}</h2><p>{message}</p></div>{onRetry && <button type="button" className="primary-button" onClick={onRetry}>重新加载</button>}</section>;
}

export function Modal({ title, children, onClose, footer, className = "" }: { title: string; children: ReactNode; onClose: () => void; footer?: ReactNode; className?: string }) {
  const titleId = useId();
  const [container, setContainer] = useState<HTMLElement | null>(null);
  useEffect(() => { setContainer(document.querySelector(".openbox-app") ?? document.body); }, []);
  if (!container) return null;
  return createPortal(<div className="ob-modal-backdrop" role="presentation" onMouseDown={event => { if (event.target === event.currentTarget) onClose(); }}>
    <section className={`ob-modal${className ? ` ${className}` : ""}`} role="dialog" aria-modal="true" aria-labelledby={titleId}>
      <header><h2 id={titleId}>{title}</h2><button type="button" aria-label="关闭" data-tooltip="关闭" className="tooltip-trigger tooltip-align-right" onClick={onClose}><XMarkIcon aria-hidden="true" /></button></header>
      <div className="ob-modal-body">{children}</div>
      {footer && <footer>{footer}</footer>}
    </section>
  </div>, container);
}

export function ConfirmButton({ children, confirmText, onConfirm, className = "", disabled = false }: {
  children: ReactNode;
  confirmText: string;
  onConfirm: () => void | Promise<void>;
  className?: string;
  disabled?: boolean;
}) {
  const handleClick = () => {
    if (window.confirm(confirmText)) void onConfirm();
  };
  return <button type="button" className={className} disabled={disabled} onClick={handleClick}>{children}</button>;
}

export function PromoBar() {
  return <div className="promo-bar">
    <span>优惠购买AI接口、机场、VPS、住宅IP，请访问：</span>
    <a href="https://blog.angeworld.cc/market" target="_blank" rel="noreferrer">安格超市</a>
    <span className="promo-right"><span>AI 中转站：</span><a href="https://ai.superdoor.top/" target="_blank" rel="noreferrer">SUPERDOOR <small>订阅服务</small></a><i>|</i><a href="https://ai.opendoor.sbs/" target="_blank" rel="noreferrer">OPENDOOR <small>按需付费</small></a></span>
  </div>;
}

export function SettingRow({ label, note, children }: { label: string; note?: string; children: ReactNode }) {
  return <div className="setting-row"><div><b>{label}</b>{note && <span>{note}</span>}</div><div className="setting-control">{children}</div></div>;
}

export function CompactSetting({ label, children }: { label: string; children: ReactNode }) {
  return <div className="compact-setting"><span>{label}</span><div>{children}</div></div>;
}
