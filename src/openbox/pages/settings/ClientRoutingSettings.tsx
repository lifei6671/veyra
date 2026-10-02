import { CheckIcon, DevicePhoneMobileIcon, PencilSquareIcon, PlusIcon, PowerIcon, TrashIcon, XMarkIcon } from "@heroicons/react/24/outline";
import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { api } from "../../api/client";
import type { ClientRoutingRule, GroupsResponse, KnownClient, OpenBoxProfile } from "../../api/types";
import { ErrorState, IconButton, Modal } from "../../components/shared";
import { SortableList } from "../../components/SortableList";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger } from "../../ui/select";
import { OutboundPicker } from "../../components/OutboundPicker";
import { usePickerSearch } from "../../components/usePickerSearch";
import { addKnownClient, admittedClientCount, clientRouteDraft, clientRouteSummary, knownClientSelected, saveClientRoute, type ClientRouteDraft } from "./ClientRoutingSettings.helpers";

const MATCH_HINTS = {
  ip: "按 IP / 网段认终端。「不进内核」「白名单」按 IP 时靠防火墙打标记实现:这些终端(白名单时是名单外的终端)不走 mwan3 多 WAN 分流;终端有 IPv6 的,v6 地址要另外列。",
  mac: "按 MAC 认终端,IP 变了也认得出;「不进内核」「白名单」按 MAC 由内核原生处理,不影响多 WAN。",
};
const MODES = [
  { value: "route", label: "进内核,走指定的出站", hint: "照常进内核,只是出口固定成下面选的这个。" },
  { value: "bypass", label: "不进内核（直连放行）", hint: "像 OpenClash 的黑名单:这些终端的流量在入口就放行,完全不经过内核,NAT 和没装时一样——游戏加速器、对 NAT 类型敏感的设备用它。需要 auto_redirect;退到纯 tun 兼容模式时按「直连」出站处理。" },
  { value: "admit", label: "只让这些终端进内核（白名单）", hint: "像 OpenClash 的白名单:有这类规则时,局域网里只有列出来的终端进内核,其余终端和没装 Open-Box 一样(DNS 也不经内核);路由器自己不受影响。需要 auto_redirect;纯 tun 兼容模式下不生效。" },
] as const;

function KnownClientPicker({ clients, draft, onChange }: { clients: KnownClient[]; draft: ClientRouteDraft; onChange: (draft: ClientRouteDraft) => void }) {
  const [open, setOpen] = useState(false), [search, setSearch] = useState("");
  const picker = usePickerSearch(open);
  const query = search.trim().toLowerCase();
  const matches = clients.filter(client => draft.match === "ip" || client.mac).filter(client => !query || [client.ip, client.name, client.mac].some(value => value?.toLowerCase().includes(query)));
  return <Select open={open} onOpenChange={next => { setOpen(next); if (next) setSearch(""); }}>
    <SelectTrigger aria-label="从已知终端添加" className="terminal-known-trigger"><span>从 DHCP 租约 / 近期流量里选</span></SelectTrigger>
    {open && <SelectContent ref={picker.popup} className="routing-picker terminal-known-picker" portalContainer={picker.container} align="start" header={<div className="panel-icon-search"><input ref={picker.input} aria-label="搜索已知终端" placeholder="搜索" value={search} onChange={event => setSearch(event.target.value)} onKeyDown={picker.onKeyDown} /><button type="button" aria-label="清空终端搜索" onClick={() => { setSearch(""); picker.input.current?.focus(); }}><XMarkIcon /></button></div>}>
      <SelectGroup>{matches.map(client => <SelectItem key={client.ip} value={client.ip} onPointerUp={event => event.preventDefault()} onClick={event => { event.preventDefault(); onChange(addKnownClient(draft, client)); }} onKeyDown={event => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onChange(addKnownClient(draft, client)); } }}>
        <CheckIcon className={knownClientSelected(draft, client) ? "is-selected" : "is-unselected"} /><span className="terminal-known-ip">{client.ip}</span><span className="terminal-known-name">{client.name}</span>{client.mac && <small>{client.mac}</small>}
      </SelectItem>)}{!matches.length && <p className="routing-picker-empty">没有匹配的条目</p>}</SelectGroup>
    </SelectContent>}
  </Select>;
}

