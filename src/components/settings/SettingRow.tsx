import type { ReactNode } from "react";

type SettingRowProps = {
  label: string;
  description?: string;
  value?: ReactNode;
  control?: ReactNode;
  onActivate?: () => void;
  tone?: "default" | "error";
};

export function SettingRow({ label, description, value, control, onActivate, tone = "default" }: SettingRowProps) {
  const content = <>
      <div className="setting-row-copy">
        <div className="setting-row-label">{label}</div>
        {description ? <div className="setting-row-description">{description}</div> : null}
      </div>
      <div className="setting-row-control">
        {control ?? <span className="setting-row-value">{value}</span>}
      </div>
    </>;
  const className = `setting-row${onActivate ? " setting-row-clickable" : ""}${tone === "error" ? " setting-row-error" : ""}`;

  return onActivate ? <button type="button" className={className} onClick={onActivate}>{content}</button> :
    <div className={className}>{content}</div>;
}
