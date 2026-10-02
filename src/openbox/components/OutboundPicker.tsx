import { BoltIcon, MinusIcon, XMarkIcon } from "@heroicons/react/24/outline";
import { useEffect, useRef, useState } from "react";
import { api } from "../api/client";
import type { GroupsResponse, ProxiesResponse } from "../api/types";
import { latestLatency } from "../lib/format";
import { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectTrigger } from "../ui/select";
import { IconButton } from "./shared";
import { resolveOutboundProxy } from "./OutboundPicker.helpers";
import { usePickerSearch } from "./usePickerSearch";

function OutboundLatency({ delay, testing }: { delay: number | null; testing: boolean }) {
  const [display, setDisplay] = useState(delay);
  const displayed = useRef(delay);
  useEffect(() => {
    if (delay === displayed.current) return;
    if (!delay || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      displayed.current = delay; setDisplay(delay); return;
    }
    const from = displayed.current ?? 0;
    let frame = 0, start: number | undefined;
    const animate = (time: number) => {
      start ??= time;
      const progress = Math.min(1, (time - start) / 1000);
      const next = Math.round(from + (delay - from) * (1 - 2 ** (-10 * progress)) / (1 - 2 ** -10));
      displayed.current = next; setDisplay(next);
      if (progress < 1) frame = requestAnimationFrame(animate);
    };
    frame = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(frame);
  }, [delay]);
  return <small className="routing-outbound-latency" aria-label={testing ? "测速中" : delay ? `${delay}ms` : "暂无延迟"} aria-busy={testing}>
    {testing ? <svg className="routing-latency-dots" viewBox="0 0 24 24" aria-hidden="true">{[4, 12, 20].map((x, index) => <circle key={x} cx={x} cy="12" r="3"><animate attributeName="cy" values="12;6;12;12" keyTimes="0;0.286;0.571;1" dur="1.05s" repeatCount="indefinite" begin={`${index * .1}s`} /></circle>)}</svg> : delay ? display : <MinusIcon aria-hidden="true" />}
  </small>;
}

export function OutboundPicker({ groups, value, testUrl, onChange, onToast }: { groups: GroupsResponse | null; value: string; testUrl: string; onChange: (value: string) => void; onToast: (value: string) => void }) {
  const [open, setOpen] = useState(false), [tab, setTab] = useState("nodes"), [search, setSearch] = useState("");
  const [proxies, setProxies] = useState<ProxiesResponse | null>(null);
  const [testing, setTesting] = useState<string[]>([]);
  const [testingAll, setTestingAll] = useState(false);
  const picker = usePickerSearch(open);
  const builtin = (groups?.groups ?? []).filter(group => group.kind && group.enabled !== false).map(group => group.name);
  const nodeGroups = (groups?.groups ?? []).filter(group => !group.kind && group.enabled !== false).map(group => group.name);
  const nodes = groups?.availableNodes ?? [];
  const subscriptions = [...new Set(nodes.map(node => node.subscription))];
  const matches = (name: string) => name.toLowerCase().includes(search.trim().toLowerCase());
  const visible = tab === "groups" ? nodeGroups.filter(matches) : [...builtin.filter(matches), ...nodes.filter(node => matches(node.name) || matches(node.subscription)).map(node => node.name)];
  const test = async (name: string) => {
    const proxy = resolveOutboundProxy(name, proxies);
    if (!proxy || ["block", "reject"].includes(proxy.type.toLowerCase())) return;
    const target = proxy.name;
    setTesting(current => [...current, name]);
    try { const { delay } = await api.testProxy(target, testUrl, 5000); setProxies(current => current ? { ...current, proxies: { ...current.proxies, [target]: { ...proxy, history: [...proxy.history ?? [], { time: new Date().toISOString(), delay }] } } } : current); }
    catch (error) { onToast(error instanceof Error ? error.message : "测速失败"); }
    finally { setTesting(current => current.filter(item => item !== name)); }
  };
  const option = (name: string) => {
    const proxy = resolveOutboundProxy(name, proxies);
    const delay = latestLatency(proxy?.history);
    const testable = proxy && !["block", "reject"].includes(proxy.type.toLowerCase());
    return <SelectItem key={name} value={name}><span>{name}</span>{testable && <><OutboundLatency delay={delay} testing={testing.includes(name)} /><IconButton label={`测试${name}`} tooltip portalTooltip className="tooltip-trigger" disabled={testing.includes(name)} onPointerUp={event => event.stopPropagation()} onClick={event => { event.preventDefault(); event.stopPropagation(); void test(name); }}><BoltIcon /></IconButton></>}</SelectItem>;
  };
  return <Select value={value} open={open} onOpenChange={next => { setOpen(next); if (next) { setSearch(""); setTab(nodeGroups.includes(value) ? "groups" : "nodes"); void api.proxies().then(setProxies).catch(reason => onToast(reason instanceof Error ? reason.message : "出站状态读取失败")); } }} onValueChange={onChange}>
    <SelectTrigger className="routing-outbound-trigger" aria-label="选择出站"><span className={!value ? "routing-outbound-placeholder" : undefined}>{value || "选择出站"}</span></SelectTrigger>
    <SelectContent ref={picker.popup} className="routing-picker routing-outbound-picker" portalContainer={picker.container} align="start" header={<div className="routing-outbound-header"><div role="tablist"><button type="button" role="tab" aria-selected={tab === "nodes"} onClick={() => setTab("nodes")}>节点 ({builtin.length + nodes.length})</button><button type="button" role="tab" aria-selected={tab === "groups"} onClick={() => setTab("groups")}>节点组 ({nodeGroups.length})</button></div><div className="panel-icon-search"><input ref={picker.input} aria-label="搜索出站" placeholder="搜索" value={search} onChange={event => setSearch(event.target.value)} onKeyDown={picker.onKeyDown} /><button type="button" aria-label="清空出站搜索" onClick={() => { setSearch(""); picker.input.current?.focus(); }}><XMarkIcon /></button></div><IconButton label="测试所有" tooltip portalTooltip className="tooltip-trigger" disabled={testingAll || testing.length > 0} aria-busy={testingAll} onClick={() => void (async () => { setTestingAll(true); try { for (let index = 0; index < visible.length; index += 5) await Promise.all(visible.slice(index, index + 5).map(test)); } finally { setTestingAll(false); } })()}>{testingAll ? <span className="loading-spinner" /> : <BoltIcon />}</IconButton></div>}>
      {tab === "groups" ? <SelectGroup>{nodeGroups.filter(matches).map(option)}</SelectGroup> : <><SelectGroup>{builtin.some(matches) && <SelectLabel>内置</SelectLabel>}{builtin.filter(matches).map(option)}</SelectGroup>{subscriptions.map(subscription => { const selected = nodes.filter(node => node.subscription === subscription && (matches(node.name) || matches(subscription))); return selected.length ? <SelectGroup key={subscription}><SelectLabel>{subscription || "无订阅"}</SelectLabel>{selected.map(node => option(node.name))}</SelectGroup> : null; })}</>}{!visible.length && <p className="routing-picker-empty">没有匹配的条目</p>}
    </SelectContent>
  </Select>;
}
