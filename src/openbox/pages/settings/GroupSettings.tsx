import {
  ArrowLeftIcon,
  ArrowRightIcon,
  ArrowUturnLeftIcon,
  Bars3Icon,
  ChevronDownIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  MagnifyingGlassIcon,
  GlobeAltIcon,
  MinusIcon,
  PencilSquareIcon,
  PlusIcon,
  PowerIcon,
  SparklesIcon,
  TrashIcon,
  XMarkIcon,
} from "@heroicons/react/24/outline";
import { useEffect, useId, useLayoutEffect, useRef, useState, type Dispatch, type PointerEvent as ReactPointerEvent, type ReactNode, type SetStateAction } from "react";
import { createPortal } from "react-dom";
import { api } from "../../api/client";
import type { GroupsResponse, OpenBoxGroup, OpenBoxGroupLane } from "../../api/types";
import { EmptyState, ErrorState, IconButton, Modal } from "../../components/shared";
import { SelectControl, SwitchControl } from "../../ui/controls";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger } from "../../ui/select";
import { PanelIconPicker, panelIcons } from "./PanelIconPicker";
import {
  buildAutoGroups,
  availableAutoCountries,
  cloneGroup,
  commaValues,
  countryNodeCount,
  COUNTRY_OPTIONS,
  DEFAULT_AUTO_COUNTRIES,
  dynamicGroupMembers,
  errorMessage,
  filterMembers,
  groupSummary,
  GROUP_TYPE_LABELS,
  mergeAutoGroups,
  memberFilterOptions,
  moveItem,
  normalizeEditorGroup,
  secondsValue,
  type GroupType,
  type MemberOption,
} from "./GroupSettings.helpers";

