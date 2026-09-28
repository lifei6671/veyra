import type { ButtonHTMLAttributes, ReactNode } from "react";
import { ExclamationTriangleIcon } from "@heroicons/react/24/outline";

export function Glyph({ children }: { children: ReactNode }) {
  return <span className="glyph" aria-hidden="true">{children}</span>;
}

export function IconButton({ label, children, active, className = "", ...props }: ButtonHTMLAttributes<HTMLButtonElement> & {
  label: string;
  children: ReactNode;
  active?: boolean;
}) {
  return <button
    type="button"
    aria-label={label}
    title={label}
    className={`icon-button${active ? " active" : ""}${className ? ` ${className}` : ""}`}
    {...props}
  >{children}</button>;
}

export function PageToolbar({ title, subtitle, children }: { title: string; subtitle?: string; children?: ReactNode }) {
  return <header className="page-toolbar"><div><h1>{title}</h1>{subtitle && <p>{subtitle}</p>}</div>{children && <div className="toolbar-actions">{children}</div>}</header>;
}

export function EmptyState({ icon, title, text, compact = false }: { icon: ReactNode; title: string; text: string; compact?: boolean }) {
  return <div className={`empty-state${compact ? " compact" : ""}`}><i>{icon}</i><h2>{title}</h2><p>{text}</p></div>;
}

export function LoadingView({ label = "正在连接 Open-Box 后端" }: { label?: string }) {
  return <main className="page loading-page" aria-label={label}><span className="loading-spinner" /><strong>{label}</strong><small>192.168.1.10:3036</small></main>;
}

export function ErrorState({ title = "加载失败", error, onRetry }: { title?: string; error: unknown; onRetry?: () => void }) {
  const message = error instanceof Error ? error.message : String(error);
  return <section className="surface error-state" role="alert"><ExclamationTriangleIcon aria-hidden="true" /><div><h2>{title}</h2><p>{message}</p></div>{onRetry && <button type="button" className="primary-button" onClick={onRetry}>重新加载</button>}</section>;
}

export function Modal({ title, children, onClose, footer }: { title: string; children: ReactNode; onClose: () => void; footer?: ReactNode }) {
  return <div className="ob-modal-backdrop" role="presentation" onMouseDown={event => { if (event.target === event.currentTarget) onClose(); }}>
    <section className="ob-modal" role="dialog" aria-modal="true" aria-labelledby="ob-modal-title">
      <header><h2 id="ob-modal-title">{title}</h2><button type="button" aria-label="关闭" onClick={onClose}>×</button></header>
      <div className="ob-modal-body">{children}</div>
      {footer && <footer>{footer}</footer>}
    </section>
  </div>;
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
  return <div className="promo-bar">优惠购买AI接口、机场、VPS、住宅IP，请访问：<a href="https://blog.angeworld.cc/market" target="_blank" rel="noreferrer">安格超市</a><span>AI 中转站：</span><a href="https://ai.superdoor.top/" target="_blank" rel="noreferrer">SUPERDOOR 订阅服务</a><i>|</i><a href="https://ai.opendoor.sbs/" target="_blank" rel="noreferrer">OPENDOOR 按需付费</a></div>;
}

export function SettingRow({ label, note, children }: { label: string; note?: string; children: ReactNode }) {
  return <div className="setting-row"><div><b>{label}</b>{note && <span>{note}</span>}</div><div className="setting-control">{children}</div></div>;
}

export function CompactSetting({ label, children }: { label: string; children: ReactNode }) {
  return <div className="compact-setting"><span>{label}</span><div>{children}</div></div>;
}
