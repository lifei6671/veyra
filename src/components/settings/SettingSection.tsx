import type { PropsWithChildren } from "react";

type SettingSectionProps = PropsWithChildren<{
  title: string;
}>;

export function SettingSection({ title, children }: SettingSectionProps) {
  return (
    <section className="setting-section" aria-labelledby={`setting-section-${toId(title)}`}>
      <h2 id={`setting-section-${toId(title)}`}>{title}</h2>
      <div className="setting-list">{children}</div>
    </section>
  );
}

function toId(value: string) {
  return Array.from(value).map((character) => character.codePointAt(0)?.toString(16)).join("-");
}