export function GroupSettings({ onToast }: { onToast: (message: string) => void }) {
  const [data, setData] = useState<GroupsResponse | null>(null);
  const [groups, setGroups] = useState<OpenBoxGroup[]>([]);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [busyGroupId, setBusyGroupId] = useState<string | null>(null);
  const [editor, setEditor] = useState<OpenBoxGroup | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<OpenBoxGroup | null>(null);
  const [restoreOpen, setRestoreOpen] = useState(false);
  const [autoOpen, setAutoOpen] = useState(false);
  const [actionsTarget, setActionsTarget] = useState<HTMLElement | null>(null);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const draggingIndex = useRef<number | null>(null);
  const dragGroups = useRef<OpenBoxGroup[] | null>(null);
  const groupElements = useRef(new Map<string, HTMLElement>());
  const groupRects = useRef(new Map<string, DOMRect>());

  const load = async () => {
    setError(null);
    try {
      const value = await api.groups();
      setData(value);
      setGroups(value.groups);
    } catch (reason) {
      setError(reason);
    }
  };

  useEffect(() => {
    setActionsTarget(document.getElementById("settings-header-actions"));
    void load();
  }, []);

  const persist = async (next: OpenBoxGroup[], message?: string) => {
    setBusy(true);
    try {
      const value = await api.saveGroups(next);
      setData(current => current ? { ...current, groups: value.groups } : current);
      setGroups(value.groups);
      if (message) onToast(message);
      const empty = (value.dropped ?? []).filter(group => group.reason !== "cycle").map(group => group.name);
      const cyclic = (value.dropped ?? []).filter(group => group.reason === "cycle").map(group => group.name);
      if (empty.length) onToast(`这些分组没有可用成员,不会写进配置:${empty.join("、")}`);
      if (cyclic.length) onToast(`这些分组互相引用,不会写进配置:${cyclic.join("、")}`);
      for (const group of value.dangling ?? []) onToast(`分组「${group.name}」引用的「${group.members.join("、")}」既不是节点也不是分组,写进配置时会被忽略`);
      return true;
    } catch (reason) {
      onToast(errorMessage(reason, "保存失败"));
      await load();
      return false;
    } finally {
      setBusy(false);
      setBusyGroupId(null);
    }
  };

  const openAdd = () => {
    setEditingId(null);
    setEditor({
      id: "",
      name: "",
      type: "urltest",
      mode: "static",
      enabled: true,
      icon: "",
      iconScale: 0,
      keywords: [],
      members: [],
      interval: "300s",
      tolerance: 100,
      testUrl: "",
    });
  };

  const openEdit = (group: OpenBoxGroup) => {
    setEditingId(group.id);
    setEditor(cloneGroup(group));
  };

  const saveEditor = async () => {
    if (!editor) return;
    const normalized = normalizeEditorGroup(editor);
    if (!normalized.name) return onToast("请输入分组名称");
    if (groups.some(group => group.name === normalized.name && group.id !== editingId)) return onToast("分组名称不能重复");
    if (normalized.type === "failover") {
      const available = new Set(data?.availableNodes.map(node => node.name) ?? []);
      const validLanes = (normalized.lanes ?? []).filter(lane => lane.members.some(member => available.has(member))).length;
      if (validLanes < (editingId ? 1 : 2)) return onToast(editingId ? "至少保留一个含有效节点的页签" : "至少需要两个各含一个有效节点的页签");
    }
    if (!normalized.kind && normalized.type !== "failover" && normalized.mode === "static" && !normalized.members.length) return onToast("静态组至少需要一个成员");
    const next = editingId
      ? groups.map(group => group.id === editingId ? normalized : group)
      : [...groups, { ...normalized, id: `g-${Date.now()}` }];
    if (await persist(next, editingId ? "分组已保存" : "分组已添加")) setEditor(null);
  };

  const toggleGroup = async (group: OpenBoxGroup) => {
    setBusyGroupId(group.id);
    await persist(groups.map(item => item.id === group.id ? { ...item, enabled: item.enabled === false } : item));
  };

  const finishDrag = () => {
    const next = dragGroups.current;
    draggingIndex.current = null;
    dragGroups.current = null;
    setDraggingId(null);
    if (next) void persist(next, "分组顺序已保存");
  };

  return <>
    {actionsTarget && createPortal(<>
      <IconButton label="恢复默认" tooltip className="settings-group-action tooltip-trigger" onClick={() => setRestoreOpen(true)}><ArrowUturnLeftIcon /></IconButton>
      <IconButton label="自动分组" tooltip className="settings-group-action tooltip-trigger" onClick={() => setAutoOpen(true)}><SparklesIcon /></IconButton>
      <IconButton label="添加分组" tooltip className="settings-group-action primary tooltip-trigger tooltip-align-right" onClick={openAdd}><PlusIcon /></IconButton>
    </>, actionsTarget)}

    {Boolean(error) && <ErrorState title="出站节点加载失败" error={error} onRetry={() => void load()} />}
    {!data && !error && <div className="group-loading" aria-label="正在加载出站节点"><span className="loading-spinner" /></div>}
    {data && <div className="group-card-list" data-dialog-open={Boolean(editor || deleting || restoreOpen || autoOpen)}>{groups.map((group, index) => <article
      className={`group-card${group.enabled === false ? " disabled" : ""}${draggingId === group.id ? " is-drag-placeholder" : ""}`}
      key={group.id}
      ref={element => { if (element) groupElements.current.set(group.id, element); else groupElements.current.delete(group.id); }}
      draggable={!busy}
      onDragStart={event => {
        event.dataTransfer.effectAllowed = "move";
        draggingIndex.current = index;
        dragGroups.current = groups;
        groupElements.current.forEach((element, key) => groupRects.current.set(key, element.getBoundingClientRect()));
        window.requestAnimationFrame(() => setDraggingId(group.id));
      }}
      onDragOver={event => {
        event.preventDefault();
        const from = draggingIndex.current;
        if (from === null || from === index) return;
        const rect = event.currentTarget.getBoundingClientRect();
        const trigger = rect.top + rect.height * (from < index ? .25 : .75);
        if ((from < index && event.clientY < trigger) || (from > index && event.clientY > trigger)) return;
        const next = moveItem(dragGroups.current ?? groups, from, index);
        dragGroups.current = next;
        draggingIndex.current = index;
        setGroups(next);
        window.requestAnimationFrame(() => animateReorder(groupElements.current, groupRects.current));
      }}
      onDrop={event => { event.preventDefault(); finishDrag(); }}
      onDragEnd={finishDrag}
    >
      <Bars3Icon className="group-drag-handle" aria-label="拖拽排序" />
      <GroupIcon code={group.icon} scale={group.iconScale} />
      <div className="group-card-copy"><div><strong>{group.name}</strong><span>{group.kind ? "内置" : GROUP_TYPE_LABELS[group.type] ?? group.type}</span>{group.enabled === false && <span>已停用</span>}</div><p>{groupSummary(group, data.availableNodes)}</p></div>
      <div className="group-card-actions">
        <IconButton label={group.enabled === false ? "当前已停用,点击启用" : "当前已启用,点击停用"} tooltip className={`tooltip-trigger${group.enabled === false ? "" : " enabled"}`} disabled={busyGroupId === group.id} onClick={() => void toggleGroup(group)}><PowerIcon /></IconButton>
        <IconButton label="修改分组" tooltip className="tooltip-trigger" onClick={() => openEdit(group)}><PencilSquareIcon /></IconButton>
        <IconButton label={group.kind ? "内置出站不能删除" : "删除"} tooltip className="tooltip-trigger tooltip-align-right" disabled={Boolean(group.kind)} onClick={() => !group.kind && setDeleting(group)}><TrashIcon /></IconButton>
      </div>
    </article>)}</div>}
    {data && !groups.length && <EmptyState icon="◇" title="没有出站分组" text="点击右上角添加分组。" compact />}

    {editor && data && <GroupEditorModal form={editor} groups={groups} data={data} busy={busy} setForm={setEditor} onSave={() => void saveEditor()} onClose={() => setEditor(null)} />}
    {deleting && <ConfirmModal title="删除分组" confirmLabel="确定" danger busy={busy} onClose={() => setDeleting(null)} onConfirm={async () => {
      if (await persist(groups.filter(group => group.id !== deleting.id), "分组已删除")) setDeleting(null);
    }}>确定删除分组「{deleting.name}」?分流规则里如果指向了它,需要另选一个目标。</ConfirmModal>}
    {restoreOpen && <ConfirmModal title="恢复默认" confirmLabel="确定" danger busy={busy} onClose={() => setRestoreOpen(false)} onConfirm={async () => {
      setBusy(true);
      try {
        const defaults = await api.defaultGroups();
        if (await persist(defaults, "已恢复默认节点组")) setRestoreOpen(false);
      } catch (reason) {
        onToast(errorMessage(reason, "恢复默认失败"));
        setBusy(false);
      }
    }}>将恢复为随安装包自带的默认节点组,你自己建的节点组会被删除。终端分流、目标分流里用到这些组的地方要重新选。</ConfirmModal>}
    {autoOpen && data && <AutoGroupsModal groups={groups} nodes={data.availableNodes} busy={busy} onClose={() => setAutoOpen(false)} onCreate={async (generated, skipped) => {
      if (!generated.length) { onToast("要生成的分组都已经存在了。"); return; }
      if (await persist(mergeAutoGroups(groups, generated))) {
        setAutoOpen(false);
        if (skipped) onToast(`已跳过 ${skipped} 个同名分组。`);
      }
    }} />}
  </>;
}

