import { ArrowTopRightOnSquareIcon, BoltIcon, LinkIcon, PencilSquareIcon, PlusIcon, PowerIcon, TrashIcon } from "@heroicons/react/24/outline";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { api } from "../../api/client";
import type { ChainProxy, GroupsResponse, OpenBoxProfile, ProxiesResponse, StorageResponse } from "../../api/types";
import { OutboundPicker } from "../../components/OutboundPicker";
import { ErrorState, IconButton, Modal, Tooltip } from "../../components/shared";
import { SortableList } from "../../components/SortableList";
import { latestLatency } from "../../lib/format";
import { SelectControl } from "../../ui/controls";
import { chainConnection, chainDraft, chainIpDetailsUrl, chainLatencyRequest, chainLocation, chainTestResult, chainUpstreamOptions, saveChainDraft, type ChainDraft, type ChainTestResult } from "./ChainProxySettings.helpers";

const LINK_HINT = "这个节点不直接连,而是经下面选的上游去连它。服务器填域名也可以,域名由上游那头解析。住宅代理商给的 http:// https:// 或 主机:端口:用户名:密码 都按 HTTP 代理接。";
const FIELDS_HINT = "住宅 / 静态 IP 代理商表格里的账号照抄进来。协议按他们标的选,没标的一般 HTTP 和 SOCKS5 都行;保存时拼成链接。";
const UPSTREAM_HINT = "上游选一个节点或节点组。保存后它会出现在节点候选里（来源「链式代理」）:把它加进节点组的成员,或在前置自定义分流 / 终端分流里直接选它当出站;按关键词 / 全部节点选成员的动态节点组不会自动收它。保存后重启内核生效。";
const TEST_HINT = "测的是框里填的这一份,不用先保存:经上游此刻选中的节点真的发一次请求(和订阅预览里的节点测速同一个做法,延迟含一次内核启动,会比列表里的高一些)。测速地址、超时和 IP 信息接口都用面板设置里的。";
const NOT_IN_KERNEL = "这条还没进内核(停用中,或保存后还没重启内核):延迟是经这条链单独发一次请求测的,含一次内核启动,会偏高";

function delayClass(ms: number, storage: StorageResponse) {
  return ms < Number(storage.entries["config/low-latency"] || 500) ? "chain-delay-fast" : ms < Number(storage.entries["config/medium-latency"] || 1000) ? "chain-delay-medium" : "chain-delay-slow";
}