export function ClientRoutingSettings({ onToast }: { onToast: (message: string) => void }) {
  const [profile, setProfile] = useState<OpenBoxProfile | null>(null), [groups, setGroups] = useState<GroupsResponse | null>(null);
  const [clients, setClients] = useState<KnownClient[]>([]), [rules, setRules] = useState<ClientRoutingRule[]>([]);
  const [error, setError] = useState<unknown>(null), [busy, setBusy] = useState(false), [actions, setActions] = useState<HTMLElement | null>(null);
  const [editor, setEditor] = useState<{ existing: boolean; draft: ClientRouteDraft } | null>(null), [deleting, setDeleting] = useState<ClientRoutingRule | null>(null);
  const load = async () => {
    setError(null);
    try { const [next, exits] = await Promise.all([api.profile(), api.groups()]); setProfile(next); setRules(next.clientRoutes); setGroups(exits); }
    catch (reason) { setError(reason); }
    try { setClients((await api.clients()).clients ?? []); } catch { setClients([]); }
  };
  useEffect(() => { setActions(document.getElementById("settings-header-actions")); void load(); }, []);
  useEffect(() => {
    if (!editor && !deleting) return;
    const escape = (event: KeyboardEvent) => { if (event.key === "Escape" && !event.defaultPrevented && !(event.target as Element).closest('[data-slot="select-content"]') && !document.querySelector('[data-slot="select-content"]')) { setEditor(null); setDeleting(null); } };
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  }, [!!editor, !!deleting]);
  const persist = async (next: ClientRoutingRule[]) => {
    setBusy(true);
    try { const response = await api.saveProfile({ clientRoutes: next }); setProfile(response); setRules(response.clientRoutes); onToast("已保存;重启内核后生效。"); return true; }
    catch (reason) { setRules(profile!.clientRoutes); onToast(reason instanceof Error ? reason.message : "保存失败"); return false; }
    finally { setBusy(false); }
  };
  const add = () => setEditor({ existing: false, draft: clientRouteDraft() });
  const patch = (next: Partial<ClientRouteDraft>) => setEditor(current => current ? { ...current, draft: { ...current.draft, ...next } } : current);
  const save = async () => {
    if (!editor || busy) return;
    let rule: ClientRoutingRule;
    try { rule = saveClientRoute(editor.draft); } catch (reason) { onToast((reason as Error).message); return; }
    const next = editor.existing ? rules.map(current => current.id === rule.id ? rule : current) : [...rules, rule];
    if (await persist(next)) setEditor(null);
  };
  const available = new Set((groups?.groups ?? []).filter(group => group.enabled !== false).map(group => group.name).concat((groups?.availableNodes ?? []).map(node => node.name)));
  if (error) return <ErrorState title="终端分流加载失败" error={error} onRetry={() => void load()} />;
  if (!profile) return <div className="settings-loading"><span className="loading-spinner" /></div>;
  const count = admittedClientCount(rules);
  const draft = editor?.draft;
  return <>
    {actions && createPortal(<IconButton label="添加终端规则" tooltip portalTooltip className="add-button terminal-add-button" disabled={busy} onClick={add}><PlusIcon /></IconButton>, actions)}
    <div className="routing-policy-stack terminal-routing" data-dialog-open={!!editor || !!deleting}>
      {!rules.length && <div className="terminal-routing-empty"><DevicePhoneMobileIcon /><p>还没有终端规则。</p><button type="button" className="primary-button" onClick={add}><PlusIcon />添加终端规则</button></div>}
      {!!count && <p className="terminal-admit-alert" role="status">白名单已生效:局域网里只有列出的 {count} 个终端 / 网段进内核,其余终端不经过 Open-Box。</p>}
      <SortableList items={rules} itemKey={rule => rule.id} disabled={busy} onChange={setRules} onEnd={next => void persist(next)} renderItem={(rule, handle) => <article className={`routing-policy-card terminal-routing-card${rule.enabled === false ? " routing-disabled" : ""}`}>
        {handle}<div className="routing-policy-copy"><div className="routing-policy-title"><h3>{rule.name}</h3><span className="terminal-outbound-badge">{rule.admit ? "只让这些进内核" : rule.bypass ? "不进内核" : rule.outbound}</span>{rule.enabled === false && <span className="routing-disabled-badge">已停用</span>}{!rule.bypass && !rule.admit && !available.has(rule.outbound ?? "") && <small className="terminal-outbound-missing">出站「{rule.outbound}」已不存在</small>}</div><p>{clientRouteSummary(rule, clients)}</p></div>
        <div className="routing-policy-actions"><IconButton label={rule.enabled === false ? "当前已停用,点击启用" : "当前已启用,点击停用"} tooltip portalTooltip className={rule.enabled === false ? "" : "is-enabled"} disabled={busy} onClick={() => void persist(rules.map(current => current.id === rule.id ? { ...current, enabled: rule.enabled === false } : current))}><PowerIcon /></IconButton><IconButton label="编辑" aria-label={`编辑终端规则 ${rule.name}`} tooltip portalTooltip disabled={busy} onClick={() => setEditor({ existing: true, draft: clientRouteDraft(rule) })}><PencilSquareIcon /></IconButton><IconButton label="删除" aria-label={`删除终端规则 ${rule.name}`} tooltip portalTooltip disabled={busy} onClick={() => setDeleting(rule)}><TrashIcon /></IconButton></div>
      </article>} />
    </div>
    {editor && draft && <Modal title={editor.existing ? "编辑终端规则" : "添加终端规则"} className="routing-dialog terminal-route-dialog" onClose={() => setEditor(null)} footer={<><button type="button" className="compact-button" onClick={() => setEditor(null)}>取消</button><button type="button" className="primary-button" disabled={busy} onClick={() => void save()}>{busy && <span className="loading-spinner" />}保存</button></>}>
      <div className="terminal-route-form">
        <label className="terminal-route-field"><span>名称</span><input aria-label="名称" placeholder="例如:客厅电视" value={draft.name} onChange={event => patch({ name: event.target.value })} /></label>
        <div className="terminal-route-field"><div className="terminal-match-header"><label htmlFor="terminal-match-input">终端</label><div role="tablist" aria-label="终端匹配方式">{(["ip", "mac"] as const).map(match => <button type="button" key={match} role="tab" aria-selected={draft.match === match} onClick={() => patch({ match })}>{match === "ip" ? "按 IP" : "按 MAC"}</button>)}</div></div>
          <textarea id="terminal-match-input" aria-label={draft.match === "ip" ? "终端 IP 或网段" : "终端 MAC 地址"} className="mono" rows={4} placeholder={draft.match === "ip" ? "每行一个 IP 或网段,如 10.0.0.209 或 10.0.0.0/24" : "每行一个,如 aa:bb:cc:dd:ee:ff;从已知终端添加会自动带上"} value={draft.match === "ip" ? draft.sourcesText : draft.macsText} onChange={event => patch(draft.match === "ip" ? { sourcesText: event.target.value } : { macsText: event.target.value })} />
          <p className="terminal-hint">{MATCH_HINTS[draft.match]}</p><div className="terminal-known-row"><span>从已知终端添加</span><KnownClientPicker clients={clients} draft={draft} onChange={next => setEditor({ ...editor, draft: next })} /></div>
        </div>
        <div className="terminal-mode-field"><span>这些终端</span>{MODES.map(mode => <label key={mode.value}><input type="radio" name="terminal-route-mode" value={mode.value} checked={draft.mode === mode.value} onChange={() => patch({ mode: mode.value })} /><span>{mode.label}<small>{mode.hint}</small></span></label>)}</div>
        {draft.mode === "route" && <div className="terminal-route-field terminal-outbound-field"><span>出站</span><OutboundPicker groups={groups} value={draft.outbound} testUrl={profile.testUrl} onChange={outbound => patch({ outbound })} onToast={onToast} /><p className="terminal-hint">这些终端的全部流量都走这里选的出站,优先级高于站点集;局域网目标仍直连。保存后重启内核生效。</p></div>}
      </div>
    </Modal>}
    {deleting && <Modal title="确认" className="routing-dialog routing-confirm-modal" onClose={() => setDeleting(null)} footer={<><button type="button" className="compact-button" onClick={() => setDeleting(null)}>取消</button><button type="button" className="danger-button" disabled={busy} onClick={() => void persist(rules.filter(rule => rule.id !== deleting.id)).then(success => { if (success) setDeleting(null); })}>确定</button></>}><p>确定删除终端规则「{deleting.name}」吗？</p></Modal>}
  </>;
}