function GroupEditorModal({ form, groups, data, busy, setForm, onSave, onClose }: {
  form: OpenBoxGroup;
  groups: OpenBoxGroup[];
  data: GroupsResponse;
  busy: boolean;
  setForm: Dispatch<SetStateAction<OpenBoxGroup | null>>;
  onSave: () => void;
  onClose: () => void;
}) {
  const patch = (value: Partial<OpenBoxGroup>) => setForm(current => current ? { ...current, ...value } : current);
  const previousNonFailover = useRef<Pick<OpenBoxGroup, "mode" | "members" | "keywords" | "interval"> | null>(null);
  const initialType = useRef(form.type);
  const [discardFailoverOpen, setDiscardFailoverOpen] = useState(false);
  const discardFailoverConfirmed = useRef(false);
  const builtin = Boolean(form.kind);
  const matchingNodes = dynamicGroupMembers(form, data.availableNodes);
  const options: MemberOption[] = [
    ...groups.filter(group => group.id !== form.id).map(group => ({ kind: "group" as const, name: group.name, subscription: "" })),
    ...data.availableNodes.map(node => ({ kind: "node" as const, name: node.name, subscription: node.subscription })),
  ];
  const validFailoverLanes = (form.lanes ?? []).filter(lane => lane.members.some(member => data.availableNodes.some(node => node.name === member))).length;
  const valid = Boolean(form.name.trim()) && (builtin || (form.type === "failover" ? validFailoverLanes >= (form.id ? 1 : 2) : form.mode === "dynamic" || form.members.length > 0));
  const changeType = (type: GroupType) => setForm(current => {
    if (!current || current.type === type) return current;
    if (type === "failover") {
      previousNonFailover.current = { mode: current.mode, members: [...current.members], keywords: [...current.keywords], interval: current.interval };
      const nodeNames = new Set(data.availableNodes.map(node => node.name));
      const firstMembers = current.mode === "static" ? current.members.filter(member => nodeNames.has(member)) : [];
      return {
        ...current,
        type,
        mode: "static",
        members: [],
        keywords: [],
        interval: "300s",
        tolerance: current.tolerance ?? 100,
        testUrl: current.testUrl ?? "",
        lanes: current.lanes?.length ? current.lanes : [createFailoverLane(firstMembers), createFailoverLane()],
        failover: current.failover ?? { timeoutMs: 5000, failureThreshold: 2, restorePrimary: true, recoveryHoldMs: 60000 },
      };
    }
    const previous = previousNonFailover.current;
    return {
      ...current,
      type,
      mode: previous?.mode ?? "static",
      members: previous ? [...previous.members] : [],
      keywords: previous ? [...previous.keywords] : [],
      interval: previous?.interval && /^(\d+)(s|m|h)$/.test(previous.interval) ? previous.interval : "300s",
      tolerance: current.tolerance ?? 100,
    };
  });
  const requestSave = () => {
    if (form.id && initialType.current === "failover" && form.type !== "failover" && !discardFailoverConfirmed.current) setDiscardFailoverOpen(true);
    else onSave();
  };
  return <><Modal title={form.id ? "修改分组" : "添加分组"} className={`group-edit-modal${builtin ? " builtin" : ""}`} onClose={onClose} footer={<><button className="compact-button" type="button" onClick={onClose}>取消</button><button className="primary-button" type="button" disabled={busy || !valid} onClick={requestSave}>{busy && <span className="loading-spinner" />}保存</button></>}>
    <div className="group-editor-form">
      <div className="group-editor-primary-row">
        <Field label="图标"><PanelIconPicker label="分组图标" showLabel value={form.icon} onChange={icon => patch({ icon })} /></Field>
        <Field label="缩放"><ScaleControl value={form.iconScale ?? 0} onChange={iconScale => patch({ iconScale })} /></Field>
        <Field label="分组名称"><input value={form.name} onChange={event => patch({ name: event.target.value })} /></Field>
      </div>
      {builtin ? <p className="group-builtin-hint">内置出站只能改名字和图标。</p> : <>
        <div className={`group-editor-rule-row ${form.type === "selector" ? "selector" : ""}`}>
          <Field label="分组规则"><SelectControl label="分组规则" value={form.type} onValueChange={value => changeType(value as GroupType)} options={(data.types.length ? data.types : ["urltest", "selector", "failover"]).map(type => ({ value: type, label: GROUP_TYPE_LABELS[type] ?? type }))} /></Field>
          {(form.type === "urltest" || form.type === "failover") && <><Field label="检测间隔"><span className="group-unit-input"><input type="number" min={5} max={86400} value={secondsValue(form.interval)} onChange={event => patch({ interval: `${Math.max(0, Number(event.target.value) || 0)}s` })} /><span>秒</span></span></Field><Field label={form.type === "failover" ? "组内延迟容差" : "容差"}><span className="group-unit-input"><input type="number" min={0} disabled={form.type === "failover" && (form.lanes ?? []).every(lane => lane.members.filter(member => data.availableNodes.some(node => node.name === member)).length <= 1)} value={form.tolerance ?? 100} onChange={event => patch({ tolerance: Math.max(0, Number(event.target.value) || 0) })} /><span>毫秒</span></span></Field><Field label="测速地址"><input className="mono" value={form.testUrl ?? ""} placeholder="留空 = 用「分流与策略 → 其他」里的全局地址" onChange={event => patch({ testUrl: event.target.value })} /></Field></>}
        </div>
        {form.type === "failover" ? <FailoverEditor form={form} nodes={data.availableNodes} onChange={patch} /> : <><div className="group-mode-tabs" role="tablist" aria-label="分组模式"><button type="button" role="tab" aria-selected={form.mode === "dynamic"} className={form.mode === "dynamic" ? "active" : ""} onClick={() => patch({ mode: "dynamic" })}>动态组</button><button type="button" role="tab" aria-selected={form.mode === "static"} className={form.mode === "static" ? "active" : ""} onClick={() => patch({ mode: "static" })}>静态组</button></div>
        {form.mode === "dynamic" ? <div className="group-dynamic-editor"><Field label="关键词"><input value={form.keywords.join(",")} placeholder="关键词,用逗号分隔,如:香港,hk" onChange={event => patch({ keywords: commaValues(event.target.value) })} /></Field><p>节点名命中任一关键词就进组,留空 = 所有节点;新订阅也按此自动加入。</p><section className="group-dynamic-results"><header>当前命中 {matchingNodes.length} 个节点</header><div>{matchingNodes.length ? matchingNodes.map(name => <div key={name}><span>{name}</span></div>) : <p>当前没有节点命中这些关键词</p>}</div></section></div> : <MemberPicker value={form.members} options={options} onChange={members => patch({ members })} />}</>}
      </>}
    </div>
  </Modal>{discardFailoverOpen && <ConfirmModal title="修改分组规则" confirmLabel="继续保存" danger busy={busy} onClose={() => setDiscardFailoverOpen(false)} onConfirm={() => { discardFailoverConfirmed.current = true; setDiscardFailoverOpen(false); onSave(); }}>改为其他分组规则后，现有的主用和备用页签配置会被删除。确定继续？</ConfirmModal>}</>;
}

