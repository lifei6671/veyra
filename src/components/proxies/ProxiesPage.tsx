import { useEffect, useMemo, useRef, useState, type FormEvent, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { filterPoolNodes, PROXY_PROTOCOLS, retainAvailableNodeId, shouldDismissDialog, useProxyRouting, validProbeUrl, wrappedDialogFocusIndex, type NodeFilter, type NodeSnapshot, type PoolSnapshot, type PoolSource, type ProxyProtocol, type Selection } from "../../lib/proxy-routing";

type Props = { active: boolean };
type FilterDraft = { regions: string; protocols: ProxyProtocol[]; includeKeywords: string; excludeKeywords: string; includeNodeIds: string[]; excludeNodeIds: string[] };
type PoolDraft = { name: string; enabled: boolean; providerIds: string[]; sourceFilters: Record<string, FilterDraft>; mode: "manual" | "urlTest"; selectedNodeId: string; probeUrl: string; intervalSeconds: string; toleranceMs: string };
const blankFilter = (): FilterDraft => ({ regions: "", protocols: [], includeKeywords: "", excludeKeywords: "", includeNodeIds: [], excludeNodeIds: [] });
const blankDraft = (): PoolDraft => ({ name: "", enabled: true, providerIds: [], sourceFilters: {}, mode: "manual", selectedNodeId: "", probeUrl: "https://www.gstatic.com/generate_204", intervalSeconds: "300", toleranceMs: "50" });
const split = (value: string) => value.split(",").map((item) => item.trim()).filter(Boolean);
const unique = (values: string[]) => new Set(values).size === values.length;
const safeText = (value: string, maximum: number) => Array.from(value).length <= maximum && !/[\u0000-\u001f\u007f]/.test(value);
const filterFromDraft = (draft: FilterDraft): NodeFilter => ({ regions: split(draft.regions), protocols: draft.protocols, includeKeywords: split(draft.includeKeywords), excludeKeywords: split(draft.excludeKeywords), includeNodeIds: draft.includeNodeIds, excludeNodeIds: draft.excludeNodeIds });
const sourcesFromDraft = (draft: PoolDraft): PoolSource[] => draft.providerIds.map((providerId) => ({ providerId, filter: filterFromDraft(draft.sourceFilters[providerId] ?? blankFilter()) }));

export function ProxiesPage({ active }: Props) {
  const { snapshot, loading, error, pending, notice, refresh, mutate, dismissNotice } = useProxyRouting();
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [dialogPool, setDialogPool] = useState<PoolSnapshot | "new" | null>(null);
  const [draft, setDraft] = useState(blankDraft);
  const [formError, setFormError] = useState<string | null>(null);
  const [pendingSelector, setPendingSelector] = useState<{ poolId: string; nodeId: string } | null>(null);
  const [menu, setMenu] = useState<{ pool: PoolSnapshot; x: number; y: number } | null>(null);
  const triggerRef = useRef<HTMLElement | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const nodesById = useMemo(() => new Map(snapshot?.nodes.map((node) => [node.id, node]) ?? []), [snapshot]);
  const selectors = useMemo(() => new Map(snapshot?.selectors.map((selector) => [selector.poolId, selector]) ?? []), [snapshot]);
  const operationBusy = pending || snapshot?.applyState.type === "applying" || snapshot?.applyState.type === "applyUnknown";
  useEffect(() => { if (active) void refresh(); }, [active, refresh]);
  useEffect(() => { if (!active) setMenu(null); }, [active]);
  useEffect(() => { if (menu === null) return; menuRef.current?.querySelector<HTMLElement>("button:not(:disabled)")?.focus(); const close = () => setMenu(null); const escape = (event: KeyboardEvent) => { if (event.key === "Escape") { setMenu(null); triggerRef.current?.focus(); } }; window.addEventListener("click", close); window.addEventListener("keydown", escape); return () => { window.removeEventListener("click", close); window.removeEventListener("keydown", escape); }; }, [menu]);

  function openDialog(pool: PoolSnapshot | "new", trigger?: HTMLElement | null) {
    triggerRef.current = trigger ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    setDialogPool(pool); setFormError(null);
    if (pool === "new") setDraft(blankDraft());
    else {
      const sourceFilters = Object.fromEntries(pool.sources.map((source) => [source.providerId, { regions: source.filter.regions.join(", "), protocols: source.filter.protocols, includeKeywords: source.filter.includeKeywords.join(", "), excludeKeywords: source.filter.excludeKeywords.join(", "), includeNodeIds: source.filter.includeNodeIds, excludeNodeIds: source.filter.excludeNodeIds }]));
      setDraft({ name: pool.name, enabled: pool.enabled, providerIds: pool.sources.map((source) => source.providerId), sourceFilters, mode: pool.selection.type, selectedNodeId: pool.selection.type === "manual" ? pool.selection.selectedNodeId ?? "" : "", probeUrl: pool.selection.type === "urlTest" ? pool.selection.probeUrl : "https://www.gstatic.com/generate_204", intervalSeconds: pool.selection.type === "urlTest" ? String(pool.selection.intervalSeconds) : "300", toleranceMs: pool.selection.type === "urlTest" ? String(pool.selection.toleranceMs) : "50" });
    }
  }
  function closeDialog() { if (pending) return; setDialogPool(null); queueMicrotask(() => triggerRef.current?.focus()); }
  async function submitPool(event: FormEvent) {
    event.preventDefault(); if (!snapshot || pending) return;
    const name = draft.name.trim(); if (!name || !safeText(name, 80)) { setFormError("名称需要 1–80 个无控制字符的文本"); return; }
    if (draft.providerIds.length === 0 || draft.providerIds.length > 100) { setFormError("需要选择 1–100 个订阅来源"); return; }
    for (const providerId of draft.providerIds) { const source = draft.sourceFilters[providerId] ?? blankFilter(); const groups = [split(source.regions), split(source.includeKeywords), split(source.excludeKeywords)]; if (groups.some((values) => values.length > 64 || !unique(values) || values.some((value) => !safeText(value, 255))) || source.includeNodeIds.some((id) => source.excludeNodeIds.includes(id))) { setFormError("来源筛选包含重复、过长或互相冲突的值"); return; } }
    const sources = draft.providerIds.map((providerId) => { const source = draft.sourceFilters[providerId] ?? blankFilter(); const filter: NodeFilter = { regions: split(source.regions), protocols: source.protocols, includeKeywords: split(source.includeKeywords), excludeKeywords: split(source.excludeKeywords), includeNodeIds: source.includeNodeIds, excludeNodeIds: source.excludeNodeIds }; return { providerId, filter }; });
    const selection: Selection = draft.mode === "manual" ? { type: "manual", selectedNodeId: draft.selectedNodeId || null } : { type: "urlTest", probeUrl: draft.probeUrl.trim(), intervalSeconds: Number(draft.intervalSeconds), toleranceMs: Number(draft.toleranceMs) };
    if (selection.type === "urlTest" && (!validProbeUrl(selection.probeUrl) || !Number.isInteger(selection.intervalSeconds) || selection.intervalSeconds < 1 || selection.intervalSeconds > 86400 || !Number.isInteger(selection.toleranceMs) || selection.toleranceMs < 0 || selection.toleranceMs > 60000)) { setFormError("自动测试 URL、间隔或容差不符合允许范围"); return; }
    const result = await mutate(dialogPool === "new" ? { type: "createCustomPool", name, enabled: draft.enabled, sources, selection } : { type: "updateCustomPool", id: dialogPool!.id, name, enabled: draft.enabled, sources, selection });
    if (result?.status === "ok") closeDialog();
  }
  async function chooseNode(poolId: string, nodeId: string) {
    setPendingSelector({ poolId, nodeId });
    try { await mutate({ type: "setManualSelection", poolId, nodeId }); }
    finally { setPendingSelector(null); }
  }
  const canApply = snapshot?.applyState.type === "savedPendingApply" || snapshot?.applyState.type === "savedApplyFailed";

  return <main className="home-content proxy-routing-page" id="page-outbounds" hidden={!active} aria-labelledby="page-outbounds-title">
    <header className="home-header"><h1 id="page-outbounds-title">出口组</h1><div className="proxy-header-actions"><span className={`apply-badge apply-${snapshot?.applyState.type ?? "loading"}`}>{applyLabel(snapshot?.applyState.type)}</span><button className="primary-button" type="button" disabled={operationBusy || (snapshot?.pools.length ?? 0) >= 100} onClick={(event) => openDialog("new", event.currentTarget)}>新建出口组</button><button className="secondary-button" type="button" disabled={operationBusy || !canApply} onClick={() => void mutate({ type: "applyConfiguration" })}>{snapshot?.applyState.type === "applying" ? "应用中…" : "应用"}</button></div></header>
    <div className="page-scroll proxy-routing-scroll">
      {loading && snapshot === null ? <PageState text="正在读取出口配置…" /> : error ? <PageState text={error === "busy" ? "配置正在使用中" : "出口配置不可用"} action="重试" onAction={() => void refresh()} /> : snapshot === null ? <PageState text="出口配置为空" /> : <>
        {snapshot.pools.length === 0 ? <PageState text="暂无出口组，可以新建一个自定义出口组" /> : <section className="proxy-groups" aria-label="出口组列表">{snapshot.pools.map((pool) => {
          const isExpanded = expanded.has(pool.id); const members = pool.resolvedNodeIds.map((id) => nodesById.get(id)).filter(Boolean); const selector = selectors.get(pool.id);
          return <article key={pool.id} className={`proxy-group ${!pool.enabled ? "is-disabled" : ""}`} onContextMenu={(event) => { if (pool.kind !== "custom") return; event.preventDefault(); triggerRef.current = event.currentTarget; setMenu({ pool, x: event.clientX, y: event.clientY }); }}>
            <header><button className="proxy-expand" type="button" aria-expanded={isExpanded} onClick={() => setExpanded((current) => { const next = new Set(current); next.has(pool.id) ? next.delete(pool.id) : next.add(pool.id); return next; })}><span className="chevron" aria-hidden="true">{isExpanded ? "⌄" : "›"}</span><span><strong>{pool.name}</strong><small>{pool.kind === "custom" ? "自定义" : "订阅出口组"} · {members.length} 个节点</small></span></button><div className="proxy-group-actions">{selector ? <span className={`selector-state selector-${selector.state}`}>{selectorLabel(selector.state)}</span> : null}{pool.kind === "custom" ? <button className="icon-button" type="button" aria-label={`${pool.name} 操作`} aria-haspopup="menu" title="操作" disabled={operationBusy} onClick={(event) => { event.stopPropagation(); triggerRef.current = event.currentTarget; const box = event.currentTarget.getBoundingClientRect(); setMenu({ pool, x: Math.max(8, box.right - 140), y: box.bottom + 4 }); }}>•••</button> : null}</div></header>
            {isExpanded ? <div className="proxy-node-list">{members.length === 0 ? <p className="dense-empty">当前筛选没有匹配节点</p> : members.map((node) => { const isPendingTarget = pendingSelector?.poolId === pool.id && pendingSelector.nodeId === node!.id; return <button key={node!.id} type="button" className={`proxy-node-row ${pool.selection.type === "manual" && pool.selection.selectedNodeId === node!.id ? "is-selected" : ""} ${isPendingTarget ? "is-pending" : ""}`} disabled={!pool.enabled || operationBusy || pool.selection.type !== "manual"} aria-pressed={pool.selection.type === "manual" && pool.selection.selectedNodeId === node!.id} aria-busy={isPendingTarget || undefined} onClick={() => void chooseNode(pool.id, node!.id)}><span><strong>{node!.name}</strong><small>{node!.protocol}</small></span>{isPendingTarget ? <span className="inline-pending">切换中…</span> : pool.selection.type === "manual" && pool.selection.selectedNodeId === node!.id ? <span className="selected-mark">已选</span> : pool.selection.type === "urlTest" ? <span className="secondary-copy">自动选择</span> : null}</button>; })}</div> : null}
          </article>;
        })}</section>}
        <section className="all-nodes" aria-label="全部节点"><header><h2>全部节点</h2><span>{snapshot.nodes.length}</span></header>{snapshot.nodes.length === 0 ? <p className="dense-empty">当前没有可用节点</p> : <div className="all-node-grid">{snapshot.nodes.map((node) => <div key={node.id} className="all-node"><strong>{node.name}</strong><span>{node.protocol}</span></div>)}</div>}</section>
      </>}
    </div>
    {notice ? <div className={`proxy-routing-notice notice-${notice.kind}`} role={notice.kind === "error" ? "alert" : "status"}><span>{notice.message}</span><button type="button" onClick={dismissNotice}>关闭</button></div> : null}
    {menu ? <div className="proxy-context-menu" ref={menuRef} role="menu" style={{ left: menu.x, top: menu.y }} onKeyDown={(event) => { if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return; event.preventDefault(); const items = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")); const index = items.indexOf(document.activeElement as HTMLButtonElement); items[(index + (event.key === "ArrowDown" ? 1 : -1) + items.length) % items.length]?.focus(); }}><button role="menuitem" type="button" onClick={() => openDialog(menu.pool, triggerRef.current)}>编辑</button><button role="menuitem" className="danger-item" type="button" disabled={operationBusy} onClick={() => void mutate({ type: "deleteCustomPool", id: menu.pool.id })}>删除</button></div> : null}
    {dialogPool ? <PoolDialog draft={draft} setDraft={setDraft} providers={snapshot?.providers ?? []} nodes={snapshot?.nodes ?? []} error={formError} busy={operationBusy} title={dialogPool === "new" ? "新建自定义出口组" : "编辑自定义出口组"} onClose={closeDialog} onSubmit={submitPool} /> : null}
  </main>;
}

function PageState({ text, action, onAction }: { text: string; action?: string; onAction?: () => void }) { return <div className="proxy-routing-state" role="status"><p>{text}</p>{action ? <button className="secondary-button" type="button" onClick={onAction}>{action}</button> : null}</div>; }
function applyLabel(type?: string) { return type === "applied" ? "已应用" : type === "savedPendingApply" ? "有未应用更改" : type === "applying" ? "正在应用" : type === "savedApplyFailed" ? "应用失败" : type === "applyUnknown" ? "应用结果待确认" : "读取中"; }
function selectorLabel(state: string) { return state === "inSync" ? "运行中" : state === "savedOnly" ? "已保存" : state === "notApplied" ? "未切换" : "待确认"; }
function trapTab(event: ReactKeyboardEvent, root: HTMLElement | null) { if (event.key !== "Tab" || !root) return; const controls = Array.from(root.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex='-1'])")); const target = wrappedDialogFocusIndex(controls.indexOf(document.activeElement as HTMLElement), controls.length, event.shiftKey); if (target !== null) { event.preventDefault(); controls[target]?.focus(); } }
function PoolDialog({ draft, setDraft, providers, nodes, error, busy, title, onClose, onSubmit }: { draft: PoolDraft; setDraft: (value: PoolDraft) => void; providers: { id: string; name: string }[]; nodes: NodeSnapshot[]; error: string | null; busy: boolean; title: string; onClose(): void; onSubmit(event: FormEvent): void }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => { ref.current?.querySelector<HTMLElement>("input, button")?.focus(); }, []);
  useEffect(() => { const key = (event: KeyboardEvent) => { if (shouldDismissDialog(event.key, busy)) onClose(); }; window.addEventListener("keydown", key); return () => window.removeEventListener("keydown", key); }, [busy, onClose]);
  const availableNodes = filterPoolNodes(nodes, sourcesFromDraft(draft));
  const updateDraft = (next: PoolDraft) => {
    const selectedNodeId = retainAvailableNodeId(next.selectedNodeId, filterPoolNodes(nodes, sourcesFromDraft(next)));
    setDraft(selectedNodeId === next.selectedNodeId ? next : { ...next, selectedNodeId });
  };
  return <div className="dialog-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}><div className="proxy-routing-dialog" ref={ref} role="dialog" aria-modal="true" aria-labelledby="pool-dialog-title" tabIndex={-1} onKeyDown={(event) => trapTab(event, ref.current)}><form onSubmit={onSubmit}><header><h2 id="pool-dialog-title">{title}</h2></header><div className="dialog-fields">
    <label>名称<input value={draft.name} maxLength={80} onChange={(event) => updateDraft({ ...draft, name: event.target.value })} /></label>
    <label className="checkbox-line"><input type="checkbox" checked={draft.enabled} onChange={(event) => updateDraft({ ...draft, enabled: event.target.checked })} />启用出口组</label>
    <fieldset><legend>订阅来源</legend>{providers.map((provider) => <label className="checkbox-line" key={provider.id}><input type="checkbox" checked={draft.providerIds.includes(provider.id)} onChange={(event) => updateDraft({ ...draft, providerIds: event.target.checked ? [...draft.providerIds, provider.id] : draft.providerIds.filter((id) => id !== provider.id), sourceFilters: event.target.checked && !draft.sourceFilters[provider.id] ? { ...draft.sourceFilters, [provider.id]: blankFilter() } : draft.sourceFilters })} />{provider.name}</label>)}</fieldset>
    {draft.providerIds.map((providerId) => { const provider = providers.find((item) => item.id === providerId); const providerNodes = nodes.filter((node) => node.providerId === providerId); const filter = draft.sourceFilters[providerId] ?? blankFilter(); const patchFilter = (patch: Partial<FilterDraft>) => updateDraft({ ...draft, sourceFilters: { ...draft.sourceFilters, [providerId]: { ...filter, ...patch } } }); return <fieldset key={providerId} className="source-filter"><legend>{provider?.name ?? providerId} · 来源筛选</legend><div className="dialog-grid"><label>节点名称包含词<input value={filter.regions} placeholder="香港, 日本" onChange={(event) => patchFilter({ regions: event.target.value })} /></label><label>包含关键词<input value={filter.includeKeywords} onChange={(event) => patchFilter({ includeKeywords: event.target.value })} /></label><label>排除关键词<input value={filter.excludeKeywords} onChange={(event) => patchFilter({ excludeKeywords: event.target.value })} /></label><label>协议<select multiple value={filter.protocols} onChange={(event) => patchFilter({ protocols: Array.from(event.target.selectedOptions, (option) => option.value as ProxyProtocol) })}>{PROXY_PROTOCOLS.map((protocol) => <option key={protocol}>{protocol}</option>)}</select></label><label>仅包含节点<select multiple value={filter.includeNodeIds} onChange={(event) => patchFilter({ includeNodeIds: Array.from(event.target.selectedOptions, (option) => option.value), excludeNodeIds: filter.excludeNodeIds.filter((id) => !Array.from(event.target.selectedOptions).some((option) => option.value === id)) })}>{providerNodes.map((node) => <option key={node.id} value={node.id}>{node.name}</option>)}</select></label><label>排除节点<select multiple value={filter.excludeNodeIds} onChange={(event) => patchFilter({ excludeNodeIds: Array.from(event.target.selectedOptions, (option) => option.value), includeNodeIds: filter.includeNodeIds.filter((id) => !Array.from(event.target.selectedOptions).some((option) => option.value === id)) })}>{providerNodes.map((node) => <option key={node.id} value={node.id}>{node.name}</option>)}</select></label></div></fieldset>; })}
    <fieldset><legend>选择策略</legend><label className="checkbox-line"><input type="radio" checked={draft.mode === "manual"} onChange={() => setDraft({ ...draft, mode: "manual" })} />手动选择</label><label className="checkbox-line"><input type="radio" checked={draft.mode === "urlTest"} onChange={() => setDraft({ ...draft, mode: "urlTest" })} />自动测试</label></fieldset>
    {draft.mode === "manual" ? <label>初始节点<select value={draft.selectedNodeId} onChange={(event) => updateDraft({ ...draft, selectedNodeId: event.target.value })}><option value="">稍后选择</option>{availableNodes.map((node) => <option key={node.id} value={node.id}>{node.name}</option>)}</select></label> : <div className="dialog-grid"><label>探测 URL<input value={draft.probeUrl} onChange={(event) => updateDraft({ ...draft, probeUrl: event.target.value })} /></label><label>间隔（秒）<input type="number" min="1" max="86400" value={draft.intervalSeconds} onChange={(event) => updateDraft({ ...draft, intervalSeconds: event.target.value })} /></label><label>容差（毫秒）<input type="number" min="0" max="60000" value={draft.toleranceMs} onChange={(event) => updateDraft({ ...draft, toleranceMs: event.target.value })} /></label></div>}
    {error ? <p className="dialog-error" role="alert">{error}</p> : null}</div><footer><button className="secondary-button" type="button" disabled={busy} onClick={onClose}>取消</button><button className="primary-button" type="submit" disabled={busy}>{busy ? "保存中…" : "保存"}</button></footer></form></div></div>;
}
