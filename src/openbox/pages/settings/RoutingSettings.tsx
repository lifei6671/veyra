import { ArrowUturnLeftIcon, Bars3Icon, PencilSquareIcon, PlusIcon, PowerIcon, TrashIcon } from "@heroicons/react/24/outline";
import { useEffect, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { api } from "../../api/client";
import type { GroupsResponse, OpenBoxProfile, RoutingPolicy } from "../../api/types";
import { ErrorState, IconButton, Modal, Tooltip } from "../../components/shared";
import { SortableList } from "../../components/SortableList";
import { SelectControl } from "../../ui/controls";
import { GroupIcon, ScaleControl } from "./GroupSettings";
import { PanelIconPicker } from "./PanelIconPicker";
import { OutboundPicker } from "../../components/OutboundPicker";
import { RuleImportModal, RuleValue } from "./RoutingPickers";
import { applyPolicyRules, importRuleRows, policyRuleRows, policySummary, RULE_TYPES, rulesValidation, sortPolicyRules, splitRuleRows, type RuleRow } from "./RoutingSettings.helpers";

type Editor = { kind: "policy" | "custom" | "fallback"; policy: RoutingPolicy; rows: RuleRow[] };
const FALLBACK_HINT = "上面都没命中的流量走它:有域名的访问只要域名条件都没命中就走它,不会拿解析出来的 IP 去对任何 IP 条件。在「代理」页选它走哪条线路。";
const CUSTOM_HINT = "一行一条,每行自己选出口（可直接选节点）。优先级最高,排在所有站点集之前,从上到下先命中先生效。IP 只管 IP、域名只管域名:IP 段 / geoip 行只管直接按 IP 发起的连接,按域名访问要写域名行。";

export function RoutingSettings({ onToast }: { onToast: (message: string) => void }) {
  const [profile, setProfile] = useState<OpenBoxProfile | null>(null), [policies, setPolicies] = useState<RoutingPolicy[]>([]);
  const [groups, setGroups] = useState<GroupsResponse | null>(null), [error, setError] = useState<unknown>(null), [busy, setBusy] = useState(false);
  const [editor, setEditor] = useState<Editor | null>(null), [deleting, setDeleting] = useState<RoutingPolicy | null>(null), [restoring, setRestoring] = useState(false), [importOpen, setImportOpen] = useState(false);
  const [actions, setActions] = useState<HTMLElement | null>(null);
  const load = async () => {
    setError(null);
    try { const response = await api.profile(); setProfile(response); setPolicies(response.routing.policies); }
    catch (reason) { setError(reason); }
  };
  useEffect(() => { setActions(document.getElementById("settings-header-actions")); void load(); }, []);
  const persist = async (routing: Partial<OpenBoxProfile["routing"]>) => {
    setBusy(true);
    try { const response = await api.saveProfile({ routing }); setProfile(response); setPolicies(response.routing.policies); onToast("已保存;重启内核后生效。"); return true; }
    catch (reason) { setPolicies(profile!.routing.policies); onToast(reason instanceof Error ? reason.message : "保存失败"); return false; }
    finally { setBusy(false); }
  };
  const openPolicy = (policy: RoutingPolicy | null) => {
    setEditor({ kind: "policy", policy: structuredClone(policy ?? { id: "", name: "", icon: "", iconScale: 0 }), rows: policyRuleRows(policy) }); setImportOpen(false);
  };
  const openCustom = () => {
    const custom = profile!.routing.custom;
    setEditor({ kind: "custom", policy: { name: custom?.name || "前置自定义分流", icon: custom?.icon || "misc:pin", iconScale: custom?.iconScale ?? 0 }, rows: custom?.rules?.length ? custom.rules.map((row, index) => ({ ...row, key: index + 1, note: row.note ?? "" })) : policyRuleRows(null) }); setImportOpen(false);
    if (!groups) void api.groups().then(setGroups).catch(reason => onToast(reason instanceof Error ? reason.message : "出口读取失败"));
  };
  const patchPolicy = (patch: Partial<RoutingPolicy>) => setEditor(current => current ? { ...current, policy: { ...current.policy, ...patch } } : current);
  const patchRows = (rows: RuleRow[]) => setEditor(current => current ? { ...current, rows } : current);
  const saveEditor = async () => {
    if (!editor || !profile || busy) return;
    const name = (editor.policy.name ?? "").trim();
    if (!name) { onToast("请填写站点集名称。"); return; }
    if (editor.kind !== "custom" && policies.some(policy => policy.name === name && (editor.kind === "fallback" || policy.id !== editor.policy.id))) { onToast("已有同名站点集;这个名字会直接用作内核里的出站名称,不能重复。"); return; }
    const rows = splitRuleRows(editor.rows); patchRows(rows);
    const validation = editor.kind === "fallback" ? "" : rulesValidation(rows, editor.kind === "custom");
    if (validation) { onToast(validation); return; }
    let routing: Partial<OpenBoxProfile["routing"]>;
    if (editor.kind === "fallback") routing = { fallbackName: name, fallbackIcon: editor.policy.icon, fallbackIconScale: editor.policy.iconScale ?? 0 };
    else if (editor.kind === "custom") routing = { custom: { name, icon: editor.policy.icon, iconScale: editor.policy.iconScale ?? 0, enabled: profile.routing.custom?.enabled !== false, rules: rows.filter(row => row.value.trim()).map(row => ({ type: row.type, value: row.value.trim(), outbound: row.outbound, ...(row.note.trim() ? { note: row.note.trim() } : {}) })) } };
    else {
      const policy = applyPolicyRules({ ...editor.policy, id: editor.policy.id || `policy-${Date.now()}`, name }, rows);
      routing = { policies: editor.policy.id ? policies.map(current => current.id === policy.id ? policy : current) : [...policies, policy] };
    }
    if (await persist(routing)) setEditor(null);
  };
  const card = (policy: RoutingPolicy, handle: ReactNode, kind?: "custom" | "fallback") => <article className={`routing-policy-card${kind ? " routing-pinned-card" : ""}${policy.enabled === false ? " routing-disabled" : ""}`}>
    {handle}{policy.icon ? <GroupIcon code={policy.icon} scale={policy.iconScale ?? 0} /> : <span className="group-icon" />}
    <div className="routing-policy-copy"><div className="routing-policy-title"><h3>{policy.name}</h3>{kind && <span>{kind === "custom" ? "前置" : "系统兜底"}</span>}{policy.enabled === false && <span className="routing-disabled-badge">已停用</span>}</div><p>{kind === "fallback" ? FALLBACK_HINT : kind === "custom" ? profile?.routing.custom?.rules?.length ? profile.routing.custom.rules.map(rule => `${rule.value} → ${rule.outbound}`).join(" · ") : "还没有规则（这条不生效）" : policySummary(policy)}</p></div>
    <div className="routing-policy-actions"><IconButton label={kind === "fallback" ? "兜底站点集不能停用" : policy.enabled === false ? "当前已停用,点击启用" : "当前已启用,点击停用"} tooltip portalTooltip className={`tooltip-trigger${policy.enabled === false ? "" : " is-enabled"}`} aria-disabled={kind === "fallback"} disabled={busy} onClick={() => { if (kind === "fallback") return; void persist(kind === "custom" ? { custom: { ...profile!.routing.custom, name: policy.name, icon: policy.icon, enabled: policy.enabled === false } } : { policies: policies.map(current => current.id === policy.id ? { ...current, enabled: policy.enabled === false } : current) }); }}><PowerIcon /></IconButton>
      <IconButton label="编辑" aria-label={kind === "fallback" ? "修改兜底站点集" : kind === "custom" ? "修改前置自定义分流" : "修改站点集"} tooltip portalTooltip className="tooltip-trigger" disabled={busy} onClick={() => { if (kind === "custom") openCustom(); else if (kind === "fallback") setEditor({ kind, policy: structuredClone(policy), rows: [] }); else openPolicy(policy); }}><PencilSquareIcon /></IconButton>
      <IconButton label={kind === "custom" ? "前置自定义分流是固定的一条,不能删除;不用时可以停用。" : kind === "fallback" ? "兜底站点集不能删除" : "删除"} aria-label={kind === "custom" ? "前置自定义分流是固定的一条,不能删除;不用时可以停用。" : kind === "fallback" ? "兜底站点集不能删除" : "删除站点集"} tooltip portalTooltip className="tooltip-trigger tooltip-align-right" aria-disabled={!!kind} disabled={busy} onClick={() => { if (!kind) setDeleting(policy); }}><TrashIcon /></IconButton></div>
  </article>;
  if (error) return <ErrorState title="目标分流加载失败" error={error} onRetry={() => void load()} />;
  if (!profile) return <div className="settings-loading"><span className="loading-spinner" /></div>;
  const custom = profile.routing.custom;
  const pinnedHandle = (hint: string) => <Tooltip label={hint} enabled={!editor && !deleting && !restoring}><span className="routing-pinned-handle"><Bars3Icon /></span></Tooltip>;
  return <>
    {actions && createPortal(<><IconButton label="恢复默认分流" tooltip portalTooltip className="tooltip-trigger" disabled={busy} onClick={() => setRestoring(true)}><ArrowUturnLeftIcon /></IconButton><IconButton label="添加站点集" tooltip portalTooltip className="tooltip-trigger add-button" disabled={busy} onClick={() => openPolicy(null)}><PlusIcon /></IconButton></>, actions)}
    <div className="routing-policy-stack" data-dialog-open={!!editor || !!deleting || restoring}>{!policies.length && <p className="routing-empty">还没有站点集。点击右上角添加。</p>}
      {card({ ...custom, name: custom?.name || "前置自定义分流", icon: custom?.icon || "misc:pin", enabled: custom?.enabled !== false }, pinnedHandle("固定位置,不能拖动排序"), "custom")}
      <SortableList items={policies} itemKey={policy => policy.id!} disabled={busy} onChange={setPolicies} onEnd={next => void persist({ policies: next })} renderItem={(policy, handle) => card(policy, handle)} />
      {card({ name: profile.routing.fallbackName || "其他", icon: profile.routing.fallbackIcon || "globe:earth-meridians", iconScale: profile.routing.fallbackIconScale ?? 0 }, pinnedHandle("固定位置,不能拖动排序"), "fallback")}
    </div>
    {editor && <Modal title={editor.kind === "fallback" ? "修改兜底站点集" : editor.kind === "custom" ? "修改前置自定义分流" : editor.policy.id ? "修改站点集" : "添加站点集"} className={`routing-dialog ${editor.kind === "fallback" ? "routing-fallback-modal" : "routing-edit-modal"}`} onClose={() => setEditor(null)} footer={<><button type="button" className="compact-button" onClick={() => setEditor(null)}>取消</button><button type="button" className="primary-button" disabled={busy} onClick={() => void saveEditor()}>{busy && <span className="loading-spinner" />}保存</button></>}>
      <div className="routing-editor-form"><div className="routing-metadata"><label className="routing-icon-field"><span>图标</span><PanelIconPicker label="站点集图标" value={editor.policy.icon ?? ""} onChange={icon => patchPolicy({ icon })} showLabel /></label><label className="routing-scale-field"><span>缩放</span><ScaleControl value={editor.policy.iconScale ?? 0} onChange={iconScale => patchPolicy({ iconScale })} /></label><label className="routing-name-field"><span>站点集名称</span><input aria-label="站点集名称" placeholder="比如:谷歌" value={editor.policy.name} onChange={event => patchPolicy({ name: event.target.value })} /></label></div>
        {editor.kind === "fallback" ? <p className="routing-hint">名字就是内核里的出站名,改名后代理页上原来选的线路要重新选一次;重启内核后生效。</p> : <div className="routing-rules">
          <div className="routing-rule-toolbar"><span>规则</span><div><button type="button" className="routing-small-button" onClick={() => setImportOpen(true)}>导入规则</button><button type="button" className="routing-small-button" onClick={() => patchRows([...editor.rows, { key: Math.max(0, ...editor.rows.map(row => row.key)) + 1, type: "domainSuffix", value: "", outbound: editor.rows.at(-1)?.outbound ?? "", note: "" }])}><PlusIcon />添加规则</button></div></div>
          {editor.kind === "custom" && <p className="routing-hint">{CUSTOM_HINT}</p>}
          <SortableList className="routing-rule-list" items={editor.rows} itemKey={row => row.key} onChange={patchRows} onEnd={rows => patchRows(editor.kind === "custom" ? rows : sortPolicyRules(rows))} renderItem={(row, handle) => <div className="routing-rule-row">{handle}
            <SelectControl className="routing-rule-type" contentClassName="routing-type-picker" label="规则类型" value={row.type} onValueChange={type => patchRows(editor.rows.map(current => current.key === row.key ? { ...current, type: type as RuleRow["type"] } : current))} options={RULE_TYPES.filter(type => (type.value !== "port" || editor.kind === "custom") && (type.value !== "ruleset" || row.type === "ruleset"))} />
            <RuleValue type={row.type} value={row.value} excluded={editor.rows.filter(current => current.key !== row.key && current.type === row.type).map(current => current.value)} onChange={value => patchRows(editor.rows.map(current => current.key === row.key ? { ...current, value } : current))} onBlur={() => setEditor(current => current ? { ...current, rows: splitRuleRows(current.rows) } : current)} onToast={onToast} />
            <input className="routing-rule-note" aria-label="备注" placeholder="备注" maxLength={60} value={row.note} onChange={event => patchRows(editor.rows.map(current => current.key === row.key ? { ...current, note: event.target.value } : current))} />
            {editor.kind === "custom" && <OutboundPicker groups={groups} value={row.outbound} testUrl={profile.testUrl} onChange={outbound => patchRows(editor.rows.map(current => current.key === row.key ? { ...current, outbound } : current))} onToast={onToast} />}
            <IconButton label="删除规则" tooltip portalTooltip className="tooltip-trigger routing-delete-rule" onClick={() => patchRows(editor.rows.filter(current => current.key !== row.key))}><TrashIcon /></IconButton>
          </div>} />{!editor.rows.length && <p className="routing-hint">还没有规则。至少加一条,否则这个站点集不会命中任何流量。</p>}
        </div>}
      </div>
    </Modal>}
    {editor && importOpen && <RuleImportModal onClose={() => setImportOpen(false)} onImport={entries => { try { patchRows(importRuleRows(editor.rows, entries)); setImportOpen(false); } catch (reason) { onToast(reason instanceof Error ? reason.message : "导入失败"); } }} />}
    {deleting && <Modal title="删除站点集" className="routing-dialog routing-confirm-modal" onClose={() => setDeleting(null)} footer={<><button type="button" className="compact-button" onClick={() => setDeleting(null)}>取消</button><button type="button" className="danger-button" disabled={busy} onClick={() => void persist({ policies: policies.filter(policy => policy.id !== deleting.id) }).then(success => { if (success) setDeleting(null); })}>删除</button></>}><p>确定删除站点集「{deleting.name}」?内核里那个同名选择器会一起消失,「代理」页上也就没有它了。</p></Modal>}
    {restoring && <Modal title="恢复默认分流" className="routing-dialog routing-confirm-modal" onClose={() => setRestoring(false)} footer={<><button type="button" className="compact-button" onClick={() => setRestoring(false)}>取消</button><button type="button" className="danger-button" disabled={busy} onClick={() => void (async () => { setBusy(true); try { const defaults = await api.defaultRouting(); if (await persist(defaults)) setRestoring(false); } catch (reason) { onToast(reason instanceof Error ? reason.message : "恢复失败"); } finally { setBusy(false); } })()}>恢复默认</button></>}><p>是否恢复默认目标分流规则?已设定的分流规则会清空,并恢复成随安装包自带的那一套。</p></Modal>}
  </>;
}