function createFailoverLane(members: string[] = []): OpenBoxGroupLane {
  return { id: `lane-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`, name: "", icon: "", members };
}

function FailoverEditor({ form, nodes, onChange }: { form: OpenBoxGroup; nodes: GroupsResponse["availableNodes"]; onChange: (value: Partial<OpenBoxGroup>) => void }) {
  const lanes = form.lanes?.length ? form.lanes : [createFailoverLane(), createFailoverLane()];
  const [activeId, setActiveId] = useState(lanes[0]?.id ?? "");
  const [advanced, setAdvanced] = useState(false);
  const [deleting, setDeleting] = useState<OpenBoxGroupLane | null>(null);
  const active = lanes.find(lane => lane.id === activeId) ?? lanes[0];
  const failover = form.failover ?? { timeoutMs: 5000, failureThreshold: 2, restorePrimary: true, recoveryHoldMs: 60000 };
  const options = nodes.map(node => ({ kind: "node" as const, name: node.name, subscription: node.subscription }));
  useEffect(() => {
    if (!form.lanes?.length) onChange({ lanes, failover });
  }, []);
  useEffect(() => {
    if (active && active.id !== activeId) setActiveId(active.id);
  }, [active?.id, activeId]);
  const updateLane = (value: Partial<OpenBoxGroupLane>) => onChange({ lanes: lanes.map(lane => lane.id === active.id ? { ...lane, ...value } : lane) });
  const removeLane = (lane: OpenBoxGroupLane) => {
    const next = lanes.filter(item => item.id !== lane.id);
    onChange({ lanes: next });
    if (activeId === lane.id) setActiveId(next[Math.min(lanes.indexOf(lane), Math.max(0, next.length - 1))]?.id ?? "");
    setDeleting(null);
  };
  if (!active) return null;
  return <>
    <section className="group-failover-editor">
      <button type="button" className="group-failover-advanced-toggle" aria-expanded={advanced} onClick={() => setAdvanced(value => !value)}>高级设置 <ChevronDownIcon /></button>
      {advanced && <div className="group-failover-advanced">
        <Field label="测速超时"><span className="group-unit-input"><input type="number" min={1} max={60} value={Math.round(failover.timeoutMs / 1000)} onChange={event => onChange({ failover: { ...failover, timeoutMs: Math.min(60, Math.max(1, Number(event.target.value) || 5)) * 1000 } })} /><span>秒</span></span></Field>
        <Field label="连续失败次数"><input type="number" min={1} value={failover.failureThreshold} onChange={event => onChange({ failover: { ...failover, failureThreshold: Math.max(1, Number(event.target.value) || 2) } })} /></Field>
        <label className="group-failover-switch"><span>主用恢复后自动切回</span><SwitchControl label="主用恢复后自动切回" checked={failover.restorePrimary} onCheckedChange={restorePrimary => onChange({ failover: { ...failover, restorePrimary } })} /></label>
        <Field label="恢复主用等待"><span className="group-unit-input"><input type="number" min={0} max={86400} value={Math.round(failover.recoveryHoldMs / 1000)} onChange={event => onChange({ failover: { ...failover, recoveryHoldMs: Math.min(86400, Math.max(0, Number(event.target.value) || 0)) * 1000 } })} /><span>秒</span></span></Field>
      </div>}
      <p className="group-failover-hint">从左侧选节点加入当前页签；最前面的页签是主用，后面依次备用。单节点页签直接使用，多节点页签可选自动优选或手动选择。</p>
      <FailoverMemberPicker
        lanes={lanes}
        active={active}
        options={options}
        groupIcon={form.icon}
        onSelectLane={setActiveId}
        onAddLane={() => { const lane = createFailoverLane(); onChange({ lanes: [...lanes, lane] }); setActiveId(lane.id); }}
        onDeleteLane={() => active.members.length ? setDeleting(active) : removeLane(active)}
        onChange={updateLane}
      />
    </section>
    {deleting && <ConfirmModal title="删除页签" confirmLabel="确定" danger busy={false} onClose={() => setDeleting(null)} onConfirm={() => removeLane(deleting)}>页签「{deleting.name || (lanes.indexOf(deleting) === 0 ? "主用" : `备用 ${lanes.indexOf(deleting)}`)}」中已有节点，确定删除？</ConfirmModal>}
  </>;
}

