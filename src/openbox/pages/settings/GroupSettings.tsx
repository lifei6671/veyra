import { useEffect, useState } from "react";
import { PlusIcon, TrashIcon } from "@heroicons/react/24/outline";
import { api } from "../../api/client";
import type { GroupsResponse, OpenBoxGroup } from "../../api/types";
import { EmptyState, ErrorState, IconButton } from "../../components/shared";
import { SelectControl, SwitchControl } from "../../ui/controls";

export function GroupSettings({ onToast }: { onToast: (message: string) => void }) {
  const [data, setData] = useState<GroupsResponse | null>(null);
  const [groups, setGroups] = useState<OpenBoxGroup[]>([]);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);

  const load = async () => {
    setError(null);
    try { const value = await api.groups(); setData(value); setGroups(value.groups); } catch (reason) { setError(reason); }
  };
  useEffect(() => { void load(); }, []);

  const update = (id: string, patch: Partial<OpenBoxGroup>) => setGroups(current => current.map(group => group.id === id ? { ...group, ...patch } : group));
  const add = () => setGroups(current => [...current, { id: crypto.randomUUID(), name: `自定义组 ${current.length + 1}`, type: "selector", mode: "dynamic", enabled: true, icon: "globe:earth-asia", iconScale: 0, keywords: [], members: [] }]);
  const save = async () => {
    setBusy(true);
    try { const value = await api.saveGroups(groups); setData(value); setGroups(value.groups); onToast("出站节点设置已保存"); }
    catch (reason) { onToast(reason instanceof Error ? reason.message : "保存失败"); }
    finally { setBusy(false); }
  };

  return <section className="surface settings-list-block"><header><div><h2>出站节点</h2><p>编辑后端管理的选择器、自动测速组和故障转移组。</p></div><button className="compact-button" type="button" onClick={add}><PlusIcon />新增分组</button></header>
    {Boolean(error) && <ErrorState title="出站节点加载失败" error={error} onRetry={() => void load()} />}
    <div className="group-editor-list">{groups.map(group => {
      const builtin = Boolean(group.kind) || ["all-auto", "all-manual"].includes(group.id);
      return <article className="group-editor-row" key={group.id}><SwitchControl label={`启用 ${group.name}`} checked={group.enabled} onCheckedChange={enabled => update(group.id, { enabled })} /><input aria-label="分组名称" value={group.name} disabled={Boolean(group.kind)} onChange={event => update(group.id, { name: event.target.value })} /><SelectControl label={`${group.name} 类型`} value={group.type} onValueChange={type => update(group.id, { type })} options={(data?.types ?? ["selector", "urltest", "failover"]).map(value => ({ value, label: value }))} /><input aria-label={`${group.name} 关键词`} value={group.keywords.join(" ")} onChange={event => update(group.id, { keywords: event.target.value.split(/\s+/).filter(Boolean) })} placeholder="节点关键词" /><IconButton label={`删除 ${group.name}`} disabled={builtin} onClick={() => setGroups(current => current.filter(item => item.id !== group.id))}><TrashIcon /></IconButton></article>;
    })}</div>
    {!groups.length && !error && <EmptyState icon="◇" title="没有出站分组" text="新增分组后保存到后端。" compact />}
    <div className="settings-footer-actions"><button className="primary-button" type="button" disabled={busy || !groups.length} onClick={() => void save()}>{busy ? "保存中…" : "保存出站节点"}</button></div>
  </section>;
}
