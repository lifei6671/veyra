import { ArrowPathIcon, ArrowUturnLeftIcon, ClipboardDocumentIcon, PencilSquareIcon, PlusIcon, PowerIcon, QrCodeIcon, ShareIcon, TrashIcon } from "@heroicons/react/24/outline";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { QRCodeSVG } from "qrcode.react";
import { api } from "../../api/client";
import type { SharedServer } from "../../api/types";
import { ErrorState, IconButton, Modal } from "../../components/shared";
import { SortableList } from "../../components/SortableList";
import { SelectControl, SwitchControl } from "../../ui/controls";
import { changeServerProtocol, generateServerPassword, SERVER_METHODS, SERVER_PROTOCOLS, serverDraft, serverHasLocalAddress, serverPortError, serverShareLink, saveServerDraft } from "./ShareNetworkSettings.helpers";

const TLS_HINT = "使用自签证书,分享链接里已带上允许不安全证书。";
const PLAIN_HINT = "不套 TLS:客户端到路由器之间只有协议本身的加密。";
const MIXED_HINT = "SOCKS5 和 HTTP 代理共用这个端口,局域网设备把代理地址填成 路由器IP:端口 就能用。只对局域网开放,不在 WAN 放行。";
const NEED_ADDRESS = "填了连接地址才会生成分享链接";

function ShareLink({ value, onCopy }: { value: string; onCopy: (value: string) => void }) {
  return <div className="server-join server-share-link"><input aria-label="分享链接" value={value || NEED_ADDRESS} readOnly spellCheck={false} /><IconButton label="复制链接" tooltip portalTooltip className="server-join-button" disabled={!value} onClick={() => onCopy(value)}><ClipboardDocumentIcon /></IconButton></div>;
}

function ServerEditor({ server, usedPorts, saving, onSave, onClose, onToast, onCopy }: { server?: SharedServer; usedPorts: number[]; saving: boolean; onSave: (server: SharedServer) => void; onClose: () => void; onToast: (message: string) => void; onCopy: (value: string) => void }) {
  const [draft, setDraft] = useState(() => serverDraft(server));
  const [checking, setChecking] = useState(false);
  const active = useRef(true);
  useEffect(() => { active.current = true; return () => { active.current = false; }; }, []);
  const patch = (value: Partial<typeof draft>) => setDraft(current => ({ ...current, ...value }));
  const link = serverShareLink(draft), protocol = draft.protocol;
  const tls = protocol === "tuic" || protocol === "hysteria2" || protocol === "vless" && draft.tls;
  const save = async () => {
    let value: SharedServer;
    try { value = saveServerDraft(draft, usedPorts); } catch (reason) { onToast((reason as Error).message); return; }
    setChecking(true);
    try {
      const message = serverPortError(await api.checkServerPort(value.port, value.id), value.port);
      if (!active.current) return;
      if (message) { onToast(message); return; }
      onSave(value);
    } catch (reason) { if (active.current) onToast(`端口检测失败:${reason instanceof Error ? reason.message : String(reason)}`); }
    finally { if (active.current) setChecking(false); }
  };
  const credential = (label: "UUID" | "密码" | "混淆密码", key: "uuid" | "password" | "obfs", generate: () => string, placeholder = "") => <label className={`server-field${protocol === "vless" && key === "uuid" ? " server-field-wide" : ""}`}><span>{label}</span><div className="server-join"><input aria-label={label} autoComplete="off" spellCheck={false} className="server-credential-input" placeholder={placeholder} value={draft[key] || ""} onChange={event => patch({ [key]: event.target.value })} /><IconButton label="随机生成" aria-label={`随机生成${label}`} tooltip portalTooltip className="server-join-button" onClick={() => patch({ [key]: generate() })}><ArrowPathIcon /></IconButton></div></label>;
  return <Modal title={server ? "编辑服务器" : "添加服务器"} className="routing-dialog server-edit-dialog" onClose={onClose} footer={<><button type="button" className="compact-button" onClick={onClose}>取消</button><button type="button" className="primary-button" disabled={checking || saving} onClick={() => void save()}>{(checking || saving) && <span className="loading-spinner" />}保存</button></>}>
    <div className="server-form"><div className="server-field-grid">
      <label className="server-field"><span>备注</span><input aria-label="备注" placeholder="例如:家里" value={draft.name} onChange={event => patch({ name: event.target.value })} /></label>
      <div className="server-field"><span>协议</span><SelectControl label="协议" value={protocol} options={SERVER_PROTOCOLS} onValueChange={value => setDraft(current => changeServerProtocol(current, value as SharedServer["protocol"]))} /></div>
      <label className="server-field"><span>域名 / IP</span><input aria-label="域名 / IP" placeholder="域名或 IP" autoComplete="off" value={draft.address} onChange={event => patch({ address: event.target.value })} /></label>
      <label className="server-field"><span>端口</span><input aria-label="端口" type="number" min={1} max={65535} value={draft.port} onChange={event => patch({ port: event.target.value })} /></label>
      {protocol === "shadowsocks" && <div className="server-field"><span>加密</span><SelectControl label="加密" value={draft.method} options={SERVER_METHODS.map(value => ({ value, label: value }))} onValueChange={method => patch({ method, password: generateServerPassword(method) })} /></div>}
      {(protocol === "vless" || protocol === "tuic") && credential("UUID", "uuid", () => crypto.randomUUID())}
      {protocol === "mixed" && <label className="server-field"><span>用户名</span><input aria-label="用户名" className="server-credential-input" placeholder="可选,留空则不需要认证" autoComplete="off" value={draft.username || ""} onChange={event => patch({ username: event.target.value })} /></label>}
      {protocol !== "vless" && credential("密码", "password", () => generateServerPassword(protocol === "shadowsocks" ? draft.method : ""), protocol === "mixed" ? "可选,留空则不需要认证" : "")}
      {protocol === "hysteria2" && credential("混淆密码", "obfs", () => generateServerPassword("", 12), "可选（salamander）")}
      {protocol === "vless" && <label className="server-tls-field"><SwitchControl label="TLS（自签证书）" checked={draft.tls !== false} onCheckedChange={value => patch({ tls: value })} /><span>TLS（自签证书）</span></label>}
    </div><p className="server-hint">{tls ? TLS_HINT : protocol === "mixed" ? MIXED_HINT : PLAIN_HINT}</p>
      <div className="server-field"><span>分享链接</span><ShareLink value={link} onCopy={onCopy} /></div>
      {link && <QRCodeSVG className="server-edit-qr" value={link} size={352} marginSize={1} level="M" boostLevel={false} aria-label="服务器分享二维码" />}
    </div>
  </Modal>;
}