function FailoverMemberPicker({ lanes, active, options, groupIcon, onSelectLane, onAddLane, onDeleteLane, onChange }: {
  lanes: OpenBoxGroupLane[];
  active: OpenBoxGroupLane;
  options: MemberOption[];
  groupIcon: string;
  onSelectLane: (id: string) => void;
  onAddLane: () => void;
  onDeleteLane: () => void;
  onChange: (value: Partial<OpenBoxGroupLane>) => void;
}) {
  const [availableSearch, setAvailableSearch] = useState("");
  const [selectedSearch, setSelectedSearch] = useState("");
  const [availableFilter, setAvailableFilter] = useState("");
  const [selectedFilter, setSelectedFilter] = useState("");
  const [availableChecked, setAvailableChecked] = useState<string[]>([]);
  const [selectedChecked, setSelectedChecked] = useState<string[]>([]);
  const subscriptions = [...new Set(options.map(item => item.subscription).filter(Boolean))];
  const available = filterMembers(options.filter(item => !active.members.includes(item.name)), availableSearch, availableFilter);
  const selected = filterMembers(active.members.map(name => options.find(item => item.name === name) ?? { kind: "node", name, subscription: "" }), selectedSearch, selectedFilter);
  const add = (names: string[]) => { onChange({ members: [...active.members, ...names.filter(name => !active.members.includes(name))] }); setAvailableChecked(current => current.filter(name => !names.includes(name))); };
  const remove = (names: string[]) => { onChange({ members: active.members.filter(name => !names.includes(name)) }); setSelectedChecked(current => current.filter(name => !names.includes(name))); };
  const resetChecks = (id: string) => { setAvailableChecked([]); setSelectedChecked([]); onSelectLane(id); };
  return <div className="group-member-picker group-failover-picker">
    <MemberPane title={`可选 (${options.length - active.members.length})`} items={available} checked={availableChecked} setChecked={setAvailableChecked} search={availableSearch} setSearch={setAvailableSearch} filter={availableFilter} setFilter={setAvailableFilter} subscriptions={subscriptions} showGroups={false} direction="right" onMove={name => add([name])} />
    <div className="group-member-transfer"><IconButton label="添加勾选的" tooltip className="tooltip-trigger" disabled={!availableChecked.length} onClick={() => add(availableChecked)}><ArrowRightIcon /></IconButton><IconButton label="移出勾选的" tooltip className="tooltip-trigger" disabled={!selectedChecked.length} onClick={() => remove(selectedChecked)}><ArrowLeftIcon /></IconButton></div>
    <section className="group-failover-selected">
      <div className="group-failover-tabs" role="tablist" aria-label="故障转移页签">
        {lanes.map((lane, index) => <button type="button" role="tab" aria-selected={lane.id === active.id} className={lane.id === active.id ? "active" : ""} key={lane.id} onClick={() => resetChecks(lane.id)}>{index === 0 ? "主用" : `备用 ${index}`} · {lane.members.filter(member => options.some(option => option.name === member)).length}</button>)}
        {lanes.length < 3 && <IconButton label="添加备用页签" tooltip className="tooltip-trigger tooltip-align-right" onClick={onAddLane}><PlusIcon /></IconButton>}
      </div>
      <div className="group-failover-lane-settings">
        <PanelIconPicker label="页签图标" showLabel value={active.icon || groupIcon} emptyLabel="图标" onChange={icon => onChange({ icon: icon === groupIcon ? "" : icon })} />
        <input aria-label="页签名称（可选）" value={active.name} placeholder="页签名（可选）" onChange={event => onChange({ name: event.target.value })} />
        <IconButton label="删除当前页签" tooltip className="group-failover-delete tooltip-trigger tooltip-align-right" onClick={onDeleteLane}><TrashIcon /></IconButton>
      </div>
      <div className="group-failover-selected-head"><strong>已选 ({active.members.length})</strong><SelectControl label="页签模式" value={active.manual ? "manual" : "auto"} onValueChange={value => onChange({ manual: value === "manual" || undefined })} options={[{ value: "auto", label: "自动优选" }, { value: "manual", label: "手动选择" }]} /><span><MagnifyingGlassIcon /><input value={selectedSearch} placeholder="按名称过滤…" onChange={event => setSelectedSearch(event.target.value)} /></span></div>
      <div className="group-member-tools"><button type="button" onClick={() => setSelectedChecked(active.members)}>全选</button><button type="button" onClick={() => setSelectedChecked(current => active.members.filter(name => !current.includes(name)))}>反选</button><button type="button" onClick={() => setSelectedChecked([])}>清空选择</button><SelectControl label="已选节点过滤" value={selectedFilter} onValueChange={setSelectedFilter} options={[{ value: "", label: "全部" }, ...subscriptions.map(subscription => ({ value: `sub:${subscription}`, label: subscription }))]} /></div>
      <div className="group-member-list">{selected.length ? selected.map(item => <label key={item.name}><input type="checkbox" checked={selectedChecked.includes(item.name)} onChange={event => setSelectedChecked(current => event.target.checked ? [...current, item.name] : current.filter(name => name !== item.name))} /><span>{item.name}</span><IconButton label="移出" tooltip className="tooltip-trigger tooltip-align-right" onClick={event => { event.preventDefault(); remove([item.name]); }}><ArrowLeftIcon /></IconButton></label>) : <p>勾选左边的条目,按中间的箭头加进来。</p>}</div>
    </section>
  </div>;
}