function ChainProxyEditor({ item, groups, storage, saving, onSave, onClose, onToast }: { item?: ChainProxy; groups: GroupsResponse | null; storage: StorageResponse; saving: boolean; onSave: (item: ChainProxy) => void; onClose: () => void; onToast: (message: string) => void }) {
  const [draft, setDraft] = useState(() => chainDraft(item));
  const [testing, setTesting] = useState(false), [result, setResult] = useState<ChainTestResult | null>(null);
  const revision = useRef(0);
  const provider = storage.entries["config/geoip-info-api"] || "ip.sb";
  const preferences = storage.entries["config/language"];
  useEffect(() => { revision.current++; setResult(null); }, [preferences]);
  useEffect(() => () => { revision.current++; }, []);
  const patch = (value: Partial<ChainDraft>) => { revision.current++; setResult(null); setDraft(current => ({ ...current, ...value })); };
  const patchFields = (value: Partial<ChainDraft["fields"]>) => patch({ fields: { ...draft.fields, ...value } });
  const save = () => {
    try { onSave(saveChainDraft(draft)); } catch (reason) { onToast((reason as Error).message); }
  };
  const test = async () => {
    let connection: { link: string; upstream: string };
    try { connection = chainConnection(draft); } catch (reason) { onToast((reason as Error).message); return; }
    const current = ++revision.current;
    setTesting(true); setResult(null);
    try { const next = chainTestResult(await api.testChainProxy(chainLatencyRequest(connection, storage))); if (current === revision.current) setResult(next); }
    catch (reason) { if (current === revision.current) setResult({ ok: false, error: reason instanceof Error ? reason.message : String(reason) }); }
    finally { setTesting(false); }
  };
  return <Modal title={item ? "编辑链式代理" : "添加链式代理"} className="routing-dialog chain-proxy-dialog" onClose={onClose} footer={<><button type="button" className="compact-button" onClick={onClose}>取消</button><button type="button" className="primary-button" disabled={saving} onClick={save}>{saving && <span className="loading-spinner" />}保存</button></>}>
    <div className="chain-proxy-form">
      <label className="chain-proxy-field"><span>名称</span><input aria-label="名称" placeholder="例如:住宅-美国" value={draft.name} onChange={event => setDraft(current => ({ ...current, name: event.target.value }))} /></label>
      <div className="chain-proxy-field">
        <div className="chain-node-header"><label htmlFor="chain-node-link">节点</label><div role="tablist" aria-label="节点输入方式">{(["link", "fields"] as const).map(mode => <button type="button" role="tab" key={mode} aria-selected={draft.mode === mode} onClick={() => patch({ mode })}>{mode === "link" ? "链接" : "字段"}</button>)}</div></div>
        {draft.mode === "link" ? <><textarea id="chain-node-link" aria-label="节点链接或配置" rows={4} placeholder="粘贴一条分享链接（socks5:// ss:// vless:// trojan:// 等）、HTTP 代理（http:// https:// 或住宅代理的 主机:端口:用户名:密码）,或一段 Clash / sing-box 节点配置" value={draft.link} onChange={event => patch({ link: event.target.value })} /><p className="chain-hint">{LINK_HINT}</p></> : <>
          <div className="chain-address-fields"><SelectControl label="代理协议" className="chain-scheme-select" value={draft.fields.scheme} onValueChange={scheme => patchFields({ scheme })} options={[{ value: "http", label: "HTTP" }, { value: "https", label: "HTTPS" }, { value: "socks5", label: "SOCKS5" }]} /><input aria-label="IP / 域名" placeholder="IP / 域名" value={draft.fields.host} onChange={event => patchFields({ host: event.target.value })} /><input aria-label="端口" placeholder="端口" inputMode="numeric" value={draft.fields.port} onChange={event => patchFields({ port: event.target.value })} /></div>
          <div className="chain-credential-fields"><input aria-label="用户名" placeholder="用户名" autoComplete="off" value={draft.fields.username} onChange={event => patchFields({ username: event.target.value })} /><input aria-label="密码" placeholder="密码" autoComplete="off" value={draft.fields.password} onChange={event => patchFields({ password: event.target.value })} /></div><p className="chain-hint">{FIELDS_HINT}</p>
        </>}
      </div>
      <div className="chain-proxy-field chain-upstream-field"><span>上游</span><OutboundPicker groups={chainUpstreamOptions(groups, item)} value={draft.upstream} testUrl={storage.entries["config/speedtest-url"] || "http://www.gstatic.com/generate_204"} onChange={upstream => patch({ upstream })} onToast={onToast} /><p className="chain-hint">{UPSTREAM_HINT}</p></div>
      <div className="chain-proxy-field"><div className="chain-test-row"><button type="button" className="compact-button chain-test-button" disabled={testing} onClick={() => void test()}>{testing ? <span className="loading-spinner" /> : <BoltIcon />}测速</button>
        {result && !testing && <>{result.ok ? <span className={`chain-test-delay ${delayClass(result.ms ?? 0, storage)}`}>{result.ms} ms</span> : <span className="chain-test-error" role="alert">{result.error}</span>}{result.via && <span className="chain-hint">经 {result.via}</span>}{result.ip && <Tooltip label={`在 ${provider} 查看这个出口 IP(新页签)`}><a className="chain-ip-details" href={chainIpDetailsUrl(result.ip.ip, provider)} target="_blank" rel="noopener noreferrer">IP 详情<ArrowTopRightOnSquareIcon /></a></Tooltip>}</>}
      </div>
        {result && !testing && (result.ip ? <div className="chain-ip-result"><div><span>出口 IP</span><code>{result.ip.ip}</code>{result.ip.asn && <span>(AS{result.ip.asn})</span>}</div>{chainLocation(result.ip) && <div>{chainLocation(result.ip)}</div>}{result.ip.organization && <div className="chain-hint">{result.ip.organization}</div>}</div> : result.ipError && <p className="chain-test-error">IP 信息没取到:{result.ipError}</p>)}
        <p className="chain-hint">{TEST_HINT}</p>
      </div>
    </div>
  </Modal>;
}