function ShareCodeDialog({ server, onClose, onCopy }: { server: SharedServer; onClose: () => void; onCopy: (value: string) => void }) {
  const link = serverShareLink(server);
  return <Modal title={`扫码连接「${server.name}」`} className="routing-dialog server-code-dialog" onClose={onClose}><div className="server-code-content"><div role="tablist" aria-label="扫码客户端"><button type="button" role="tab" aria-disabled="true" disabled>Open-Box App <small>· 即将开放</small></button><button type="button" role="tab" aria-selected="true">其他客户端</button></div>
    {link && <QRCodeSVG className="server-code-qr" value={link} size={352} marginSize={1} level="M" boostLevel={false} aria-label="服务器连接二维码" />}
    <p className="server-hint">普通节点链接,给其他客户端用。</p><ShareLink value={link} onCopy={onCopy} />
    {server.enabled === false && <p className="server-warning">这台服务器已停用,扫了也连不上。</p>}
    {serverHasLocalAddress(server.address) && <p className="server-warning">连接地址 {server.address.trim()} 只在家里能用,在外面连不上;要填公网 IP 或域名。</p>}
  </div></Modal>;
}

export function ShareNetworkSettings({ onToast }: { onToast: (message: string) => void }) {
  const [items, setItems] = useState<SharedServer[]>([]), [loading, setLoading] = useState(true), [error, setError] = useState<unknown>(null), [busy, setBusy] = useState(false);
  const saved = useRef<SharedServer[]>([]);
  const [editor, setEditor] = useState<{ server?: SharedServer } | null>(null), [deleting, setDeleting] = useState<SharedServer | null>(null), [sharing, setSharing] = useState<SharedServer | null>(null), [actions, setActions] = useState<HTMLElement | null>(null);
  const load = async () => { setLoading(true); setError(null); try { const next = await api.profile(); saved.current = next.servers || []; setItems(saved.current); } catch (reason) { setError(reason); } finally { setLoading(false); } };
  useEffect(() => { setActions(document.getElementById("settings-header-actions")); void load(); }, []);
  useEffect(() => {
    if (!editor && !deleting && !sharing) return;
    const escape = (event: KeyboardEvent) => { if (event.key === "Escape" && !event.defaultPrevented && !(event.target as Element).closest('[data-slot="select-content"]') && !document.querySelector('[data-slot="select-content"]')) { setEditor(null); setDeleting(null); setSharing(null); } };
    window.addEventListener("keydown", escape); return () => window.removeEventListener("keydown", escape);
  }, [!!editor, !!deleting, !!sharing]);
  const persist = async (next: SharedServer[]) => {
    setBusy(true);
    try { const profile = await api.saveProfile({ servers: next }); saved.current = profile.servers || []; setItems(saved.current); onToast("已保存;重启内核后生效。"); return true; }
    catch (reason) { setItems(saved.current); onToast(reason instanceof Error ? reason.message : "保存失败"); return false; }
    finally { setBusy(false); }
  };
  const save = async (server: SharedServer) => { if (await persist(items.some(item => item.id === server.id) ? items.map(item => item.id === server.id ? server : item) : [...items, server])) setEditor(null); };
  const copy = (value: string) => { if (value) void navigator.clipboard.writeText(value).then(() => onToast("复制成功"), () => onToast("复制失败")); };
  const modalOpen = !!editor || !!deleting || !!sharing;
  return <>
    {actions && createPortal(<IconButton label="添加服务器" tooltip portalTooltip className="add-button server-add-button" disabled={busy} onClick={() => setEditor({})}><PlusIcon /></IconButton>, actions)}
    <div className="routing-policy-stack share-network-settings" data-dialog-open={modalOpen}>
      {error ? <ErrorState title="共享网络加载失败" error={error} onRetry={() => void load()} /> : loading ? <div className="surface server-loading"><span className="loading-spinner" /></div> : <>
        {!items.length && <div className="server-empty"><ShareIcon /><p>还没有服务器。</p><button type="button" className="primary-button" onClick={() => setEditor({})}>添加服务器</button></div>}
        <SortableList items={items} itemKey={item => item.id} disabled={busy} onChange={setItems} onEnd={next => void persist(next)} renderItem={(item, handle) => {
          const link = serverShareLink(item);
          return <article className="routing-policy-card server-card">{handle}<div className="routing-policy-copy"><div className="routing-policy-title"><h3>{item.name}</h3>{item.enabled === false && <span className="routing-disabled-badge">已停用</span>}</div><div className="server-card-detail"><span>{SERVER_PROTOCOLS.find(protocol => protocol.value === item.protocol)?.label || item.protocol}</span><span>端口 {item.port}</span>{item.address ? <span>{item.address}</span> : <span className="server-warning">没填连接地址,无法生成分享链接</span>}</div></div>
            <div className="routing-policy-actions"><IconButton label="扫码" aria-label={`扫码连接 ${item.name}`} tooltip portalTooltip disabled={busy || !link} onClick={() => setSharing(item)}><QrCodeIcon /></IconButton><IconButton label="复制链接" aria-label={`复制链接 ${item.name}`} tooltip portalTooltip disabled={!link} onClick={() => copy(link)}><ClipboardDocumentIcon /></IconButton><IconButton label={item.enabled === false ? "当前已停用,点击启用" : "当前已启用,点击停用"} tooltip portalTooltip className={item.enabled === false ? "" : "is-enabled"} disabled={busy} onClick={() => void persist(items.map(server => server.id === item.id ? { ...server, enabled: server.enabled === false } : server))}><PowerIcon /></IconButton><IconButton label="编辑" aria-label={`编辑服务器 ${item.name}`} tooltip portalTooltip disabled={busy} onClick={() => setEditor({ server: item })}><PencilSquareIcon /></IconButton><IconButton label="删除" aria-label={`删除服务器 ${item.name}`} tooltip portalTooltip disabled={busy} onClick={() => setDeleting(item)}><TrashIcon /></IconButton></div>
          </article>;
        }} />
        {!!items.length && <section className="surface server-regions-card"><header><h2>地区分流 <span>即将开放</span></h2><div><IconButton label="恢复默认" tooltip portalTooltip disabled><ArrowUturnLeftIcon /></IconButton><IconButton label="添加地区组" tooltip portalTooltip className="add-button" disabled><PlusIcon /></IconButton></div></header><p>Open-Box App 按所在地区选一组:命中规则的按规则走,其余按这一组的「其余流量」走。</p></section>}
      </>}
    </div>
    {editor && <ServerEditor server={editor.server} usedPorts={items.filter(item => item.id !== editor.server?.id).map(item => item.port)} saving={busy} onSave={server => void save(server)} onClose={() => setEditor(null)} onToast={onToast} onCopy={copy} />}
    {sharing && <ShareCodeDialog server={sharing} onClose={() => setSharing(null)} onCopy={copy} />}
    {deleting && <Modal title="确认" className="routing-dialog routing-confirm-modal" onClose={() => setDeleting(null)} footer={<><button type="button" className="compact-button" onClick={() => setDeleting(null)}>取消</button><button type="button" className="danger-button" disabled={busy} onClick={() => void persist(items.filter(item => item.id !== deleting.id)).then(success => { if (success) setDeleting(null); })}>确定</button></>}><p>确定删除共享服务器「{deleting.name}」吗？使用它的客户端将无法连接。</p></Modal>}
  </>;
}