function MemberPicker({ value, options, availableTitle = "可选", selectedTitle = "已选", onChange }: { value: string[]; options: MemberOption[]; availableTitle?: string; selectedTitle?: string; onChange: (value: string[]) => void }) {
  const [availableSearch, setAvailableSearch] = useState("");
  const [selectedSearch, setSelectedSearch] = useState("");
  const [availableFilter, setAvailableFilter] = useState("");
  const [selectedFilter, setSelectedFilter] = useState("");
  const [availableChecked, setAvailableChecked] = useState<string[]>([]);
  const [selectedChecked, setSelectedChecked] = useState<string[]>([]);
  const availablePool = options.filter(item => !value.includes(item.name));
  const selectedPool: MemberOption[] = value.map(name => options.find(item => item.name === name) ?? { kind: "node", name, subscription: "" });
  const available = filterMembers(availablePool, availableSearch, availableFilter);
  const selected = filterMembers(selectedPool, selectedSearch, selectedFilter);
  const add = (names: string[]) => { onChange([...value, ...names.filter(name => !value.includes(name))]); setAvailableChecked(current => current.filter(name => !names.includes(name))); };
  const remove = (names: string[]) => { onChange(value.filter(name => !names.includes(name))); setSelectedChecked(current => current.filter(name => !names.includes(name))); };
  return <div className="group-member-picker">
    <MemberPane title={`${availableTitle} (${options.length - value.length})`} items={available} checked={availableChecked} setChecked={setAvailableChecked} search={availableSearch} setSearch={setAvailableSearch} filter={availableFilter} setFilter={setAvailableFilter} filterOptions={memberFilterOptions(availablePool)} direction="right" onMove={name => add([name])} />
    <div className="group-member-transfer"><IconButton label="添加勾选的" tooltip className="tooltip-trigger" disabled={!availableChecked.length} onClick={() => add(availableChecked)}><ChevronRightIcon /></IconButton><IconButton label="移出勾选的" tooltip className="tooltip-trigger" disabled={!selectedChecked.length} onClick={() => remove(selectedChecked)}><ChevronLeftIcon /></IconButton></div>
    <MemberPane title={`${selectedTitle} (${value.length})`} items={selected} checked={selectedChecked} setChecked={setSelectedChecked} search={selectedSearch} setSearch={setSelectedSearch} filter={selectedFilter} setFilter={setSelectedFilter} filterOptions={memberFilterOptions(selectedPool)} direction="left" onMove={name => remove([name])} empty="勾选左边的条目,按中间的箭头加进来。" />
  </div>;
}

function MemberPane({ title, items, checked, setChecked, search, setSearch, filter, setFilter, subscriptions = [], filterOptions, showGroups = true, direction, onMove, empty }: {
  title: string;
  items: MemberOption[];
  checked: string[];
  setChecked: Dispatch<SetStateAction<string[]>>;
  search: string;
  setSearch: (value: string) => void;
  filter: string;
  setFilter: (value: string) => void;
  subscriptions?: string[];
  filterOptions?: { value: string; label: string }[];
  showGroups?: boolean;
  direction: "left" | "right";
  onMove: (name: string) => void;
  empty?: string;
}) {
  const names = items.map(item => item.name);
  const toggleAll = () => setChecked(current => [...new Set([...current, ...names])]);
  const invert = () => setChecked(current => [...current.filter(name => !names.includes(name)), ...names.filter(name => !current.includes(name))]);
  const clear = () => setChecked(current => current.filter(name => !names.includes(name)));
  return <section className="group-member-pane"><header className="group-member-head"><div className="group-member-title"><strong>{title}</strong><span><input value={search} placeholder="按名称过滤…" onChange={event => setSearch(event.target.value)} /></span></div><div className="group-member-tools"><button type="button" onClick={toggleAll}>全选</button><button type="button" onClick={invert}>反选</button><button type="button" onClick={clear}>清空选择</button><SelectControl label={`${title}过滤`} value={filter} onValueChange={setFilter} options={filterOptions ?? [{ value: "", label: "全部" }, ...(showGroups ? [{ value: "kind:group", label: "全部节点组" }] : []), { value: "kind:node", label: "全部节点" }, ...subscriptions.map(subscription => ({ value: `sub:${subscription}`, label: subscription }))]} /></div></header><div className="group-member-list">{items.length ? items.map(item => <label key={`${item.kind}-${item.name}`}><input type="checkbox" checked={checked.includes(item.name)} onChange={event => setChecked(current => event.target.checked ? [...current, item.name] : current.filter(name => name !== item.name))} />{direction === "left" && <IconButton label="移出" tooltip className="tooltip-trigger" onClick={event => { event.preventDefault(); onMove(item.name); }}><ChevronLeftIcon /></IconButton>}<span>{item.name}</span>{direction === "right" && <>{item.kind === "group" && <small>出站节点</small>}<IconButton label="添加" tooltip className="tooltip-trigger tooltip-align-right" onClick={event => { event.preventDefault(); onMove(item.name); }}><ChevronRightIcon /></IconButton></>}</label>) : <p>{empty ?? "没有匹配的条目。"}</p>}</div></section>;
}

export function ScaleControl({ value, onChange }: { value: number; onChange: (value: number) => void }) {
  const id = useId();
  const tooltip = useRef<HTMLDivElement>(null);
  const [tip, setTip] = useState<{ text: string; left: number; top: number; triggerTop: number; triggerBottom: number; wide: boolean } | null>(null);
  const show = (target: EventTarget) => {
    const element = (target as Element).closest<HTMLElement>("[data-tooltip]");
    if (!element || element.matches(":disabled")) return;
    const text = element.dataset.tooltip!;
    const rect = element.getBoundingClientRect();
    const wide = element.classList.contains("group-scale-value");
    const halfWidth = Math.min(wide ? 280 : 100, window.innerWidth - 32) / 2;
    setTip({ text, wide, left: Math.max(halfWidth + 16, Math.min(rect.left + rect.width / 2, window.innerWidth - halfWidth - 16)), top: rect.bottom + 6, triggerTop: rect.top, triggerBottom: rect.bottom });
  };
  useLayoutEffect(() => {
    if (!tip || !tooltip.current) return;
    const height = tooltip.current.getBoundingClientRect().height;
    const below = tip.triggerBottom + 6;
    const top = below + height <= window.innerHeight - 8 ? below : Math.max(8, tip.triggerTop - height - 6);
    if (top !== tip.top) setTip({ ...tip, top });
  }, [tip]);
  useEffect(() => {
    if (!tip) return;
    const close = () => setTip(null);
    window.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    return () => { window.removeEventListener("scroll", close, true); window.removeEventListener("resize", close); };
  }, [tip]);
  return <>
    <div className="group-scale-control" onMouseOver={event => show(event.target)} onMouseLeave={() => setTip(null)} onFocus={event => show(event.target)} onBlur={() => setTip(null)} onKeyDown={event => { if (event.key === "Escape") setTip(null); }}>
      <IconButton label="缩小 1px" tooltip onClick={() => onChange(value - 1)}><MinusIcon /></IconButton>
      <span className="group-scale-value" data-tooltip="默认 0 不缩放;按代理页的大图标算,+1 加大 1px,-1 缩小 1px;其他地方的小图标按同一比例等比缩放"><input aria-label="图标缩放" aria-describedby={tip ? id : undefined} readOnly value={value} /></span>
      <IconButton label="加大 1px" tooltip onClick={() => onChange(value + 1)}><PlusIcon /></IconButton>
      <button type="button" className="group-scale-reset" aria-label="重置缩放" data-tooltip="重置" disabled={value === 0} onClick={() => onChange(0)}>重置</button>
    </div>
    {tip && createPortal(<div ref={tooltip} id={id} role="tooltip" className={`group-scale-tooltip${tip.wide ? " wide" : ""}`} style={{ left: tip.left, top: tip.top }}>{tip.text}</div>, document.body)}
  </>;
}

