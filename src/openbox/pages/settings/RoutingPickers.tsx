import { ArrowPathIcon, XMarkIcon } from "@heroicons/react/24/outline";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { api } from "../../api/client";
import type { RuleSetEntriesResponse, RuleSetEntry, RoutingRuleType } from "../../api/types";
import { IconButton, Modal } from "../../components/shared";
import { SelectControl } from "../../ui/controls";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger } from "../../ui/select";
import { usePickerSearch } from "../../components/usePickerSearch";
import { filterGeoCategories, geoDescription, RULE_TYPES, type GeoKind } from "./RoutingSettings.helpers";

const countCache = new Map<string, Promise<number>>();
const sourceKey = (source: { tag: string } | { url: string }) => "tag" in source ? source.tag : `url:${source.url}`;
function ruleCount(source: { tag: string } | { url: string }) {
  const key = sourceKey(source);
  if (!countCache.has(key)) countCache.set(key, api.ruleSetEntries(source, "", 0, 1).then(value => value.total).catch(error => { countCache.delete(key); throw error; }));
  return countCache.get(key)!;
}

export function GeoCategorySelect({ kind, value, excluded, onChange }: { kind: GeoKind; value: string; excluded: string[]; onChange: (value: string) => void }) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [scope, setScope] = useState("all");
  const [start, setStart] = useState(0);
  const heights = useRef(new Map<string, number>());
  const [measurement, setMeasurement] = useState(0);
  const picker = usePickerSearch(open);
  const rows = useMemo(() => filterGeoCategories(kind, search, excluded, value, scope).map(row => ({ code: row[0], description: geoDescription(kind, row[0]) })), [kind, search, excluded, value, scope]);
  const offsets = useMemo(() => {
    const result = [0];
    for (const row of rows) result.push(result[result.length - 1] + (heights.current.get(row.code) ?? (row.description ? 45 : 32)));
    return result;
  }, [rows, measurement]);
  const end = Math.min(rows.length, start + 60);
  useLayoutEffect(() => {
    if (!open) return;
    let changed = false;
    picker.popup.current?.querySelectorAll<HTMLElement>('[role="option"]').forEach(item => {
      const code = item.dataset.geoCode!, height = item.offsetHeight;
      if (height && heights.current.get(code) !== height) { heights.current.set(code, height); changed = true; }
    });
    if (changed) setMeasurement(current => current + 1);
  }, [open, rows, start]);
  const openPicker = (next: boolean) => {
    setOpen(next);
    if (next) { setSearch(""); setScope("all"); const index = filterGeoCategories(kind, "", excluded, value, "all").findIndex(row => row[0] === value); setStart(Math.max(0, index - 20)); }
  };
  return <Select open={open} value={value} onOpenChange={openPicker} onValueChange={next => { onChange(next); setOpen(false); }}>
    <SelectTrigger className="routing-geo-trigger" aria-label={`${kind} 分类`}><span>{value || "cn、google、netflix"}</span>{value && geoDescription(kind, value) && <small>{geoDescription(kind, value)}</small>}</SelectTrigger>
    {/* Keep Radix from remounting the catalog in its closed DocumentFragment. */}
    {open && <SelectContent ref={picker.popup} className="routing-picker routing-geo-picker" portalContainer={picker.container} align="start" onScrollCapture={event => {
      const viewport = event.target as HTMLElement;
      if (!viewport.hasAttribute("data-radix-select-viewport")) return;
      const index = Math.max(0, offsets.findIndex(offset => offset > viewport.scrollTop) - 1);
      setStart(Math.min(Math.max(0, rows.length - 60), Math.max(0, index - 20)));
    }} header={<div className="routing-picker-search-row"><div className="panel-icon-search"><input ref={picker.input} aria-label="搜索分类,支持中英文" placeholder="搜索分类,支持中英文" value={search} onChange={event => { setSearch(event.target.value); setStart(0); }} onKeyDown={picker.onKeyDown} /><button type="button" aria-label="清空分类搜索" onClick={() => { setSearch(""); setStart(0); picker.input.current?.focus(); }}><XMarkIcon /></button></div>{kind === "geoip" && <SelectControl contentClassName="routing-type-picker" label="IP 分类范围" value={scope} onValueChange={next => { setScope(next); setStart(0); }} options={[{ value: "all", label: "全部" }, { value: "region", label: "地区" }, { value: "other", label: "其他" }]} />}</div>}>
      <SelectGroup><div aria-hidden="true" style={{ height: offsets[start] ?? 0 }} />{rows.slice(start, end).map(row => <SelectItem key={row.code} value={row.code} data-geo-code={row.code}><span className="mono">{row.code}</span>{row.description && <small>{row.description}</small>}</SelectItem>)}<div aria-hidden="true" style={{ height: offsets[rows.length] - (offsets[end] ?? 0) }} />{!rows.length && <p className="routing-picker-empty">没有匹配的分类</p>}</SelectGroup>
    </SelectContent>}
  </Select>;
}

