import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ComponentProps, Dispatch, SetStateAction } from "react";
import { QRCodeSVG } from "qrcode.react";
import {
  ArrowPathIcon,
  ArrowPathRoundedSquareIcon,
  ArrowRightIcon,
  ArrowUturnLeftIcon,
  Bars3Icon,
  BoltIcon,
  CheckCircleIcon,
  ChevronDownIcon,
  ChevronUpIcon,
  ClipboardDocumentIcon,
  LinkIcon,
  MagnifyingGlassIcon,
  NoSymbolIcon,
  PencilSquareIcon,
  PlusIcon,
  PowerIcon,
  TrashIcon,
  XMarkIcon,
} from "@heroicons/react/24/outline";
import { SelectControl } from "../../ui/controls";
import { api } from "../../api/client";
import type {
  ControllerProxy,
  GroupsResponse,
  ProxiesResponse,
  ProxyHistory,
  StorageResponse,
  Subscription,
  SubscriptionAutoUpdate,
  SubscriptionNodeLatencyResult,
  SubscriptionNodeDns,
  SubscriptionPreview,
  SubscriptionRenameOptions,
  SubscriptionShare,
} from "../../api/types";
import { NodeHealthDots, ProxyOption, sortProxyNodes } from "../../components/ProxyCards";
import { ErrorState, IconButton, Modal } from "../../components/shared";
import { formatRelativeTime, latestLatency } from "../../lib/format";

type EditorTab = "source" | "rules" | "nodes";
type FormValue = {
  id?: string;
  sourceType: "url" | "paste";
  tab: EditorTab;
  name: string;
  urls: string[];
  content: string;
  autoUpdate: SubscriptionAutoUpdate;
  renameOptions: SubscriptionRenameOptions;
  nodeDns: SubscriptionNodeDns | null;
  nodeDnsExpanded: boolean;
};
type ShareFormValue = {
  id?: string;
  name: string;
  protocol: string;
  host: string;
  subscriptionIds: string[];
  token: string;
};