type CountryDrag = { code: string; x: number; y: number; startX: number; startY: number; offsetX: number; offsetY: number; width: number; height: number; active: boolean };

function AutoGroupsModal({ groups, nodes, busy, onClose, onCreate }: { groups: OpenBoxGroup[]; nodes: GroupsResponse["availableNodes"]; busy: boolean; onClose: () => void; onCreate: (groups: OpenBoxGroup[], skipped: number) => void }) {
  const [types, setTypes] = useState<GroupType[]>(["urltest"]);
  const [countries, setCountries] = useState<string[]>(DEFAULT_AUTO_COUNTRIES);
  const [drag, setDrag] = useState<CountryDrag | null>(null);
  const dragState = useRef<CountryDrag | null>(null);
  const currentCountries = useRef(countries);
  currentCountries.current = countries;
  const countryElements = useRef(new Map<string, HTMLElement>());
  const countryRects = useRef(new Map<string, DOMRect>());
  const list = useRef<HTMLDivElement>(null);
  const addable = availableAutoCountries(countries, nodes);
  const draggingCountry = drag?.active ? COUNTRY_OPTIONS.find(country => country.code === drag.code) : null;

  const reorderAtPointer = (y: number) => {
    const current = dragState.current;
    if (!current?.active) return;
    const from = currentCountries.current.indexOf(current.code);
    const to = currentCountries.current.findIndex(code => {
      const rect = countryElements.current.get(code)!.getBoundingClientRect();
      return y >= rect.top && y <= rect.bottom;
    });
    if (to < 0 || to === from) return;
    const rect = countryElements.current.get(currentCountries.current[to])!.getBoundingClientRect();
    if (from < to ? y < rect.top + rect.height * .25 : y > rect.top + rect.height * .75) return;
    countryElements.current.forEach((element, code) => countryRects.current.set(code, element.getBoundingClientRect()));
    const next = moveItem(currentCountries.current, from, to);
    currentCountries.current = next;
    setCountries(next);
    requestAnimationFrame(() => animateReorder(countryElements.current, countryRects.current));
  };

  const beginDrag = (event: ReactPointerEvent<SVGSVGElement>, code: string) => {
    if (event.button !== 0) return;
    event.preventDefault();
    list.current!.setPointerCapture(event.pointerId);
    const rect = countryElements.current.get(code)!.getBoundingClientRect();
    dragState.current = { code, x: event.clientX, y: event.clientY, startX: event.clientX, startY: event.clientY, offsetX: event.clientX - rect.left, offsetY: event.clientY - rect.top, width: rect.width, height: rect.height, active: false };
  };
  const moveDrag = (event: ReactPointerEvent<HTMLDivElement>) => {
    const current = dragState.current;
    if (!current) return;
    const active = current.active || Math.hypot(event.clientX - current.startX, event.clientY - current.startY) >= 3;
    dragState.current = { ...current, x: event.clientX, y: event.clientY, active };
    if (active) { setDrag(dragState.current); reorderAtPointer(event.clientY); }
  };
  const endDrag = (event: ReactPointerEvent<HTMLDivElement>) => {
    dragState.current = null;
    setDrag(null);
    countryRects.current.clear();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
  };
  useEffect(() => {
    if (!drag?.active) return;
    let frame: number;
    const scroll = () => {
      const pointer = dragState.current;
      const element = list.current;
      if (!pointer || !element) return;
      const scroller = element.scrollHeight > element.clientHeight ? element : element.closest<HTMLElement>(".ob-modal-body")!;
      const rect = scroller.getBoundingClientRect();
      const before = scroller.scrollTop;
      if (pointer.y < rect.top + 24) scroller.scrollTop -= 6;
      else if (pointer.y > rect.bottom - 24) scroller.scrollTop += 6;
      if (scroller.scrollTop !== before) reorderAtPointer(pointer.y);
      frame = requestAnimationFrame(scroll);
    };
    frame = requestAnimationFrame(scroll);
    return () => cancelAnimationFrame(frame);
  }, [drag?.active]);

  return <Modal title="按国家自动分组" className="group-auto-modal" onClose={onClose} footer={<><button className="compact-button" type="button" onClick={onClose}>取消</button><button className="primary-button" type="button" disabled={busy || !countries.length || !types.length} onClick={() => {
    const generated = buildAutoGroups(groups, countries, types);
    onCreate(generated, countries.length * types.length - generated.length);
  }}>{busy && <span className="loading-spinner" />}生成 {countries.length * types.length} 个分组</button></>}>
    <div className="group-auto-form">
      <fieldset><legend>要建哪种组（可以都建）</legend>{(["urltest", "selector"] as GroupType[]).map(type => <label key={type}><input type="checkbox" checked={types.includes(type)} onChange={event => setTypes(current => event.target.checked ? [...current, type] : current.filter(value => value !== type))} />{GROUP_TYPE_LABELS[type]}</label>)}</fieldset>
      <section>
        <header><strong>选择国家/地区</strong><AutoCountrySelect options={addable} onSelect={code => setCountries(current => [...current, code])} /><button type="button" disabled={!countries.length} onClick={() => setCountries([])}>清空选择</button></header>
        <div ref={list} className="group-auto-country-list" onPointerMove={moveDrag} onPointerUp={endDrag} onPointerCancel={endDrag} onLostPointerCapture={endDrag} onDragStart={event => event.preventDefault()}>
          {countries.map(code => {
            const country = COUNTRY_OPTIONS.find(item => item.code === code)!;
            return <article key={code} ref={element => { if (element) countryElements.current.set(code, element); else countryElements.current.delete(code); }} className={`group-auto-country-row${drag?.active && drag.code === code ? " is-drag-placeholder" : ""}`}>
              <Bars3Icon className="country-drag-handle" aria-label={`拖动${country.name}`} onPointerDown={event => beginDrag(event, code)} />
              <GroupIcon code={code} baseSize={16} /><strong>{country.name}</strong><span>{countryNodeCount(country.keywords, nodes)} 个节点</span><IconButton label={`删除${country.name}`} onClick={() => setCountries(current => current.filter(value => value !== code))}><TrashIcon /></IconButton>
            </article>;
          })}
          {!countries.length && <p className="group-auto-empty">还没有选国家/地区,用右上角的下拉框添加。</p>}
        </div>
      </section>
      <p>按「国家-自动」「国家-手动」命名并配国旗;动态组,以后新订阅里这个国家的节点自动进组。</p>
    </div>
    {draggingCountry && drag && createPortal(<article aria-hidden="true" className="group-auto-country-row group-auto-drag-preview" style={{ left: drag.x - drag.offsetX, top: drag.y - drag.offsetY, width: drag.width, height: drag.height }}><Bars3Icon className="country-drag-handle" /><GroupIcon code={drag.code} baseSize={16} /><strong>{draggingCountry.name}</strong><span>{countryNodeCount(draggingCountry.keywords, nodes)} 个节点</span><span className="icon-button"><TrashIcon /></span></article>, document.querySelector(".openbox-app")!)}
  </Modal>;
}