export function RuleValue({ type, value, excluded, onChange, onBlur, onToast }: { type: RoutingRuleType; value: string; excluded: string[]; onChange: (value: string) => void; onBlur: () => void; onToast: (value: string) => void }) {
  const [count, setCount] = useState<number | null>(null);
  const [status, setStatus] = useState("idle");
  const [details, setDetails] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const geo = type === "geosite" || type === "geoip";
  const url = type === "ruleUrl";
  const validUrl = /^https?:\/\/[^\s]+$/i.test(value.trim());
  const source = geo ? { tag: `${type}-${value}` } : { url: value.trim() };
  useEffect(() => {
    setCount(null); setStatus("idle");
    if (!value || !geo && (!url || !validUrl)) return;
    let active = true;
    setStatus("loading");
    const timer = setTimeout(() => ruleCount(geo ? { tag: `${type}-${value}` } : { url: value.trim() }).then(next => { if (active) { setCount(next); setStatus("idle"); } }).catch(() => { if (active) setStatus("failed"); }), url ? 600 : 0);
    return () => { active = false; clearTimeout(timer); };
  }, [type, value]);
  const refresh = async () => {
    setRefreshing(true);
    try { const response = await api.refreshRuleSet(value.trim()); countCache.set(sourceKey(source), Promise.resolve(response.total)); setCount(response.total); setStatus("idle"); onToast(`名单已更新(${response.total.toLocaleString()} 条),${response.needsRestart ? "重启内核后生效。" : "内核会自动加载新内容。"}`); }
    catch (error) { onToast(error instanceof Error ? error.message : "更新规则集失败"); }
    finally { setRefreshing(false); }
  };
  return <div className="routing-rule-value">
    {geo ? <GeoCategorySelect kind={type} value={value} excluded={excluded} onChange={onChange} /> : <input className="mono" aria-label="规则值" type={url ? "url" : "text"} value={value} placeholder={RULE_TYPES.find(option => option.value === type)!.placeholder} onChange={event => onChange(event.target.value)} onBlur={onBlur} />}
    {geo && value || url && count !== null ? <button type="button" className="routing-small-button" onClick={() => setDetails(true)}>详情{count !== null && ` (${count.toLocaleString()})`}</button> : url && status === "loading" ? <span className="loading-spinner" /> : url && status === "failed" ? <span className="routing-read-error" title="这个网址拉不下来,或者里面一条域名 / IP 都没解析出来。保存后部署时会再试一次。">读取失败</span> : null}
    {url && validUrl && status !== "loading" && <IconButton label="立即更新:重新拉取这份名单并编译。名单平时每 24 小时随内核启动更新一次" tooltip portalTooltip className="tooltip-trigger" disabled={refreshing} onClick={() => void refresh()}>{refreshing ? <span className="loading-spinner" /> : <ArrowPathIcon />}</IconButton>}
    {details && <RuleEntriesModal source={source} onClose={() => setDetails(false)} />}
  </div>;
}

