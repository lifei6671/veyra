import {
  ArrowLeftIcon,
  ArrowRightIcon,
  ArrowUturnLeftIcon,
  Bars3Icon,
  ChevronDownIcon,
  MagnifyingGlassIcon,
  PencilSquareIcon,
  PlusIcon,
  PowerIcon,
  SparklesIcon,
  TrashIcon,
  XMarkIcon,
} from "@heroicons/react/24/outline";
import { useEffect, useRef, useState, type Dispatch, type ReactNode, type SetStateAction } from "react";
import { createPortal } from "react-dom";
import { api } from "../../api/client";
import type { GroupsResponse, OpenBoxGroup, OpenBoxGroupLane } from "../../api/types";
import { EmptyState, ErrorState, IconButton, Modal } from "../../components/shared";
import { SwitchControl } from "../../ui/controls";
import {
  buildAutoGroups,
  cloneGroup,
  commaValues,
  countryFlag,
  countryNodeCount,
  COUNTRY_OPTIONS,
  DEFAULT_AUTO_COUNTRIES,
  dynamicGroupMembers,
  errorMessage,
  filterMembers,
  groupSummary,
  GROUP_TYPE_LABELS,
  ICON_ASSETS,
  ICON_OPTIONS,
  mergeAutoGroups,
  moveItem,
  normalizeEditorGroup,
  secondsValue,
  type GroupType,
  type IconCategory,
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
    {data && <div className="group-card-list">{groups.map((group, index) => <article
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
    {autoOpen && data && <AutoGroupsModal groups={groups} nodes={data.availableNodes} busy={busy} onClose={() => setAutoOpen(false)} onCreate={async generated => { if (await persist(mergeAutoGroups(groups, generated), "自动分组已生成")) setAutoOpen(false); }} />}
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
        <Field label="图标"><IconPicker value={form.icon} onChange={icon => patch({ icon })} /></Field>
        <Field label="缩放"><ScaleControl value={form.iconScale ?? 0} onChange={iconScale => patch({ iconScale })} /></Field>
        <Field label="分组名称"><input value={form.name} onChange={event => patch({ name: event.target.value })} /></Field>
      </div>
      {builtin ? <p className="group-builtin-hint">内置出站只能改名字和图标。</p> : <>
        <div className={`group-editor-rule-row ${form.type === "selector" ? "selector" : ""}`}>
          <Field label="分组规则"><select value={form.type} onChange={event => changeType(event.target.value as GroupType)}>{(data.types.length ? data.types : ["urltest", "selector", "failover"]).map(type => <option key={type} value={type}>{GROUP_TYPE_LABELS[type] ?? type}</option>)}</select></Field>
          {(form.type === "urltest" || form.type === "failover") && <><Field label="检测间隔"><span className="group-unit-input"><input type="number" min={5} max={86400} value={secondsValue(form.interval)} onChange={event => patch({ interval: `${Math.max(0, Number(event.target.value) || 0)}s` })} /><span>秒</span></span></Field><Field label={form.type === "failover" ? "组内延迟容差" : "容差"}><span className="group-unit-input"><input type="number" min={0} disabled={form.type === "failover" && (form.lanes ?? []).every(lane => lane.members.filter(member => data.availableNodes.some(node => node.name === member)).length <= 1)} value={form.tolerance ?? 100} onChange={event => patch({ tolerance: Math.max(0, Number(event.target.value) || 0) })} /><span>毫秒</span></span></Field><Field label="测速地址"><input className="mono" value={form.testUrl ?? ""} placeholder="留空 = 用「分流与策略 → 其他」里的全局地址" onChange={event => patch({ testUrl: event.target.value })} /></Field></>}
        </div>
        {form.type === "failover" ? <FailoverEditor form={form} nodes={data.availableNodes} onChange={patch} /> : <><div className="group-mode-tabs" role="tablist" aria-label="分组模式"><button type="button" role="tab" aria-selected={form.mode === "dynamic"} className={form.mode === "dynamic" ? "active" : ""} onClick={() => patch({ mode: "dynamic" })}>动态组</button><button type="button" role="tab" aria-selected={form.mode === "static"} className={form.mode === "static" ? "active" : ""} onClick={() => patch({ mode: "static" })}>静态组</button></div>
        {form.mode === "dynamic" ? <div className="group-dynamic-editor"><Field label="关键词"><input value={form.keywords.join(",")} placeholder="关键词,用逗号分隔,如:香港,hk" onChange={event => patch({ keywords: commaValues(event.target.value) })} /></Field><p>节点名命中任一关键词就进组,留空 = 所有节点;新订阅也按此自动加入。</p><strong>当前命中 {matchingNodes.length} 个节点</strong><div>{matchingNodes.length ? matchingNodes.map(name => <span key={name}>{name}</span>) : <p>当前没有节点命中这些关键词</p>}</div></div> : <MemberPicker value={form.members} options={options} onChange={members => patch({ members })} />}</>}
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
        <IconPicker value={active.icon || groupIcon} emptyLabel="图标" onChange={icon => onChange({ icon: icon === groupIcon ? "" : icon })} />
        <input aria-label="页签名称（可选）" value={active.name} placeholder="页签名（可选）" onChange={event => onChange({ name: event.target.value })} />
        <IconButton label="删除当前页签" tooltip className="group-failover-delete tooltip-trigger tooltip-align-right" onClick={onDeleteLane}><TrashIcon /></IconButton>
      </div>
      <div className="group-failover-selected-head"><strong>已选 ({active.members.length})</strong><select aria-label="页签模式" value={active.manual ? "manual" : "auto"} onChange={event => onChange({ manual: event.target.value === "manual" || undefined })}><option value="auto">自动优选</option><option value="manual">手动选择</option></select><span><MagnifyingGlassIcon /><input value={selectedSearch} placeholder="按名称过滤…" onChange={event => setSelectedSearch(event.target.value)} /></span></div>
      <div className="group-member-tools"><button type="button" onClick={() => setSelectedChecked(active.members)}>全选</button><button type="button" onClick={() => setSelectedChecked(current => active.members.filter(name => !current.includes(name)))}>反选</button><button type="button" onClick={() => setSelectedChecked([])}>清空选择</button><select aria-label="已选节点过滤" value={selectedFilter} onChange={event => setSelectedFilter(event.target.value)}><option value="">全部</option>{subscriptions.map(subscription => <option key={subscription} value={`sub:${subscription}`}>{subscription}</option>)}</select></div>
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
  const subscriptions = [...new Set(options.filter(item => item.kind === "node").map(item => item.subscription).filter(Boolean))];
  const available = filterMembers(options.filter(item => !value.includes(item.name)), availableSearch, availableFilter);
  const selected = filterMembers(value.map(name => options.find(item => item.name === name) ?? { kind: "node", name, subscription: "" }), selectedSearch, selectedFilter);
  const add = (names: string[]) => { onChange([...value, ...names.filter(name => !value.includes(name))]); setAvailableChecked(current => current.filter(name => !names.includes(name))); };
  const remove = (names: string[]) => { onChange(value.filter(name => !names.includes(name))); setSelectedChecked(current => current.filter(name => !names.includes(name))); };
  return <div className="group-member-picker">
    <MemberPane title={`${availableTitle} (${options.length - value.length})`} items={available} checked={availableChecked} setChecked={setAvailableChecked} search={availableSearch} setSearch={setAvailableSearch} filter={availableFilter} setFilter={setAvailableFilter} subscriptions={subscriptions} direction="right" onMove={name => add([name])} />
    <div className="group-member-transfer"><IconButton label="添加勾选的" tooltip className="tooltip-trigger" disabled={!availableChecked.length} onClick={() => add(availableChecked)}><ArrowRightIcon /></IconButton><IconButton label="移出勾选的" tooltip className="tooltip-trigger" disabled={!selectedChecked.length} onClick={() => remove(selectedChecked)}><ArrowLeftIcon /></IconButton></div>
    <MemberPane title={`${selectedTitle} (${value.length})`} items={selected} checked={selectedChecked} setChecked={setSelectedChecked} search={selectedSearch} setSearch={setSelectedSearch} filter={selectedFilter} setFilter={setSelectedFilter} subscriptions={subscriptions} direction="left" onMove={name => remove([name])} empty="勾选左边的条目,按中间的箭头加进来。" />
  </div>;
}

function MemberPane({ title, items, checked, setChecked, search, setSearch, filter, setFilter, subscriptions, showGroups = true, direction, onMove, empty }: {
  title: string;
  items: MemberOption[];
  checked: string[];
  setChecked: Dispatch<SetStateAction<string[]>>;
  search: string;
  setSearch: (value: string) => void;
  filter: string;
  setFilter: (value: string) => void;
  subscriptions: string[];
  showGroups?: boolean;
  direction: "left" | "right";
  onMove: (name: string) => void;
  empty?: string;
}) {
  const names = items.map(item => item.name);
  const toggleAll = () => setChecked(current => [...new Set([...current, ...names])]);
  const invert = () => setChecked(current => [...current.filter(name => !names.includes(name)), ...names.filter(name => !current.includes(name))]);
  const clear = () => setChecked(current => current.filter(name => !names.includes(name)));
  return <section className="group-member-pane"><div className="group-member-title"><strong>{title}</strong><span><MagnifyingGlassIcon /><input value={search} placeholder="按名称过滤…" onChange={event => setSearch(event.target.value)} /></span></div><div className="group-member-tools"><button type="button" onClick={toggleAll}>全选</button><button type="button" onClick={invert}>反选</button><button type="button" onClick={clear}>清空选择</button><select aria-label={`${title}过滤`} value={filter} onChange={event => setFilter(event.target.value)}><option value="">全部</option>{showGroups && <option value="kind:group">全部节点组</option>}<option value="kind:node">全部节点</option>{subscriptions.map(subscription => <option key={subscription} value={`sub:${subscription}`}>{subscription}</option>)}</select></div><div className="group-member-list">{items.length ? items.map(item => <label key={`${item.kind}-${item.name}`}><input type="checkbox" checked={checked.includes(item.name)} onChange={event => setChecked(current => event.target.checked ? [...current, item.name] : current.filter(name => name !== item.name))} /><span>{item.name}</span>{item.kind === "group" && <small>出站节点</small>}<IconButton label={direction === "right" ? "添加" : "移出"} tooltip className="tooltip-trigger tooltip-align-right" onClick={event => { event.preventDefault(); onMove(item.name); }}>{direction === "right" ? <ArrowRightIcon /> : <ArrowLeftIcon />}</IconButton></label>) : <p>{empty ?? "没有匹配的条目。"}</p>}</div></section>;
}

function IconPicker({ value, emptyLabel = "无", onChange }: { value: string; emptyLabel?: string; onChange: (value: string) => void }) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [tab, setTab] = useState<"all" | IconCategory>("all");
  const current = ICON_OPTIONS.find(option => option.code === value);
  const shown = ICON_OPTIONS.filter(option => (tab === "all" || option.category === tab) && (!search.trim() || `${option.code} ${option.label}`.toLowerCase().includes(search.trim().toLowerCase())));
  return <div className="group-icon-picker-wrap"><button type="button" className="group-icon-picker" aria-expanded={open} onClick={() => setOpen(value => !value)}>{value ? <GroupIcon code={value} /> : <span className="group-icon-empty">◎</span>}<span>{current?.label ?? (value || emptyLabel)}</span><ChevronDownIcon /></button>{open && <div className="group-icon-popover"><label><MagnifyingGlassIcon /><input autoFocus value={search} placeholder="搜索国家/地区" onChange={event => setSearch(event.target.value)} /><IconButton label="关闭图标选择" tooltip className="tooltip-trigger tooltip-align-right" onClick={() => setOpen(false)}><XMarkIcon /></IconButton></label><div role="tablist"><button type="button" className={tab === "all" ? "active" : ""} onClick={() => setTab("all")}>全部</button><button type="button" className={tab === "region" ? "active" : ""} onClick={() => setTab("region")}>地区</button><button type="button" className={tab === "brand" ? "active" : ""} onClick={() => setTab("brand")}>公司</button><button type="button" className={tab === "other" ? "active" : ""} onClick={() => setTab("other")}>其他</button></div><div>{shown.map(option => <button type="button" key={option.code} className={value === option.code ? "active" : ""} onClick={() => { onChange(option.code); setOpen(false); }}><GroupIcon code={option.code} /><span>{option.label}</span></button>)}<button type="button" className={!value ? "active" : ""} onClick={() => { onChange(""); setOpen(false); }}><span className="group-icon-none">—</span><span>无</span></button></div></div>}</div>;
}

function ScaleControl({ value, onChange }: { value: number; onChange: (value: number) => void }) {
  return <div className="group-scale-control"><IconButton label="缩小 1px" tooltip className="tooltip-trigger" onClick={() => onChange(value - 1)}>−</IconButton><input aria-label="图标缩放" readOnly value={value} /><IconButton label="加大 1px" tooltip className="tooltip-trigger" onClick={() => onChange(value + 1)}>＋</IconButton><IconButton label="重置缩放" tooltip className="tooltip-trigger tooltip-align-right" disabled={value === 0} onClick={() => onChange(0)}><ArrowUturnLeftIcon /></IconButton></div>;
}

function AutoGroupsModal({ groups, nodes, busy, onClose, onCreate }: { groups: OpenBoxGroup[]; nodes: GroupsResponse["availableNodes"]; busy: boolean; onClose: () => void; onCreate: (groups: OpenBoxGroup[]) => void }) {
  const [types, setTypes] = useState<GroupType[]>(["urltest"]);
  const [countries, setCountries] = useState<string[]>(DEFAULT_AUTO_COUNTRIES);
  const [dragging, setDragging] = useState<string | null>(null);
  const draggingIndex = useRef<number | null>(null);
  const countryElements = useRef(new Map<string, HTMLElement>());
  const countryRects = useRef(new Map<string, DOMRect>());
  const generated = buildAutoGroups(groups, countries, types);
  const addable = COUNTRY_OPTIONS.filter(country => !countries.includes(country.code));
  return <Modal title="按国家自动分组" className="group-auto-modal" onClose={onClose} footer={<><button className="compact-button" type="button" onClick={onClose}>取消</button><button className="primary-button" type="button" disabled={busy || !generated.length} onClick={() => onCreate(generated)}>{busy && <span className="loading-spinner" />}生成 {countries.length * types.length} 个分组</button></>}>
    <div className="group-auto-form"><fieldset><legend>要建哪种组（可以都建）</legend>{(["urltest", "selector"] as GroupType[]).map(type => <label key={type}><input type="checkbox" checked={types.includes(type)} onChange={event => setTypes(current => event.target.checked ? [...current, type] : current.filter(value => value !== type))} />{GROUP_TYPE_LABELS[type]}</label>)}</fieldset><section><header><strong>选择国家/地区</strong><label className="group-auto-add"><PlusIcon /><select aria-label="添加国家/地区" value="" onChange={event => { if (event.target.value) setCountries(current => [...current, event.target.value]); }}><option value="">添加国家/地区</option>{addable.map(country => <option key={country.code} value={country.code}>{country.name}</option>)}</select></label><button type="button" disabled={!countries.length} onClick={() => setCountries([])}>清空选择</button></header><div>{countries.map((code, index) => { const country = COUNTRY_OPTIONS.find(item => item.code === code); if (!country) return null; return <article key={code} ref={element => { if (element) countryElements.current.set(code, element); else countryElements.current.delete(code); }} className={dragging === code ? "is-drag-placeholder" : ""} draggable onDragStart={() => { draggingIndex.current = index; countryElements.current.forEach((element, key) => countryRects.current.set(key, element.getBoundingClientRect())); requestAnimationFrame(() => setDragging(code)); }} onDragOver={event => { event.preventDefault(); const from = draggingIndex.current; if (from === null || from === index) return; const rect = event.currentTarget.getBoundingClientRect(); const trigger = rect.top + rect.height * (from < index ? .25 : .75); if ((from < index && event.clientY < trigger) || (from > index && event.clientY > trigger)) return; setCountries(current => moveItem(current, from, index)); draggingIndex.current = index; requestAnimationFrame(() => animateReorder(countryElements.current, countryRects.current)); }} onDragEnd={() => { draggingIndex.current = null; countryRects.current.clear(); setDragging(null); }}><Bars3Icon /><GroupIcon code={code} /><strong>{country.name}</strong><span>{countryNodeCount(country.keywords, nodes)} 个节点</span><IconButton label={`删除${country.name}`} tooltip className="tooltip-trigger tooltip-align-right" onClick={() => setCountries(current => current.filter(value => value !== code))}><TrashIcon /></IconButton></article>; })}</div></section><p>按「国家-自动」「国家-手动」命名并配国旗;动态组,以后新订阅里这个国家的节点自动进组。</p></div>
  </Modal>;
}

function ConfirmModal({ title, children, confirmLabel, danger = false, busy, onClose, onConfirm }: { title: string; children: ReactNode; confirmLabel: string; danger?: boolean; busy: boolean; onClose: () => void; onConfirm: () => void | Promise<void> }) {
  return <Modal title={title} className="group-confirm-modal" onClose={onClose} footer={<><button className="compact-button" type="button" onClick={onClose}>取消</button><button className={danger ? "danger-button" : "primary-button"} type="button" disabled={busy} onClick={() => void onConfirm()}>{busy && <span className="loading-spinner" />}{confirmLabel}</button></>}><p>{children}</p></Modal>;
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return <label className="group-field"><span>{label}</span>{children}</label>;
}

function GroupIcon({ code, scale = 0 }: { code: string; scale?: number }) {
  const country = /^[A-Za-z]{2}$/.test(code) ? code.toUpperCase() : "";
  const size = Math.max(12, 18 + scale);
  if (country) return <span className="group-country-icon" title={country} style={{ fontSize: `${size}px` }}>{countryFlag(country)}</span>;
  const asset = ICON_ASSETS[code];
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
