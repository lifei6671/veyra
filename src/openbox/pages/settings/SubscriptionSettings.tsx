import { useEffect, useState } from "react";
import { ArrowPathIcon, LinkIcon, PencilSquareIcon, PlusIcon, TrashIcon } from "@heroicons/react/24/outline";
import { api } from "../../api/client";
import type { Subscription, SubscriptionShare } from "../../api/types";
import { formatDateTime } from "../../lib/format";
import { EmptyState, ErrorState, IconButton, Modal } from "../../components/shared";

type FormValue = { id?: string; name: string; url: string };

export function SubscriptionSettings({ onToast, addToken = 0 }: { onToast: (message: string) => void; addToken?: number }) {
  const [items, setItems] = useState<Subscription[]>([]);
  const [shares, setShares] = useState<SubscriptionShare[]>([]);
  const [form, setForm] = useState<FormValue | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState("");

  const load = async () => {
    setError(null);
    try {
      const [subscriptions, shareItems] = await Promise.all([api.subscriptions(), api.subscriptionShares()]);
      setItems(subscriptions);
      setShares(shareItems);
    } catch (reason) { setError(reason); }
  };
  useEffect(() => { void load(); }, []);
  useEffect(() => { if (addToken > 0) setForm({ name: "", url: "" }); }, [addToken]);

  const save = async () => {
    if (!form?.name.trim() || !form.url.trim()) return;
    setBusy("save");
    try {
      const payload = { name: form.name.trim(), url: form.url.trim(), urls: [form.url.trim()], format: "auto" };
      if (form.id) await api.updateSubscription(form.id, payload); else await api.createSubscription(payload);
      setForm(null);
      await load();
      onToast(form.id ? "订阅已更新，重启内核后生效" : "订阅已添加");
    } catch (reason) { onToast(reason instanceof Error ? reason.message : "保存订阅失败"); } finally { setBusy(""); }
  };

  const refresh = async (item: Subscription) => {
    setBusy(item.id);
    try { await api.refreshSubscription(item.id); await load(); onToast(`${item.name} 已刷新`); }
    catch (reason) { onToast(reason instanceof Error ? reason.message : "刷新失败"); }
    finally { setBusy(""); }
  };

  const remove = async (item: Subscription) => {
    if (!window.confirm(`确定删除订阅“${item.name}”吗？`)) return;
    setBusy(item.id);
    try { await api.deleteSubscription(item.id); await load(); onToast("订阅已删除"); }
    catch (reason) { onToast(reason instanceof Error ? reason.message : "删除失败"); }
    finally { setBusy(""); }
  };

  return <div className="settings-content-stack">
    <section className="surface settings-list-block"><header><div><h2>订阅管理</h2><p>订阅保存到 Open-Box 后端，刷新后节点会进入配置生成流程。</p></div><button className="primary-button" type="button" onClick={() => setForm({ name: "", url: "" })}><PlusIcon />添加订阅</button></header>
      {Boolean(error) && <ErrorState title="订阅加载失败" error={error} onRetry={() => void load()} />}
      <div className="settings-card-list">{items.map(item => <article className="settings-data-card" key={item.id}><div><h3>{item.name}</h3><p>{item.format || "auto"} · {item.nodeCount} 个节点</p><small>更新于 {formatDateTime(item.updatedAt)}{item.kernelStale ? " · 等待内核重启应用" : ""}</small></div><div className="card-actions"><IconButton label={`刷新 ${item.name}`} disabled={Boolean(busy)} onClick={() => void refresh(item)}><ArrowPathIcon /></IconButton><IconButton label={`编辑 ${item.name}`} disabled={Boolean(busy)} onClick={() => setForm({ id: item.id, name: item.name, url: item.url ?? item.urls?.[0] ?? "" })}><PencilSquareIcon /></IconButton><IconButton label={`删除 ${item.name}`} disabled={Boolean(busy)} onClick={() => void remove(item)}><TrashIcon /></IconButton></div></article>)}</div>
      {!items.length && !error && <EmptyState icon={<LinkIcon />} title="还没有订阅" text="添加一个订阅地址后即可获取真实节点。" compact />}
    </section>

    <section className="surface settings-list-block"><header><div><h2>订阅分享</h2><p>后端已经生成的分享入口会显示在这里。</p></div></header>{shares.length ? <div className="settings-card-list">{shares.map(share => <article className="settings-data-card" key={share.id}><div><h3>{share.name}</h3><p>{share.enabled === false ? "已停用" : "可用"}</p></div>{share.url && <button type="button" className="compact-button" onClick={() => void navigator.clipboard.writeText(share.url ?? "").then(() => onToast("分享链接已复制"))}>复制链接</button>}</article>)}</div> : <EmptyState icon={<LinkIcon />} title="没有订阅分享" text="当前后端未配置分享链接。" compact />}</section>

    {form && <Modal title={form.id ? "编辑订阅" : "添加订阅"} onClose={() => setForm(null)} footer={<><button type="button" className="compact-button" onClick={() => setForm(null)}>取消</button><button type="button" className="primary-button" disabled={busy === "save" || !form.name.trim() || !form.url.trim()} onClick={() => void save()}>保存</button></>}><div className="form-stack"><label><span>订阅名称</span><input autoFocus value={form.name} onChange={event => setForm(current => current ? { ...current, name: event.target.value } : current)} /></label><label><span>订阅地址</span><input type="url" value={form.url} onChange={event => setForm(current => current ? { ...current, url: event.target.value } : current)} placeholder="https://example.com/subscription" /></label></div></Modal>}
  </div>;
}
