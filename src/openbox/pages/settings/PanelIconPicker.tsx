import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { ChevronDownIcon, XMarkIcon } from "@heroicons/react/24/outline";
import icons from "../../assets/panel-icons.json";

export const panelIcons = icons;

export function PanelIconPicker({ value, label, showLabel = false, emptyLabel = "无", onChange }: { value: string; label: string; showLabel?: boolean; emptyLabel?: string; onChange: (value: string) => void }) {
  const [position, setPosition] = useState<{ top: number; left: number } | null>(null);
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("all");
  const trigger = useRef<HTMLButtonElement>(null);
  const popup = useRef<HTMLDivElement>(null);
  const active = useRef<HTMLButtonElement>(null);
  const searchInput = useRef<HTMLInputElement>(null);
  const current = icons.find(icon => icon.code === value);
  const options = icons.filter(icon => (category === "all" || icon.category === category) && `${icon.label} ${icon.code}`.toLowerCase().includes(search.toLowerCase()));

  useEffect(() => {
    if (!position) return;
    const list = active.current?.closest("ul");
    if (list && active.current) list.scrollTop = active.current.offsetTop - list.offsetTop - list.clientHeight / 2 + active.current.clientHeight / 2;
    const dismiss = (event: PointerEvent) => {
      if (!popup.current?.contains(event.target as Node) && !trigger.current?.contains(event.target as Node)) setPosition(null);
    };
    const escape = (event: KeyboardEvent) => { if (event.key === "Escape") { setPosition(null); trigger.current?.focus(); } };
    const close = () => setPosition(null);
    const scroll = (event: Event) => { if (!popup.current?.contains(event.target as Node)) close(); };
    document.addEventListener("pointerdown", dismiss);
    document.addEventListener("keydown", escape);
    window.addEventListener("resize", close);
    window.addEventListener("scroll", scroll, true);
    return () => {
      document.removeEventListener("pointerdown", dismiss);
      document.removeEventListener("keydown", escape);
      window.removeEventListener("resize", close);
      window.removeEventListener("scroll", scroll, true);
    };
  }, [position]);

  const open = () => {
    if (position) { setPosition(null); return; }
    const rect = trigger.current!.getBoundingClientRect();
    setSearch(""); setCategory("all");
    const below = rect.bottom + 4;
    const top = below + 320 <= window.innerHeight - 8 ? below : rect.top >= 328 ? rect.top - 324 : Math.max(8, window.innerHeight - 328);
    setPosition({ left: Math.max(8, Math.min(rect.left, window.innerWidth - 264)), top });
  };

  return <>
    <button ref={trigger} className={showLabel ? "group-icon-picker" : "site-icon-picker"} type="button" aria-label={label} aria-expanded={Boolean(position)} onClick={open}>
      {current ? <img src={current.asset} alt="" /> : showLabel && <span className="group-icon-empty">—</span>}{showLabel && <span>{current?.label ?? (value || emptyLabel)}</span>}<ChevronDownIcon />
    </button>
    {position && createPortal(<div ref={popup} className="panel-icon-popover" style={position}>
      <div className="panel-icon-search"><input ref={searchInput} autoFocus aria-label="搜索国家/地区" placeholder="搜索国家/地区" value={search} onChange={event => setSearch(event.target.value)} /><button type="button" aria-label="清空图标搜索" onClick={() => { setSearch(""); searchInput.current?.focus(); }}><XMarkIcon /></button></div>
      <div role="tablist" aria-label="图标分类">{[{ id: "all", label: "全部" }, { id: "region", label: "地区" }, { id: "brand", label: "公司" }, { id: "other", label: "其他" }].map(tab => <button key={tab.id} type="button" role="tab" aria-selected={category === tab.id} onClick={() => setCategory(tab.id)}>{tab.label}</button>)}</div>
      <ul>{[...options.filter(icon => icon.category !== "region"), ...(showLabel ? [{ code: "", label: "无", category: "none", asset: "" }] : []), ...options.filter(icon => icon.category === "region")].map(icon => <li key={icon.code}><button ref={value === icon.code ? active : undefined} type="button" data-active={value === icon.code} onClick={() => { onChange(icon.code); setPosition(null); trigger.current?.focus(); }}>{icon.asset && <img src={icon.asset} alt="" />}<span>{icon.label}</span>{icon.category === "region" && <small>{icon.code}</small>}</button></li>)}{!options.length && <li className="panel-icon-empty">没有匹配的国家/地区</li>}</ul>
    </div>, document.querySelector(".openbox-app") ?? document.body)}
  </>;
}