function AutoCountrySelect({ options, onSelect }: { options: ReturnType<typeof availableAutoCountries>; onSelect: (code: string) => void }) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [container, setContainer] = useState<HTMLElement>();
  const searchInput = useRef<HTMLInputElement>(null);
  const popup = useRef<HTMLDivElement>(null);
  const filtered = options.filter(country => `${country.name} ${country.code}`.toLowerCase().includes(search.trim().toLowerCase()));
  useEffect(() => { setContainer(document.querySelector<HTMLElement>(".openbox-app")!); }, []);
  useEffect(() => {
    if (!open) return;
    const frame = requestAnimationFrame(() => searchInput.current?.focus());
    return () => cancelAnimationFrame(frame);
  }, [open]);
  return <Select value="" open={open} onOpenChange={value => { setOpen(value); if (value) setSearch(""); }} onValueChange={onSelect}>
    <SelectTrigger className="group-auto-add" aria-label="添加国家/地区"><GlobeAltIcon /><span>添加国家/地区</span></SelectTrigger>
    <SelectContent ref={popup} className="country-select-content" portalContainer={container} align="start" header={<div className="panel-icon-search country-select-search"><input ref={searchInput} aria-label="搜索国家/地区" placeholder="搜索国家/地区" value={search} onChange={event => setSearch(event.target.value)} onKeyDown={event => {
      if (event.key === "ArrowDown") { event.preventDefault(); popup.current?.querySelector<HTMLElement>('[role="option"]:not([data-disabled])')?.focus(); }
      if (event.key !== "Escape" && event.key !== "Tab") event.stopPropagation();
    }} /><button type="button" aria-label="清空国家搜索" onClick={() => { setSearch(""); searchInput.current?.focus(); }}><XMarkIcon /></button></div>}>
      <SelectGroup>{filtered.map(country => <SelectItem key={country.code} value={country.code}><GroupIcon code={country.code} baseSize={16} /><span>{country.name}</span><small>{country.code}</small></SelectItem>)}{!filtered.length && <p className="country-select-empty">没有匹配的国家/地区</p>}</SelectGroup>
    </SelectContent>
  </Select>;
}

function ConfirmModal({ title, children, confirmLabel, danger = false, busy, onClose, onConfirm }: { title: string; children: ReactNode; confirmLabel: string; danger?: boolean; busy: boolean; onClose: () => void; onConfirm: () => void | Promise<void> }) {
  return <Modal title={title} className="group-confirm-modal" onClose={onClose} footer={<><button className="compact-button" type="button" onClick={onClose}>取消</button><button className={danger ? "danger-button" : "primary-button"} type="button" disabled={busy} onClick={() => void onConfirm()}>{busy && <span className="loading-spinner" />}{confirmLabel}</button></>}><p>{children}</p></Modal>;
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return <label className="group-field"><span>{label}</span>{children}</label>;
}

export function GroupIcon({ code, scale = 0, baseSize = 18 }: { code: string; scale?: number; baseSize?: number }) {
  const size = Math.max(12, baseSize + scale);
  const asset = panelIcons.find(icon => icon.code === code)?.asset;
  if (asset) return <img className="group-icon" src={asset} alt={code} title={code} style={{ width: size, height: size }} />;
  return <span className="group-icon-fallback" title={code || "无"}>{code ? "◎" : "—"}</span>;
}

function animateReorder(elements: Map<string, HTMLElement>, previousRects: Map<string, DOMRect>) {
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  elements.forEach((element, key) => {
    const previous = previousRects.get(key);
    const next = element.getBoundingClientRect();
    if (!previous) return;
    const y = previous.top - next.top;
    if (Math.abs(y) > .5) element.animate([{ transform: `translateY(${y}px)` }, { transform: "translateY(0)" }], { duration: 180, easing: "cubic-bezier(.2,.8,.2,1)" });
  });
  previousRects.clear();
  elements.forEach((element, key) => previousRects.set(key, element.getBoundingClientRect()));
}