export function RuleEntriesModal({ source, onClose }: { source: { tag: string } | { url: string }; onClose: () => void }) {
  const [search, setSearch] = useState("");
  const [data, setData] = useState<RuleSetEntriesResponse>({ entries: [], total: 0, matched: 0 });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const revision = useRef(0);
  const title = "tag" in source ? source.tag : source.url;
  const geo = "tag" in source ? /^(geosite|geoip)-(.+)$/.exec(source.tag) : null;
  const load = async (offset: number) => {
    const current = ++revision.current;
    setBusy(true); setError("");
    try { const response = await api.ruleSetEntries(source, search.trim(), offset); if (current === revision.current) setData(previous => ({ ...response, entries: offset ? [...previous.entries, ...response.entries] : response.entries })); }
    catch (reason) { if (current === revision.current) { setError(reason instanceof Error ? reason.message : "规则详情加载失败"); setData({ entries: [], total: 0, matched: 0 }); } }
    finally { if (current === revision.current) setBusy(false); }
  };
  useEffect(() => { void load(0); return () => { revision.current++; }; }, [search, title]);
  return <Modal title={title} className="routing-dialog routing-entries-modal" onClose={onClose}><div className="routing-entries-form">
    {geo && geoDescription(geo[1] as GeoKind, geo[2]) && <p className="routing-geo-description">{geoDescription(geo[1] as GeoKind, geo[2])}</p>}
    <div className="routing-entries-search"><input aria-label="在这个分类里搜索域名或 IP" placeholder="在这个分类里搜索域名或 IP" value={search} onChange={event => setSearch(event.target.value)} /><button type="button" aria-label="清空详情搜索" onClick={() => setSearch("")}><XMarkIcon /></button></div>
    <p>{search ? `匹配 ${data.matched.toLocaleString()} 条,共 ${data.total.toLocaleString()} 条` : `共 ${data.total.toLocaleString()} 条`}</p>
    {error && <p role="alert" className="routing-read-error">{error}</p>}
    <div className="routing-entries-list" aria-busy={busy}>{busy && !data.entries.length ? <div className="routing-entries-empty"><span className="loading-spinner" /></div> : <EntriesList entries={data.entries} />}</div>
    {data.entries.length < data.matched && <button type="button" className="routing-load-more" disabled={busy} onClick={() => void load(data.entries.length)}>{busy && <span className="loading-spinner" />}加载更多</button>}
  </div></Modal>;
}

export function EntriesList({ entries }: { entries: RuleSetEntry[] }) {
  const labels: Record<string, string> = { domain: "域名", domain_suffix: "域名后缀", domain_keyword: "域名关键词", ip_cidr: "IP 段", domain_regex: "域名正则" };
  return entries.length ? <ul>{entries.map((entry, index) => <li key={`${entry.type}:${entry.value}:${index}`}><span>{labels[entry.type] ?? entry.type}</span><code title={entry.value}>{entry.value}</code></li>)}</ul> : <p className="routing-entries-empty">没有匹配的条目</p>;
}

export function RuleImportModal({ onClose, onImport }: { onClose: () => void; onImport: (entries: RuleSetEntry[]) => void }) {
  const [url, setUrl] = useState("");
  const [entries, setEntries] = useState<RuleSetEntry[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const preview = async () => {
    if (!url.trim() || busy) return;
    setBusy(true); setError("");
    try { const response = await api.importRuleSet(url.trim()); setEntries(response.entries.filter(entry => ["domain", "domain_suffix", "domain_keyword", "ip_cidr"].includes(entry.type) && entry.value.trim())); }
    catch (reason) { setEntries(null); setError(reason instanceof Error ? reason.message : "预览失败"); }
    finally { setBusy(false); }
  };
  return <Modal title="导入规则" className="routing-dialog routing-import-modal" onClose={onClose} footer={<><button type="button" className="compact-button" onClick={onClose}>取消</button>{entries && <button type="button" className="primary-button" disabled={!entries.length || entries.length > 200} onClick={() => onImport(entries)}>导入明细</button>}</>}><div className="routing-entries-form">
    <p>输入规则集地址（支持 .list / .mrs / .srs），预览确认后会把域名和 IP 明细追加到当前站点集，原有规则集链接会保留。只支持导入 200 条以内；超过 200 条请直接用「规则集链接」引用这个地址。</p>
    <div className="routing-import-input"><input className="mono" aria-label="规则集地址" placeholder={RULE_TYPES.find(type => type.value === "ruleUrl")!.placeholder} value={url} onChange={event => setUrl(event.target.value)} onKeyDown={event => { if (event.key === "Enter") { event.preventDefault(); void preview(); } }} /><button type="button" className="primary-button" disabled={busy || !url.trim()} onClick={() => void preview()}>{busy && <span className="loading-spinner" />}预览</button></div>
    {error && <p role="alert" className="routing-read-error">{error}</p>}
    {entries && <><p className={entries.length > 200 ? "routing-warning" : ""}>{entries.length > 200 ? `共 ${entries.length} 条，超过 200 条不能导入明细——请改用「规则集链接」直接引用这个地址。` : `共 ${entries.length} 条可导入规则`}</p><div className="routing-entries-list import-list"><EntriesList entries={entries} /></div></>}
  </div></Modal>;
}