export function ChainProxySettings({ storage, onToast }: { storage: StorageResponse; onToast: (message: string) => void }) {
  const [profile, setProfile] = useState<OpenBoxProfile | null>(null), [groups, setGroups] = useState<GroupsResponse | null>(null), [proxies, setProxies] = useState<ProxiesResponse | null>(null);
  const [items, setItems] = useState<ChainProxy[]>([]), [error, setError] = useState<unknown>(null), [busy, setBusy] = useState(false);
  const [editor, setEditor] = useState<{ item?: ChainProxy } | null>(null), [deleting, setDeleting] = useState<ChainProxy | null>(null), [actions, setActions] = useState<HTMLElement | null>(null);
  const [testing, setTesting] = useState<string[]>([]), [results, setResults] = useState<Record<string, ChainTestResult>>({});
  const preferences = storage.entries["config/language"];
  useEffect(() => { setResults({}); }, [preferences]);
  const load = async () => {
    setError(null);
    try { const [next, exits] = await Promise.all([api.profile(), api.groups()]); setProfile(next); setItems(next.chainProxies); setGroups(exits); }
    catch (reason) { setError(reason); }
    void api.proxies().then(setProxies).catch(() => {});
  };
  useEffect(() => { setActions(document.getElementById("settings-header-actions")); void load(); }, []);
  useEffect(() => {
    if (!editor && !deleting) return;
    const escape = (event: KeyboardEvent) => { if (event.key === "Escape" && !event.defaultPrevented && !(event.target as Element).closest('[data-slot="select-content"]') && !document.querySelector('[data-slot="select-content"]')) { setEditor(null); setDeleting(null); } };
    window.addEventListener("keydown", escape);
    return () => window.removeEventListener("keydown", escape);
  }, [!!editor, !!deleting]);
  const persist = async (next: ChainProxy[]) => {
    setBusy(true);
    let saved = false;
    try {
      const response = await api.saveProfile({ chainProxies: next });
      setProfile(response); setItems(response.chainProxies);
      saved = true;
      setGroups(await api.groups());
      onToast("已保存;重启内核后生效。");
      return true;
    } catch (reason) { if (!saved) setItems(profile!.chainProxies); onToast(`${saved ? "已保存，节点候选刷新失败:" : ""}${reason instanceof Error ? reason.message : "保存失败"}`); return saved; }
    finally { setBusy(false); }
  };
  const save = async (item: ChainProxy) => {
    setResults(current => { const next = { ...current }; delete next[item.id]; return next; });
    const next = items.some(current => current.id === item.id) ? items.map(current => current.id === item.id ? item : current) : [...items, item];
    if (await persist(next)) setEditor(null);
  };
  const inKernel = (item: ChainProxy) => item.enabled !== false && !!proxies?.proxies[item.name];
  const test = async (item: ChainProxy) => {
    if (testing.includes(item.id)) return;
    setTesting(current => [...current, item.id]);
    try {
      const [response, core] = await Promise.all([api.testChainProxy(chainLatencyRequest(item, storage)), inKernel(item) ? api.testProxy(item.name, storage.entries["config/speedtest-url"] || "http://www.gstatic.com/generate_204", Number(storage.entries["config/speedtest-timeout"] || 5000)).catch(() => null) : Promise.resolve(null)]);
      setResults(current => ({ ...current, [item.id]: chainTestResult(response) }));
      if (core) setProxies(current => current && { ...current, proxies: { ...current.proxies, [item.name]: { ...current.proxies[item.name], history: [...current.proxies[item.name].history ?? [], { time: new Date().toISOString(), delay: core.delay }] } } });
    } catch (reason) { setResults(current => ({ ...current, [item.id]: { ok: false, error: reason instanceof Error ? reason.message : String(reason) } })); }
    finally { setTesting(current => current.filter(id => id !== item.id)); }
  };
  const upstreams = new Set([...(groups?.groups ?? []).filter(group => !group.kind && group.enabled !== false).map(group => group.name), ...(groups?.availableNodes ?? []).map(node => node.name)]);
  if (error) return <ErrorState title="链式代理加载失败" error={error} onRetry={() => void load()} />;
  if (!profile) return <div className="settings-loading"><span className="loading-spinner" /></div>;
  return <>
    {actions && createPortal(<IconButton label="添加链式代理" tooltip portalTooltip className="add-button chain-add-button" disabled={busy} onClick={() => setEditor({})}><PlusIcon /></IconButton>, actions)}
    <div className="routing-policy-stack chain-proxy-settings" data-dialog-open={!!editor || !!deleting}>
      {!items.length && <div className="chain-proxy-empty"><LinkIcon /><p>没有住宅 IP？去 <a href="https://blog.angeworld.cc/market" target="_blank" rel="noopener noreferrer">安格超市</a> 买一个</p><button type="button" className="primary-button" onClick={() => setEditor({})}><PlusIcon />添加链式代理</button></div>}
      <SortableList items={items} itemKey={item => item.id} disabled={busy} onChange={setItems} onEnd={next => void persist(next)} renderItem={(item, handle) => {
        const result = results[item.id], pending = testing.includes(item.id), delay = inKernel(item) ? latestLatency(proxies?.proxies[item.name].history) ?? 0 : result?.ok ? result.ms ?? 0 : 0;
        return <article className={`routing-policy-card chain-proxy-card${item.enabled === false ? " routing-disabled" : ""}`}>{handle}<div className="routing-policy-copy"><div className="routing-policy-title"><h3>{item.name}</h3><span className="chain-via-badge">经 {item.upstream}</span>
          {result && !pending && <>{delay > 0 ? <Tooltip label={inKernel(item) ? "" : NOT_IN_KERNEL} enabled={!inKernel(item)}><span className={`chain-card-delay ${delayClass(delay, storage)}`}>{delay} ms</span></Tooltip> : <Tooltip label={result.error || ""}><span className="chain-test-error">不通</span></Tooltip>}{result.ip ? <Tooltip label={`出口 IP ${result.ip.ip}${result.ip.asn ? ` (AS${result.ip.asn})` : ""}`}><span className="chain-card-location">{chainLocation(result.ip)}{result.ip.organization && ` · ${result.ip.organization}`}</span></Tooltip> : result.ipError && <Tooltip label={result.ipError}><span className="chain-ip-unknown">IP 归属地没查到</span></Tooltip>}</>}
          {item.enabled === false && <span className="routing-disabled-badge">已停用</span>}{(item.upstream === item.name || !upstreams.has(item.upstream)) && <small className="chain-upstream-missing">上游「{item.upstream}」已不存在</small>}
        </div><p>{item.node ? `${item.node.type} · ${item.node.server}:${item.node.port}` : ""}</p></div>
          <div className="routing-policy-actions"><IconButton label={item.enabled === false ? "当前已停用,点击启用" : "当前已启用,点击停用"} tooltip portalTooltip className={item.enabled === false ? "" : "is-enabled"} disabled={busy} onClick={() => void persist(items.map(current => current.id === item.id ? { ...current, enabled: current.enabled === false } : current))}><PowerIcon /></IconButton><IconButton label="测速" aria-label={`测速链式代理 ${item.name}`} tooltip portalTooltip disabled={pending} onClick={() => void test(item)}>{pending ? <span className="loading-spinner" /> : <BoltIcon />}</IconButton><IconButton label="编辑" aria-label={`编辑链式代理 ${item.name}`} tooltip portalTooltip disabled={busy} onClick={() => setEditor({ item })}><PencilSquareIcon /></IconButton><IconButton label="删除" aria-label={`删除链式代理 ${item.name}`} tooltip portalTooltip disabled={busy} onClick={() => setDeleting(item)}><TrashIcon /></IconButton></div>
        </article>;
      }} />
    </div>
    {editor && <ChainProxyEditor item={editor.item} groups={groups} storage={storage} saving={busy} onSave={item => void save(item)} onClose={() => setEditor(null)} onToast={onToast} />}
    {deleting && <Modal title="确认" className="routing-dialog routing-confirm-modal" onClose={() => setDeleting(null)} footer={<><button type="button" className="compact-button" onClick={() => setDeleting(null)}>取消</button><button type="button" className="danger-button" disabled={busy} onClick={() => void persist(items.filter(item => item.id !== deleting.id)).then(success => { if (success) setDeleting(null); })}>确定</button></>}><p>确定删除链式代理「{deleting.name}」吗？</p></Modal>}
  </>;
}