export function SubscriptionSettings({ storage, onToast, addToken = 0 }: {
  storage: StorageResponse;
  onToast: (message: string) => void;
  addToken?: number;
}) {
  const [items, setItems] = useState<Subscription[]>([]);
  const [shares, setShares] = useState<SubscriptionShare[]>([]);
  const [proxies, setProxies] = useState<ProxiesResponse>({ proxies: {} });
  const [groups, setGroups] = useState<GroupsResponse | null>(null);
  const [latencyHistory, setLatencyHistory] = useState<Record<string, ProxyHistory[]>>({});
  const [form, setForm] = useState<FormValue | null>(null);
  const [shareForm, setShareForm] = useState<ShareFormValue | null>(null);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const [shareCollapsed, setShareCollapsed] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState("");
  const [testingNodes, setTestingNodes] = useState<Set<string>>(() => new Set());
  const [testingSubscriptions, setTestingSubscriptions] = useState<Set<string>>(() => new Set());
  const [preview, setPreview] = useState<SubscriptionPreview | null>(null);
  const [previewError, setPreviewError] = useState("");
  const previewRequest = form && validSubscriptionForm(form)
    ? JSON.stringify(subscriptionPayload(form))
    : "";

  const timeout = finiteSetting(storage.entries["config/speedtest-timeout"], 5000);
  const testUrl = storage.entries["config/speedtest-url"] || "http://www.gstatic.com/generate_204";
  const thresholds = {
    low: finiteSetting(storage.entries["config/low-latency"], 500),
    medium: finiteSetting(storage.entries["config/medium-latency"], 1000),
  };

  const load = async () => {
    setError(null);
    try {
      const [subscriptions, shareItems, proxyItems, groupItems, latency] = await Promise.all([
        api.subscriptions(),
        api.subscriptionShares(),
        api.proxies(),
        api.groups(),
        api.proxyLatencyHistory(),
      ]);
      setItems(subscriptions);
      setShares(shareItems);
      setProxies(proxyItems);
      setGroups(groupItems);
      setLatencyHistory(latency.history);
    } catch (reason) {
      setError(reason);
    } finally {
      setLoaded(true);
    }
  };

  useEffect(() => { void load(); }, []);
  useEffect(() => {
    if (addToken > 0) {
      setPreview(null);
      setPreviewError("");
      setForm(emptyForm());
    }
  }, [addToken]);

  useEffect(() => {
    if (!previewRequest) {
      setPreview(null);
      setPreviewError("");
      return;
    }
    let cancelled = false;
    setPreviewError("");
    const payload = JSON.parse(previewRequest) as Partial<Subscription>;
    const timer = window.setTimeout(() => {
      void api.previewSubscription(payload).then(value => {
        if (!cancelled) setPreview(value);
      }).catch(reason => {
        if (!cancelled) {
          setPreview(null);
          setPreviewError(errorMessage(reason, "预览失败"));
        }
      });
    }, 300);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [previewRequest]);

  const nodesBySubscription = useMemo(() => Object.fromEntries(items.map(item => [
    item.id,
    subscriptionNodesFromApi(item, groups, proxies, latencyHistory),
  ])), [groups, items, latencyHistory, proxies]);

  const save = async () => {
    if (!form || !validSubscriptionForm(form)) return;
    setBusy("subscription:save");
    try {
      const payload = subscriptionPayload(form);
      if (form.id) await api.updateSubscription(form.id, payload); else await api.createSubscription(payload);
      setForm(null);
      await load();
      onToast(form.id ? "订阅已更新，重启内核后生效" : "订阅已添加");
    } catch (reason) {
      onToast(errorMessage(reason, "保存订阅失败"));
    } finally {
      setBusy("");
    }
  };

  const refresh = async (item: Subscription) => {
    setBusy(`subscription:${item.id}`);
    try {
      await api.refreshSubscription(item.id);
      await load();
      onToast(`${item.name} 已刷新`);
    } catch (reason) {
      onToast(errorMessage(reason, "刷新失败"));
    } finally {
      setBusy("");
    }
  };

  const toggleSubscription = async (item: Subscription) => {
    const enabled = item.enabled !== false;
    setBusy(`subscription:${item.id}`);
    try {
      await api.updateSubscription(item.id, { enabled: !enabled });
      await load();
      onToast(`${item.name} 已${enabled ? "停用" : "启用"}`);
    } catch (reason) {
      onToast(errorMessage(reason, "订阅状态更新失败"));
    } finally {
      setBusy("");
    }
  };

  const remove = async (item: Subscription) => {
    if (!window.confirm(`确定删除订阅“${item.name}”吗？`)) return;
    setBusy(`subscription:${item.id}`);
    try {
      await api.deleteSubscription(item.id);
      await load();
      onToast("订阅已删除");
    } catch (reason) {
      onToast(errorMessage(reason, "删除失败"));
    } finally {
      setBusy("");
    }
  };

  const testNode = async (node: ControllerProxy) => {
    setTestingNodes(current => new Set(current).add(node.name));
    try {
      const result = await api.testProxy(node.name, testUrl, timeout);
      appendLatency(setLatencyHistory, node.name, result.delay);
    } catch (reason) {
      onToast(errorMessage(reason, `${node.name} 测速失败`));
    } finally {
      setTestingNodes(current => {
        const next = new Set(current);
        next.delete(node.name);
        return next;
      });
    }
  };

  const testSubscription = async (item: Subscription, nodes: ControllerProxy[]) => {
    if (!nodes.length || testingSubscriptions.has(item.id)) return;
    setTestingSubscriptions(current => new Set(current).add(item.id));
    setTestingNodes(current => new Set([...current, ...nodes.map(node => node.name)]));
    try {
      const results = await Promise.allSettled(nodes.map(node => api.testProxy(node.name, testUrl, timeout)));
      results.forEach((result, index) => {
        if (result.status === "fulfilled") appendLatency(setLatencyHistory, nodes[index].name, result.value.delay);
      });
      const failed = results.filter(result => result.status === "rejected").length;
      onToast(failed ? `测速完成，${failed} 个节点失败` : "测速完成");
    } finally {
      setTestingNodes(current => {
        const next = new Set(current);
        nodes.forEach(node => next.delete(node.name));
        return next;
      });
      setTestingSubscriptions(current => {
        const next = new Set(current);
        next.delete(item.id);
        return next;
      });
    }
  };

  const openEditorTab = (tab: EditorTab) => {
    if (!form) return;
    setForm({ ...form, tab });
  };

  const openNewShare = () => setShareForm({
    name: "",
    protocol: window.location.protocol === "https:" ? "https" : "http",
    host: window.location.host,
    subscriptionIds: [],
    token: newShareToken(),
  });

  const saveShare = async () => {
    if (!shareForm?.name.trim() || !shareForm.host.trim() || !shareForm.subscriptionIds.length) return;
    setBusy("share:save");
    try {
      if (shareForm.id) {
        await api.updateSubscriptionShare(shareForm.id, {
          name: shareForm.name.trim(),
          host: shareForm.host.trim(),
          protocol: shareForm.protocol,
          subscriptionIds: shareForm.subscriptionIds,
          regenerate: false,
        });
      } else {
        await api.createSubscriptionShare({
          name: shareForm.name.trim(),
          host: shareForm.host.trim(),
          protocol: shareForm.protocol,
          subscriptionIds: shareForm.subscriptionIds,
          token: shareForm.token,
        });
      }
      setShareForm(null);
      await load();
      onToast("订阅分享已保存");
    } catch (reason) {
      onToast(errorMessage(reason, "保存订阅分享失败"));
    } finally {
      setBusy("");
    }
  };

  const toggleShare = async (share: SubscriptionShare) => {
    setBusy(`share:${share.id}`);
    try {
      await api.updateSubscriptionShare(share.id, {
        name: share.name,
        host: share.host,
        protocol: share.protocol,
        subscriptionIds: share.subscriptionIds,
        enabled: !share.enabled,
      });
      await load();
    } catch (reason) {
      onToast(errorMessage(reason, "订阅分享状态更新失败"));
    } finally {
      setBusy("");
    }
  };

  const regenerateShare = async (share: SubscriptionShare) => {
    if (!window.confirm(`重新生成“${share.name}”的链接吗？旧链接将立即失效。`)) return;
    setBusy(`share:${share.id}`);
    try {
      await api.regenerateSubscriptionShare(share.id);
      await load();
      onToast("分享链接已重新生成");
    } catch (reason) {
      onToast(errorMessage(reason, "重新生成分享链接失败"));
    } finally {
      setBusy("");
    }
  };

  const removeShare = async (share: SubscriptionShare) => {
    if (!window.confirm(`确定删除订阅分享“${share.name}”吗？`)) return;
    setBusy(`share:${share.id}`);
    try {
      await api.deleteSubscriptionShare(share.id);
      await load();
      onToast("订阅分享已删除");
    } catch (reason) {
      onToast(errorMessage(reason, "删除订阅分享失败"));
    } finally {
      setBusy("");
    }
  };

  if (!loaded) return <section className="surface subscription-loading" aria-label="正在加载订阅数据">
    <span className="loading-spinner" />
    <strong>正在加载订阅数据</strong>
  </section>;

  return <div className="subscription-settings-page">
    {Boolean(error) && <ErrorState title="订阅数据加载失败" error={error} onRetry={() => void load()} />}

    <section className={`surface subscription-share-panel${shareCollapsed ? " collapsed" : ""}`}>
      <header className="subscription-panel-header">
        <button type="button" className="subscription-panel-toggle" aria-expanded={!shareCollapsed} onClick={() => setShareCollapsed(value => !value)}>
          <span><strong>订阅分享</strong>{shareCollapsed ? <ChevronDownIcon /> : <ChevronUpIcon />}</span>
          <small>生成可供其他设备或代理软件直接使用的订阅链接</small>
        </button>
        <button type="button" className="subscription-panel-add tooltip-trigger tooltip-align-right" aria-label="添加订阅分享" data-tooltip="添加订阅分享" disabled={!items.length} onClick={openNewShare}><PlusIcon /></button>
      </header>
      <div className={`subscription-share-collapse${shareCollapsed ? " collapsed" : ""}`} aria-hidden={shareCollapsed} inert={shareCollapsed || undefined}>
        <div className="subscription-share-collapse-inner">
          {shares.length ? <div className="subscription-share-list">{shares.map(share => <article key={share.id}>
            <div><strong>{share.name}</strong><small>{share.subscriptionIds.map(id => items.find(item => item.id === id)?.name ?? id).join("、")}</small><span>{subscriptionShareUrl(share)}</span></div>
            <div className="subscription-card-actions">
              <ActionButton label={share.enabled ? "停用订阅分享" : "启用订阅分享"} disabled={Boolean(busy)} active={share.enabled} onClick={() => void toggleShare(share)}><PowerIcon /></ActionButton>
              <ActionButton label="复制分享链接" onClick={() => void navigator.clipboard.writeText(subscriptionShareUrl(share)).then(() => onToast("分享链接已复制"))}><ClipboardDocumentIcon /></ActionButton>
              <ActionButton label="重新生成分享链接" disabled={Boolean(busy)} onClick={() => void regenerateShare(share)}><ArrowPathRoundedSquareIcon /></ActionButton>
              <ActionButton label="修改订阅分享" disabled={Boolean(busy)} onClick={() => setShareForm({ id: share.id, name: share.name, protocol: share.protocol, host: share.host, subscriptionIds: [...share.subscriptionIds], token: share.token })}><PencilSquareIcon /></ActionButton>
              <ActionButton label="删除订阅分享" disabled={Boolean(busy)} onClick={() => void removeShare(share)}><TrashIcon /></ActionButton>
            </div>
          </article>)}</div> : <div className="subscription-share-empty">暂无订阅分享,点右上角「添加」创建</div>}
        </div>
      </div>
    </section>

    {items.map(item => {
      const nodes = nodesBySubscription[item.id] ?? [];
      const tested = nodes.filter(node => latestLatency(node.history) != null).length;
      const isCollapsed = Boolean(collapsed[item.id]);
      const itemBusy = busy === `subscription:${item.id}`;
      const itemTesting = testingSubscriptions.has(item.id);
      const enabled = item.enabled !== false;
      return <article className={`surface subscription-manage-card${isCollapsed ? " collapsed" : ""}`} key={item.id}>
        <header>
          <button type="button" className="subscription-manage-toggle" aria-expanded={!isCollapsed} aria-label={`${isCollapsed ? "展开" : "收起"} ${item.name}`} onClick={() => setCollapsed(value => ({ ...value, [item.id]: !value[item.id] }))}>
            <span className="subscription-title-row"><Bars3Icon /><strong>{item.name}</strong><small>({tested}/{nodes.length || item.nodeCount})</small></span>
            <span className="subscription-updated">更新于 {formatRelativeTime(item.updatedAt)}</span>
            {isCollapsed && nodes.length > 0 && <NodeHealthDots options={nodes} thresholds={thresholds} />}
          </button>
          <div className="subscription-card-actions">
            <ActionButton label={enabled ? "停用这条订阅；重启内核后生效" : "启用这条订阅；重启内核后生效"} active={enabled} disabled={Boolean(busy)} onClick={() => void toggleSubscription(item)}><PowerIcon /></ActionButton>
            <ActionButton label="测试这条订阅的节点延迟" disabled={!nodes.length} aria-busy={itemTesting || undefined} onClick={() => void testSubscription(item, nodes)}><BoltIcon /></ActionButton>
            <ActionButton label="刷新" disabled={Boolean(busy)} onClick={() => void refresh(item)}><ArrowPathIcon className={itemBusy ? "spinning" : undefined} /></ActionButton>
            <ActionButton label="修改订阅" disabled={Boolean(busy)} onClick={() => { setPreview(null); setPreviewError(""); setForm(formFromSubscription(item)); }}><PencilSquareIcon /></ActionButton>
            <ActionButton label="删除" disabled={Boolean(busy)} onClick={() => void remove(item)}><TrashIcon /></ActionButton>
          </div>
        </header>
        <section className={`subscription-node-collapse${isCollapsed ? " collapsed" : ""}`} aria-hidden={isCollapsed} inert={isCollapsed || undefined}>
          <div className="subscription-node-collapse-inner">
            {nodes.length ? <div className="policy-options subscription-node-grid">{nodes.map(node => <ProxyOption
              key={node.name}
              option={node}
              active={false}
              disabled={false}
              testDisabled={testingNodes.has(node.name)}
              thresholds={thresholds}
              showIcon={false}
              testing={testingNodes.has(node.name)}
              onTest={() => void testNode(node)}
            />)}</div> : <div className="subscription-node-empty">暂无可显示节点</div>}
          </div>
        </section>
      </article>;
    })}

    {!items.length && !error && <section className="surface subscription-empty"><LinkIcon /><strong>暂无订阅</strong><span>点右上角「添加」创建</span></section>}

    {form && <SubscriptionEditorModal
      form={form}
      nodeCount={form.id ? items.find(item => item.id === form.id)?.nodeCount ?? 0 : 0}
      setForm={setForm}
      busy={busy === "subscription:save"}
      preview={preview}
      previewError={previewError}
      testUrl={testUrl}
      timeout={timeout}
      onToast={onToast}
      onTab={openEditorTab}
      onSave={() => void save()}
      onClose={() => setForm(null)}
    />}

    {shareForm && <SubscriptionShareModal
      form={shareForm}
      items={items}
      busy={busy === "share:save"}
      setForm={setShareForm}
      onSave={() => void saveShare()}
      onClose={() => setShareForm(null)}
    />}
  </div>;
}

function SubscriptionEditorModal({ form, nodeCount, setForm, busy, preview, previewError, testUrl, timeout, onToast, onTab, onSave, onClose }: {
  form: FormValue;
  nodeCount: number;
  setForm: Dispatch<SetStateAction<FormValue | null>>;
  busy: boolean;
  preview: SubscriptionPreview | null;
  previewError: string;
  testUrl: string;
  timeout: number;
  onToast: (message: string) => void;
  onTab: (tab: EditorTab) => void;
  onSave: () => void;
  onClose: () => void;
}) {
  const patchForm = (patch: Partial<FormValue>) => setForm(current => current ? { ...current, ...patch } : current);
  const patchRename = (patch: Partial<SubscriptionRenameOptions>) => patchForm({ renameOptions: { ...form.renameOptions, ...patch } });
  const sourceLocked = Boolean(form.id);
  const [initialRenameOptions] = useState(() => cloneRenameOptions(form.renameOptions));
  const draggingToken = useRef<number | null>(null);
  const draggingRegion = useRef<number | null>(null);
  const [draggingTokenKey, setDraggingTokenKey] = useState<string | null>(null);
  const [draggingRegionKey, setDraggingRegionKey] = useState<string | null>(null);
  const [previewTab, setPreviewTab] = useState<"kept" | "excluded" | "disabled">("kept");
  const [latencies, setLatencies] = useState<Record<string, SubscriptionNodeLatencyResult>>({});
  const [testingPreviewNodes, setTestingPreviewNodes] = useState<Set<string>>(() => new Set());
  const [testingPreviewAll, setTestingPreviewAll] = useState(false);
  const renameTokenElements = useRef(new Map<string, HTMLElement>());
  const renameTokenRects = useRef(new Map<string, DOMRect>());
  const regionElements = useRef(new Map<string, HTMLElement>());
  const regionRects = useRef(new Map<string, DOMRect>());
  const regionDict = form.renameOptions.regionDict ?? [];
  const rulesEnabled = form.renameOptions.enabled !== false;
  const renameTokens = templateTokens(form.renameOptions.template);
  const overrides = form.renameOptions.overrides ?? {};
  const disabledNodes = form.renameOptions.disabled ?? [];

  useLayoutEffect(() => animateReorder(renameTokenElements.current, renameTokenRects.current), [form.renameOptions.template]);
  useLayoutEffect(() => animateReorder(regionElements.current, regionRects.current), [regionDict]);

  const patchRegion = (index: number, patch: Partial<(typeof regionDict)[number]>) => patchRename({
    regionDict: regionDict.map((region, itemIndex) => itemIndex === index ? { ...region, ...patch } : region),
  });
  const moveRegion = (from: number, to: number) => {
    if (from === to) return;
    const next = [...regionDict];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    patchRename({ regionDict: next });
  };
  const moveRenameToken = (from: number, to: number) => {
    if (from === to) return;
    const next = [...renameTokens];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    patchRename({ template: next.join("-") });
  };
  const updateOverride = (originalTag: string, value: string, defaultName: string) => {
    const next = { ...overrides };
    if (!value.trim() || value === defaultName) delete next[originalTag]; else next[originalTag] = value;
    patchRename({ overrides: next });
  };
  const togglePreviewNode = (originalTag: string, disabled: boolean) => {
    const next = new Set(disabledNodes);
    if (disabled) next.add(originalTag); else next.delete(originalTag);
    patchRename({ disabled: [...next] });
  };
  const runPreviewTests = async (tags: string[]) => {
    if (!tags.length || testingPreviewAll) return;
    const testingAll = tags.length > 1;
    if (testingAll) setTestingPreviewAll(true);
    setTestingPreviewNodes(current => new Set([...current, ...tags]));
    try {
      const response = await api.testSubscriptionNodes(subscriptionNodeTestPayload(form, tags, testUrl, timeout));
      setLatencies(current => {
        const next = { ...current };
        tags.forEach((tag, index) => { next[tag] = response.results[index] ?? { ok: false, reason: "error" }; });
        return next;
      });
      if (testingAll) {
        const failures = response.results.filter(result => !result.ok).length;
        onToast(failures ? `测速完成，${failures} 个节点失败` : "测速完成");
      }
    } catch (reason) {
      setLatencies(current => ({ ...current, ...Object.fromEntries(tags.map(tag => [tag, { ok: false, reason: "error", error: errorMessage(reason, "测速失败") }])) }));
      onToast(errorMessage(reason, "测速失败"));
    } finally {
      setTestingPreviewNodes(current => {
        const next = new Set(current);
        tags.forEach(tag => next.delete(tag));
        return next;
      });
      if (testingAll) setTestingPreviewAll(false);
    }
  };

  const previewRows = previewTab === "kept"
    ? preview?.preview ?? []
    : (previewTab === "excluded" ? preview?.excluded ?? [] : preview?.disabled ?? []).map(item => ({ originalTag: item.name, newTag: "" }));
  return <Modal
    title={form.id ? "修改订阅" : "添加订阅或节点"}
    className="subscription-editor-modal"
    onClose={onClose}
    footer={<><button type="button" className="compact-button" onClick={onClose}>取消</button><button type="button" className="primary-button" disabled={busy || !validSubscriptionForm(form)} onClick={onSave}>{busy && <span className="loading-spinner" />}保存</button></>}
  >
    <div className="subscription-editor-tabs">
      <div className={`subscription-tab-group${sourceLocked ? " locked" : ""}`} role="tablist" aria-label="来源类型" title={sourceLocked ? "来源类型不能修改；要换请新建一条订阅。" : undefined}>
        <button type="button" role="tab" aria-selected={form.sourceType === "url"} className={form.sourceType === "url" ? "active" : ""} disabled={sourceLocked} onClick={() => patchForm({ sourceType: "url" })}>订阅</button>
        <button type="button" role="tab" aria-selected={form.sourceType === "paste"} className={form.sourceType === "paste" ? "active" : ""} disabled={sourceLocked} onClick={() => patchForm({ sourceType: "paste" })}>节点</button>
      </div>
      <div className="subscription-tab-group" role="tablist" aria-label="编辑页面">
        {(["source", "rules", "nodes"] as const).map(tab => <button type="button" role="tab" aria-selected={form.tab === tab} className={form.tab === tab ? "active" : ""} key={tab} onClick={() => onTab(tab)}>{tab === "source" ? "订阅" : tab === "rules" ? "规则" : `节点${form.id ? ` (${preview?.nodes.length ?? nodeCount})` : preview ? ` (${preview.nodes.length})` : ""}`}</button>)}
      </div>
    </div>

    {form.tab === "source" && <div className="subscription-source-form">
      <div className="subscription-field-group"><label htmlFor="subscription-name">名称</label><input id="subscription-name" value={form.name} onChange={event => patchForm({ name: event.target.value })} placeholder="比如“我的订阅”" /></div>
      {form.sourceType === "url" ? <div className="subscription-url-form">
        <div className="subscription-form-label"><span>订阅链接</span><button type="button" onClick={() => patchForm({ urls: [...form.urls, ""] })}><PlusIcon />添加订阅</button></div>
        <div className="subscription-url-list">{form.urls.map((url, index) => <div key={index}><input type="url" value={url} placeholder="https://" autoComplete="off" onChange={event => patchForm({ urls: form.urls.map((value, itemIndex) => itemIndex === index ? event.target.value : value) })} /><IconButton label="删除" tooltip className="tooltip-trigger tooltip-align-right" disabled={form.urls.length === 1} onClick={() => patchForm({ urls: form.urls.filter((_, itemIndex) => itemIndex !== index) })}><TrashIcon /></IconButton></div>)}</div>
        <p>一行一个,多个地址的节点合在一起;支持内网 / 本机地址,不校验 https 证书。</p>
        <div className="subscription-source-options">
          <label><span>定期更新</span><input type="checkbox" className="switch-input" aria-label="定期更新" checked={form.autoUpdate.enabled} onChange={event => patchForm({ autoUpdate: { ...form.autoUpdate, enabled: event.target.checked } })} /></label>
          {form.autoUpdate.enabled && <><SelectControl label="更新周期" value={form.autoUpdate.mode === "hours" ? `h${form.autoUpdate.hours ?? 6}` : `d${form.autoUpdate.days}`} onValueChange={value => patchForm({ autoUpdate: autoUpdateFromValue(form.autoUpdate, value) })} options={[...[1,2,3,4,6,8,12].map(value => ({ value: `h${value}`, label: `每 ${value} 小时` })), ...[1,2,3,7,14,30].map(value => ({ value: `d${value}`, label: `每 ${value} 天` }))]} />{form.autoUpdate.mode !== "hours" && <SelectControl label="更新时间" value={String(form.autoUpdate.hour)} onValueChange={value => patchForm({ autoUpdate: { ...form.autoUpdate, hour: Number(value) } })} options={Array.from({ length: 24 }, (_, hour) => ({ value: String(hour), label: `${String(hour).padStart(2, "0")}:00` }))} />}</>}
          <button type="button" className="subscription-dns-toggle" onClick={() => patchForm({ nodeDnsExpanded: !form.nodeDnsExpanded })}>订阅 DNS 解析 <ChevronDownIcon className={form.nodeDnsExpanded ? "expanded" : ""} /></button>
        </div>
      </div> : <div className="subscription-node-source">
        <div className="subscription-form-label"><span>节点内容</span><button type="button" className="subscription-dns-toggle" onClick={() => patchForm({ nodeDnsExpanded: !form.nodeDnsExpanded })}>订阅 DNS 解析 <ChevronDownIcon className={form.nodeDnsExpanded ? "expanded" : ""} /></button></div>
        <textarea rows={8} value={form.content} onChange={event => patchForm({ content: event.target.value })} placeholder="粘贴 ss:// / trojan:// / hysteria2:// 等链接,或 Clash/sing-box 配置" />
        <p>一行一个节点链接（ss:// / trojan:// / vless:// / socks5:// 等）;也支持整段 Clash / sing-box 配置。</p>
      </div>}
      {form.nodeDnsExpanded && <div className="subscription-dns-fields"><label><span>DNS over HTTPS 地址</span><input value={form.nodeDns?.url ?? ""} placeholder="https://dns.example/dns-query" onChange={event => patchForm({ nodeDns: { url: event.target.value, bootstrap: form.nodeDns?.bootstrap } })} /></label><label><span>Bootstrap IP</span><input value={form.nodeDns?.bootstrap ?? ""} placeholder="1.1.1.1" onChange={event => patchForm({ nodeDns: { url: form.nodeDns?.url ?? "", bootstrap: event.target.value } })} /></label></div>}
    </div>}

    {form.tab === "rules" && <div className="subscription-rules-form">
      <label className="subscription-prefix-option"><input type="checkbox" checked={form.renameOptions.usePrefix === true} onChange={event => patchRename({ usePrefix: event.target.checked })} /><span>节点名前加订阅名（如「机场名称 | 香港-01」）</span></label>
      <div className="subscription-rules-heading">
        <label><h3>重命名规则</h3><input type="checkbox" className="subscription-rule-switch" aria-label="重命名规则" checked={rulesEnabled} onChange={event => patchRename({ enabled: event.target.checked })} /></label>
        {rulesEnabled && <button type="button" className="subscription-text-button" onClick={() => patchForm({ renameOptions: cloneRenameOptions(initialRenameOptions) })}><ArrowPathIcon />重置</button>}
      </div>
      {!rulesEnabled && <p className="subscription-rule-hint">已关闭:节点保留机场的原始名字;地区仍按关键词识别,用于国旗和按地区选成员。</p>}
      {rulesEnabled && <div className="subscription-rule-order">
        <div className="subscription-rule-tokens"><span>拖拽排序:</span>{renameTokens.map((token, index) => <button className={draggingTokenKey === token ? "is-drag-placeholder" : undefined} key={token} ref={element => { if (element) renameTokenElements.current.set(token, element); else renameTokenElements.current.delete(token); }} type="button" draggable onDragStart={event => {
          event.dataTransfer.effectAllowed = "move";
          draggingToken.current = index;
          window.requestAnimationFrame(() => setDraggingTokenKey(token));
        }} onDragOver={event => {
          event.preventDefault();
          event.dataTransfer.dropEffect = "move";
          const from = draggingToken.current;
          if (from === null || from === index) return;
          const rect = event.currentTarget.getBoundingClientRect();
          const trigger = rect.left + rect.width * (from < index ? .25 : .75);
          if ((from < index && event.clientX < trigger) || (from > index && event.clientX > trigger)) return;
          moveRenameToken(from, index);
          draggingToken.current = index;
        }} onDrop={event => { event.preventDefault(); draggingToken.current = null; setDraggingTokenKey(null); }} onDragEnd={() => { draggingToken.current = null; setDraggingTokenKey(null); }}><Bars3Icon />{renameTokenLabel(token)}</button>)}</div>
        <label><span>序号位数:</span><input type="number" min="1" max="4" value={form.renameOptions.seqPad ?? 2} onChange={event => patchRename({ seqPad: Number(event.target.value) })} /></label>
        <p>效果:{renameExample(form.renameOptions)}</p>
      </div>}
      <div className="subscription-region-rules">
        <div className="subscription-form-label"><span>地区关键词</span><button type="button" onClick={() => patchRename({ regionDict: [...regionDict, { code: "", name: "", keywords: [] }] })}><PlusIcon />新增</button></div>
        <p>按原始节点名匹配（不区分大小写）。</p>
        <div className="subscription-region-list">{regionDict.map((region, index) => <div
          className={`subscription-region-row${draggingRegionKey === (region.code || `new-region-${index}`) ? " is-drag-placeholder" : ""}`}
          key={region.code || `new-region-${index}`}
          ref={element => { const key = region.code || `new-region-${index}`; if (element) regionElements.current.set(key, element); else regionElements.current.delete(key); }}
          draggable
          onDragStart={event => {
            event.dataTransfer.effectAllowed = "move";
            draggingRegion.current = index;
            const key = region.code || `new-region-${index}`;
            window.requestAnimationFrame(() => setDraggingRegionKey(key));
          }}
          onDragOver={event => {
            event.preventDefault();
            event.dataTransfer.dropEffect = "move";
            const from = draggingRegion.current;
            if (from === null || from === index) return;
            const rect = event.currentTarget.getBoundingClientRect();
            const trigger = rect.top + rect.height * (from < index ? .25 : .75);
            if ((from < index && event.clientY < trigger) || (from > index && event.clientY > trigger)) return;
            moveRegion(from, index);
            draggingRegion.current = index;
          }}
          onDrop={event => { event.preventDefault(); draggingRegion.current = null; setDraggingRegionKey(null); }}
          onDragEnd={() => { draggingRegion.current = null; setDraggingRegionKey(null); }}
        >
          <Bars3Icon className="subscription-drag-handle" aria-label="拖拽排序" />
          <label className="subscription-region-select">
            <span aria-hidden="true">{countryFlag(region.code)}</span>
            <SelectControl label={`${region.name || region.code || "新地区"} 地区`} value={region.code} onValueChange={value => { const selected = regionDict.find(item => item.code === value); patchRegion(index, selected ? { code: selected.code, name: selected.name } : { code: value }); }} options={[...(!region.code ? [{ value: "", label: "选择地区" }] : []), ...uniqueRegions(regionDict).map(option => ({ value: option.code, label: option.name || option.code }))]} />
          </label>
          <input value={region.keywords.join(",")} aria-label={`${region.name || region.code || "新地区"} 关键词`} placeholder="关键词,用逗号分隔" onChange={event => patchRegion(index, { keywords: commaValues(event.target.value) })} />
          <IconButton label="删除地区规则" tooltip className="tooltip-trigger tooltip-align-right subscription-region-remove" onClick={() => patchRename({ regionDict: regionDict.filter((_, itemIndex) => itemIndex !== index) })}><XMarkIcon /></IconButton>
        </div>)}</div>
      </div>
      {rulesEnabled && <div className="subscription-rule-field"><label>无法识别地区时的标签</label><input value={form.renameOptions.unknownLabel ?? ""} onChange={event => patchRename({ unknownLabel: event.target.value })} /></div>}
      {rulesEnabled && <div className="subscription-rule-field"><label>特征关键词</label><p>命中的关键词转成大写写进节点名,如填 iplc,ipv6,「美国 IPLC IPv6 01」变成 美国-IPLC-IPV6-01。</p><input value={(form.renameOptions.featureKeywords ?? []).join(",")} placeholder="关键词,用逗号分隔" onChange={event => patchRename({ featureKeywords: commaValues(event.target.value) })} /></div>}
      <div className="subscription-rule-field"><label>过滤节点</label><p>原始节点名命中关键词的整条不导入,用来过滤「官网」「提工单」这类公告条目;清空则不过滤。</p><input value={(form.renameOptions.excludeKeywords ?? []).join(",")} placeholder="关键词,用逗号分隔" onChange={event => patchRename({ excludeKeywords: commaValues(event.target.value) })} /></div>
    </div>}

    {form.tab === "nodes" && <div className="subscription-preview-panel">
      {!validSubscriptionForm(form) ? <div className="subscription-preview-state"><MagnifyingGlassIcon aria-hidden="true" />填写链接或粘贴内容后,这里会实时显示预览结果。</div> : <>
        {preview && <div className="subscription-preview-heading"><strong>解析出 {preview.nodes.length} 个节点</strong><span>{preview.format}</span></div>}
        {previewError && <div className="subscription-preview-error">{previewError}</div>}
        {preview && !previewError && <>
          {preview.groups.length > 0 && <div className="subscription-preview-groups">{preview.groups.map(group => <span key={`${group.type}-${group.name}`}>{group.name} × {group.nodeTags.length}</span>)}</div>}
          {preview.skipped.length > 0 && <div className="subscription-preview-skipped"><strong>跳过了 {preview.skipped.length} 个</strong>{preview.skipped.map((node, index) => <span key={`${node.name}-${index}`}>{node.name} —— {node.detail || node.reason}</span>)}</div>}
          <div className="subscription-preview-toolbar">
            <div className="subscription-preview-tabs" role="tablist" aria-label="节点状态">
              <button type="button" role="tab" aria-selected={previewTab === "kept"} className={previewTab === "kept" ? "active" : ""} onClick={() => setPreviewTab("kept")}>有效（{preview.preview.length}）</button>
              <button type="button" role="tab" aria-selected={previewTab === "excluded"} className={previewTab === "excluded" ? "active" : ""} onClick={() => setPreviewTab("excluded")}>过滤（{preview.excluded.length}）</button>
              <button type="button" role="tab" aria-selected={previewTab === "disabled"} className={previewTab === "disabled" ? "active" : ""} onClick={() => setPreviewTab("disabled")}>禁用（{preview.disabled.length}）</button>
            </div>
            {previewTab === "kept" && <button type="button" className="subscription-preview-action" disabled={!Object.keys(overrides).length} title={`恢复 ${Object.keys(overrides).length} 条手工改名`} onClick={() => patchRename({ overrides: {} })}><ArrowUturnLeftIcon />恢复默认名称</button>}
            {previewTab === "kept" && <button type="button" className="subscription-preview-action" disabled={testingPreviewAll || !preview.preview.length} onClick={() => void runPreviewTests(preview.preview.map(node => node.originalTag))}>{testingPreviewAll ? <span className="loading-spinner" /> : <BoltIcon />}一键测速</button>}
          </div>
          <div className="subscription-preview-table-wrap">
            {!previewRows.length ? <div className="subscription-preview-empty">还没解析出节点。</div> : <div className="subscription-preview-table-scroll"><table className="subscription-preview-table"><thead><tr><th>原名</th><th>新名</th></tr></thead><tbody>{previewRows.map((node, index) => {
              const latency = latencies[node.originalTag];
              const testing = testingPreviewNodes.has(node.originalTag);
              return <tr key={`${node.originalTag}-${index}`}><td title={node.originalTag}>{node.originalTag}</td><td>
                {previewTab === "excluded" ? <span className="subscription-not-imported">不导入</span> : previewTab === "disabled" ? <span className="subscription-disabled-row"><span>不导入</span><IconButton label="启用" tooltip className="tooltip-trigger tooltip-align-right" onClick={() => togglePreviewNode(node.originalTag, false)}><CheckCircleIcon /></IconButton></span> : <span className="subscription-node-edit-row">
                  <ArrowRightIcon className="subscription-rename-arrow" />
                  <input value={overrides[node.originalTag] ?? node.newTag} aria-label={`${node.originalTag} 新名`} onChange={event => updateOverride(node.originalTag, event.target.value, node.newTag)} />
                  <span className={`subscription-node-latency${latency && !latency.ok ? " error" : ""}`} title={latency?.error || latency?.reason}>{latency ? latency.ok ? `${latency.ms ?? 0}ms` : "失败" : "未测速"}</span>
                  <IconButton label="测速" tooltip className="tooltip-trigger" disabled={testingPreviewAll || testing} onClick={() => void runPreviewTests([node.originalTag])}>{testing ? <span className="loading-spinner" /> : <BoltIcon />}</IconButton>
                  <IconButton label="禁用" tooltip className="tooltip-trigger tooltip-align-right" onClick={() => togglePreviewNode(node.originalTag, true)}><NoSymbolIcon /></IconButton>
                </span>}
              </td></tr>;
            })}</tbody></table></div>}
          </div>
        </>}
      </>}
    </div>}
  </Modal>;
}

function SubscriptionShareModal({ form, items, busy, setForm, onSave, onClose }: {
  form: ShareFormValue;
  items: Subscription[];
  busy: boolean;
  setForm: Dispatch<SetStateAction<ShareFormValue | null>>;
  onSave: () => void;
  onClose: () => void;
}) {
  const patchForm = (patch: Partial<ShareFormValue>) => setForm(current => current ? { ...current, ...patch } : current);
  const shareUrl = `${form.protocol}://${form.host}/sub/${form.token}`;
  return <Modal title={form.id ? "编辑订阅分享" : "新增订阅分享"} className="subscription-share-modal" onClose={onClose} footer={<><button type="button" className="compact-button" onClick={onClose}>取消</button><button type="button" className="primary-button" disabled={busy || !form.name.trim() || !form.host.trim() || !form.subscriptionIds.length} onClick={onSave}>{busy && <span className="loading-spinner" />}保存</button></>}>
    <div className="subscription-share-layout">
      <fieldset><legend>选择要分享的订阅</legend>{items.map(item => <label key={item.id}><input type="checkbox" checked={form.subscriptionIds.includes(item.id)} onChange={event => patchForm({ subscriptionIds: event.target.checked ? [...form.subscriptionIds, item.id] : form.subscriptionIds.filter(id => id !== item.id) })} /><span>{item.name}</span><small>{item.nodeCount} 个节点</small></label>)}</fieldset>
      <div className="subscription-share-fields">
        <label><span>标题</span><input value={form.name} placeholder="例如:手机代理订阅" onChange={event => patchForm({ name: event.target.value })} /></label>
        <label><span>域名或 IP</span><span className="subscription-share-host"><SelectControl label="分享链接协议" value={form.protocol} onValueChange={value => patchForm({ protocol: value })} options={[{ value: "http", label: "http://" }, { value: "https", label: "https://" }]} /><input value={form.host} onChange={event => patchForm({ host: event.target.value })} /></span></label>
        <label><span>分享链接</span><span className="subscription-share-link"><input readOnly value={shareUrl} /><IconButton label="复制分享链接" tooltip className="tooltip-trigger tooltip-align-right" onClick={() => void navigator.clipboard.writeText(shareUrl)}><ClipboardDocumentIcon /></IconButton></span></label>
        <div className="subscription-share-qr"><QRCodeSVG value={shareUrl} size={164} level="M" aria-label="订阅分享二维码" /></div>
      </div>
    </div>
  </Modal>;
}

function ActionButton({ label, active = false, ...props }: ComponentProps<typeof IconButton>) {
  return <IconButton {...props} label={label} active={active} tooltip className="subscription-action tooltip-trigger" />;
}

export function subscriptionNodesFromApi(
  subscription: Subscription,
  groups: GroupsResponse | null,
  proxies: ProxiesResponse,
  latencyHistory: Record<string, ProxyHistory[]>,
) {
  const names = groups?.availableNodes
    .filter(node => node.subscription === subscription.name)
    .map(node => node.name) ?? [];
  const nodes = [...new Set(names)].flatMap(name => {
    const proxy = proxies.proxies[name];
    return proxy ? [{ ...proxy, history: latencyHistory[name] ?? proxy.history }] : [];
  });
  return sortProxyNodes(nodes, "latencyAsc");
}

function appendLatency(setLatencyHistory: Dispatch<SetStateAction<Record<string, ProxyHistory[]>>>, name: string, delay: number) {
  const point = { time: new Date().toISOString(), delay, node: name };
  setLatencyHistory(current => ({ ...current, [name]: [...(current[name] ?? []), point] }));
}

function animateReorder(elements: Map<string, HTMLElement>, previousRects: Map<string, DOMRect>) {
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const nextRects = new Map<string, DOMRect>();
  elements.forEach((element, key) => {
    const animations = element.getAnimations();
    const visual = animations.length ? element.getBoundingClientRect() : previousRects.get(key);
    animations.forEach(animation => animation.cancel());
    const next = element.getBoundingClientRect();
    nextRects.set(key, next);
    const x = visual ? visual.left - next.left : 0;
    const y = visual ? visual.top - next.top : 0;
    if (!reduceMotion && (Math.abs(x) > 0.5 || Math.abs(y) > 0.5)) {
      element.animate([
        { transform: `translate(${x}px, ${y}px)` },
        { transform: "translate(0, 0)" },
      ], { duration: 180, easing: "cubic-bezier(.2, .8, .2, 1)" });
    }
  });
  previousRects.clear();
  nextRects.forEach((rect, key) => previousRects.set(key, rect));
}

function subscriptionShareUrl(share: SubscriptionShare) {
  return share.url || `${share.protocol}://${share.host}/sub/${share.token}`;
}

export function emptyForm(): FormValue {
  return {
    sourceType: "url",
    tab: "source",
    name: "",
    urls: [""],
    content: "",
    autoUpdate: { enabled: false, days: 1, hour: 4 },
    renameOptions: defaultRenameOptions(),
    nodeDns: null,
    nodeDnsExpanded: false,
  };
}

function defaultRenameOptions(): SubscriptionRenameOptions {
  return {
    enabled: true,
    usePrefix: true,
    template: "{region}-{feature}-{seq}",
    seqPad: 2,
    unknownLabel: "其他",
    featureKeywords: ["iepl", "iplc", "ipv6", "专线", "家宽", "2x"],
    excludeKeywords: ["官网", "工单", "客服"],
    disabled: [],
    overrides: {},
    regionDict: [
      { code: "US", name: "美国", keywords: ["us", "united states", "america", "美国", "美國", "洛杉矶", "洛杉磯", "硅谷", "圣何塞", "西雅图", "纽约"] },
      { code: "HK", name: "香港", keywords: ["hk", "hong kong", "hongkong", "香港", "深港"] },
      { code: "JP", name: "日本", keywords: ["jp", "japan", "日本", "东京", "東京", "大阪"] },
      { code: "SG", name: "新加坡", keywords: ["sg", "singapore", "新加坡", "狮城", "獅城"] },
      { code: "TW", name: "台湾", keywords: ["tw", "taiwan", "台湾", "台灣", "臺灣", "台北"] },
      { code: "KR", name: "韩国", keywords: ["kr", "korea", "韩国", "韓國", "首尔", "首爾"] },
      { code: "GB", name: "英国", keywords: ["uk", "gb", "united kingdom", "britain", "英国", "英國", "伦敦", "倫敦"] },
      { code: "DE", name: "德国", keywords: ["de", "germany", "德国", "德國", "法兰克福", "法蘭克福"] },
      { code: "CN", name: "中国", keywords: ["cn", "china", "中国", "中國", "回国", "回國", "back to china"] },
    ],
  };
}

export function formFromSubscription(item: Subscription): FormValue {
  const urls = item.urls?.length ? item.urls : item.url ? [item.url] : [];
  return {
    id: item.id,
    sourceType: urls.length ? "url" : "paste",
    tab: "source",
    name: item.name,
    urls: urls.length ? [...urls] : [""],
    content: item.content ?? "",
    autoUpdate: item.autoUpdate ? { ...item.autoUpdate } : { enabled: false, days: 1, hour: 4 },
    renameOptions: cloneRenameOptions(item.renameOptions),
    nodeDns: item.nodeDns ? { ...item.nodeDns } : null,
    nodeDnsExpanded: Boolean(item.nodeDns?.url),
  };
}

function cloneRenameOptions(value?: SubscriptionRenameOptions): SubscriptionRenameOptions {
  return {
    ...(value ?? {}),
    disabled: [...(value?.disabled ?? [])],
    overrides: { ...(value?.overrides ?? {}) },
    featureKeywords: [...(value?.featureKeywords ?? [])],
    excludeKeywords: [...(value?.excludeKeywords ?? [])],
    regionDict: value?.regionDict?.map(region => ({ ...region, keywords: [...region.keywords] })) ?? [],
  };
}

function validSubscriptionForm(form: FormValue) {
  return form.sourceType === "url" ? form.urls.some(value => value.trim()) : Boolean(form.content.trim());
}

export function subscriptionPayload(form: FormValue): Partial<Subscription> {
  const nodeDns = form.nodeDns?.url.trim() ? { url: form.nodeDns.url.trim(), ...(form.nodeDns.bootstrap?.trim() ? { bootstrap: form.nodeDns.bootstrap.trim() } : {}) } : null;
  const common = {
    name: form.name.trim() || "订阅",
    renameOptions: form.renameOptions,
    autoUpdate: form.sourceType === "url" ? form.autoUpdate : { enabled: false, days: 1, hour: 4 },
    nodeDns,
  };
  return form.sourceType === "url"
    ? { ...common, urls: [...new Set(form.urls.map(value => value.trim()).filter(Boolean))] }
    : { ...common, content: form.content.trim() };
}

export function subscriptionNodeTestPayload(form: FormValue, tags: string[], testUrl: string, timeoutMs: number): Record<string, unknown> {
  const nodeDns = form.nodeDns?.url.trim()
    ? { url: form.nodeDns.url.trim(), ...(form.nodeDns.bootstrap?.trim() ? { bootstrap: form.nodeDns.bootstrap.trim() } : {}) }
    : null;
  const source = form.sourceType === "url"
    ? { urls: [...new Set(form.urls.map(value => value.trim()).filter(Boolean))] }
    : { content: form.content.trim() };
  return {
    ...source,
    ...(form.renameOptions.usePrefix ? { name: form.name.trim() || "订阅" } : {}),
    renameOptions: form.renameOptions,
    nodeDns,
    tags,
    testUrl,
    timeoutMs,
  };
}

function templateTokens(template?: string) {
  const tokens = ["{region}", "{feature}", "{seq}"];
  const present = tokens
    .map(token => ({ token, index: (template ?? "").indexOf(token) }))
    .filter(item => item.index >= 0)
    .sort((a, b) => a.index - b.index)
    .map(item => item.token);
  return [...present, ...tokens.filter(token => !present.includes(token))];
}

function renameTokenLabel(token: string) {
  return token === "{region}" ? "地区" : token === "{feature}" ? "特征" : "序号";
}

function renameExample(options: SubscriptionRenameOptions) {
  return (options.template || "{region}-{feature}-{seq}")
    .replaceAll("{region}", "美国")
    .replaceAll("{feature}", "专线")
    .replaceAll("{seq}", String(1).padStart(options.seqPad ?? 2, "0"));
}

function uniqueRegions(regions: NonNullable<SubscriptionRenameOptions["regionDict"]>) {
  return regions.filter((region, index) => region.code && regions.findIndex(item => item.code === region.code) === index);
}

function countryFlag(code: string) {
  const normalized = code.trim().toUpperCase();
  if (!/^[A-Z]{2}$/.test(normalized)) return "";
  return String.fromCodePoint(...[...normalized].map(char => 127397 + char.charCodeAt(0)));
}

function autoUpdateFromValue(current: SubscriptionAutoUpdate, value: string): SubscriptionAutoUpdate {
  const amount = Number(value.slice(1)) || 1;
  return value.startsWith("h")
    ? { enabled: current.enabled, days: current.days, hour: current.hour, mode: "hours", hours: amount }
    : { enabled: current.enabled, days: amount, hour: current.hour };
}

function commaValues(value: string) {
  return value.split(",").map(item => item.trim()).filter(Boolean);
}

function newShareToken() {
  return Array.from(crypto.getRandomValues(new Uint8Array(24)), value => value.toString(16).padStart(2, "0")).join("");
}

function finiteSetting(value: string | undefined, fallback: number) {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : fallback;
}

function errorMessage(reason: unknown, fallback: string) {
  return reason instanceof Error ? reason.message : fallback;
}
