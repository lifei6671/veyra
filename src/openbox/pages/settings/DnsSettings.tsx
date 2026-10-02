import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { ArrowPathIcon, ArrowUturnLeftIcon, BoltIcon, PencilSquareIcon, PlusIcon, TrashIcon } from "@heroicons/react/24/outline";
import { api } from "../../api/client";
import type { DnsFilterStatus, DnsRewriteRule, DnsUpstreamTest, OpenBoxProfile } from "../../api/types";
import { ErrorState, IconButton, Modal } from "../../components/shared";
import { SelectControl, SegmentedGroup, SegmentedItem, SwitchControl } from "../../ui/controls";
import { DnsFilterSettings } from "./DnsFilterSettings";
import { DnsQueryRecords } from "./DnsQueryRecords";
import { DNS_TEXT as T } from "./DnsSettings.messages";
import { defaultUpstreams, dnsError, dnsMessage, normalizeDomain, parseAddresses, restoreRewrites, rewriteNeedsRestart, rewriteValidation, upstreamPatch, upstreamRows, upstreamValidation, type UpstreamRow } from "./DnsSettings.helpers";

function testDetail(result: DnsUpstreamTest) {
  return [result.ms !== undefined ? `${result.ms} ms` : "", result.policy && result.via ? dnsMessage("dnsUpstreamTestVia", { policy: result.policy, node: result.chain?.join(" → ") || result.via }) : result.via, result.warning, result.note].filter(Boolean).join(" · ");
}
function UpstreamEditor({ original, rows, onSave, onClose }: { original?: UpstreamRow; rows: UpstreamRow[]; onSave: (rows: UpstreamRow[], message: string) => Promise<boolean>; onClose: () => void }) {
  const [value, setValue] = useState<UpstreamRow>(original || { side: "proxy", index: 0, server: "", protocol: "tcp", port: 53 });
  const [busy, setBusy] = useState(false), [result, setResult] = useState<DnsUpstreamTest | null>(null), [error, setError] = useState("");
  const invalid = upstreamValidation(value, rows, original);
  const change = (patch: Partial<UpstreamRow>) => { setValue(current => ({ ...current, ...patch })); setResult(null); setError(""); };
  const test = async (save: boolean) => {
    setBusy(true); setError("");
    try {
      const response = await api.testDnsUpstream({ server: value.server.trim(), protocol: value.protocol, port: value.port, side: value.side }); setResult(response);
      if (!response.ok) return;
      if (save) {
        const next = original ? rows.map(item => item.side === original.side && item.index === original.index ? value : item) : [...rows, value];
        if (await onSave(next, dnsMessage("dnsUpstreamSaved", { side: value.side === "direct" ? T.dnsUpstreamDirect : T.dnsUpstreamProxy, detail: testDetail(response) }))) onClose();
      }
    } catch (reason) { setError(dnsError(reason)); } finally { setBusy(false); }
  };
  return <Modal title={original ? T.dnsUpstreamEditTitle : T.dnsUpstreamAddTitle} className="dns-dialog routing-dialog" onClose={() => { if (!busy) onClose(); }} footer={<><button type="button" className="compact-button" disabled={busy} onClick={onClose}>取消</button><button type="button" className="primary-button" disabled={busy || !!invalid} onClick={() => void test(true)}>{busy && <span className="loading-spinner" />}保存</button></>}>
    <div className="dns-side-field"><span className="dns-hint">{T.dnsUpstreamSide}</span><SegmentedGroup disabled={busy} value={value.side} onValueChange={side => change({ side: side as UpstreamRow["side"] })} aria-label={T.dnsUpstreamSide}><SegmentedItem value="direct">{T.dnsUpstreamDirect}</SegmentedItem><SegmentedItem value="proxy">{T.dnsUpstreamProxy}</SegmentedItem></SegmentedGroup>
    <p className="dns-hint">{value.side === "direct" ? T.dnsUpstreamDirectNote : T.dnsUpstreamProxyNote}</p>
    </div><div className="dns-upstream-fields"><SelectControl label={T.dnsUpstreamProtocol} value={value.protocol} disabled={busy} options={[{ value: "udp", label: "UDP" }, { value: "tcp", label: "TCP" }]} onValueChange={protocol => change({ protocol: protocol as "tcp" | "udp" })} /><input aria-label="DNS 地址" placeholder={value.side === "direct" ? "223.5.5.5" : "1.1.1.1"} spellCheck={false} className="dns-mono" value={value.server} disabled={busy} onChange={event => change({ server: event.target.value.trim() })} /><input type="number" aria-label={T.dnsUpstreamPort} min={1} max={65535} value={value.port} disabled={busy} onChange={event => change({ port: Number(event.target.value) })} /><IconButton label={result?.ok ? testDetail(result) : T.dnsUpstreamTest} aria-label={T.dnsUpstreamTest} tooltip portalTooltip disabled={busy || !!invalid} onClick={() => void test(false)}>{busy ? <span className="loading-spinner" /> : result?.ok && result.ms !== undefined ? <span className="dns-test-latency dns-success">{result.ms}</span> : <BoltIcon />}</IconButton></div>
    {value.server && invalid && <p className="dns-error">{invalid}</p>}{result && !result.ok && <p className="dns-error">{`${T.dnsUpstreamTestFail.replace("{message}", result.error || "测试失败")} ${result.note || ""}`}</p>}{error && <p role="alert" className="dns-error">{error}</p>}
  </Modal>;
}
function RewriteEditor({ original, rules, onSave, onClose }: { original?: DnsRewriteRule; rules: DnsRewriteRule[]; onSave: (rules: DnsRewriteRule[]) => Promise<boolean>; onClose: () => void }) {
  const [source, setSource] = useState(original?.source || ""), [domain, setDomain] = useState(original?.domain || ""), [addresses, setAddresses] = useState(original?.addresses.join("\n") || ""), [kind, setKind] = useState(original?.addresses.length ? "ip" : "domain"), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const save = async () => {
    const rule: DnsRewriteRule = { id: original?.id || `rw-${Date.now().toString(36)}`, enabled: original?.enabled !== false, source: normalizeDomain(source), domain: kind === "domain" ? normalizeDomain(domain) : "", addresses: kind === "ip" ? parseAddresses(addresses) : [], ...(original?.note ? { note: original.note } : {}) };
    const invalid = rewriteValidation(rule, rules, kind); if (invalid) { setError(invalid); return; }
    setBusy(true); try { if (await onSave(original ? rules.map(item => item.id === original.id ? rule : item) : [...rules, rule])) onClose(); } finally { setBusy(false); }
  };
  return <Modal title={original ? T.dnsRewriteEditTitle : T.dnsRewriteAddTitle} className="dns-dialog routing-dialog" onClose={() => { if (!busy) onClose(); }} footer={<><button type="button" className="compact-button" disabled={busy} onClick={onClose}>取消</button><button type="button" className="primary-button" disabled={busy} onClick={() => void save()}>{busy && <span className="loading-spinner" />}保存</button></>}>
    <label className="dns-side-field"><span className="dns-hint">{T.dnsRewriteSource}</span><input aria-label={T.dnsRewriteSource} spellCheck={false} className="dns-mono" placeholder="services.googleapis.cn / *.example.com" value={source} onChange={event => setSource(event.target.value)} disabled={busy} /><p className="dns-hint">{T.dnsRewriteSourceHint}</p></label>
    <div className="dns-side-field"><span className="dns-hint">{T.dnsRewriteTarget}</span><SegmentedGroup disabled={busy} value={kind} onValueChange={setKind} aria-label={T.dnsRewriteTarget}><SegmentedItem value="domain">{T.dnsRewriteKindDomain}</SegmentedItem><SegmentedItem value="ip">{T.dnsRewriteKindIp}</SegmentedItem></SegmentedGroup>
    {kind === "domain" ? <><input aria-label="目标域名" spellCheck={false} className="dns-mono" placeholder="services.googleapis.com" value={domain} onChange={event => setDomain(event.target.value)} disabled={busy} /><p className="dns-hint">{T.dnsRewriteDomainHint}</p></> : <><textarea aria-label="固定 IP" spellCheck={false} className="dns-mono" rows={3} value={addresses} onChange={event => setAddresses(event.target.value)} placeholder="192.168.3.1\n2001:db8::1" disabled={busy} /><p className="dns-hint">{T.dnsRewriteIpHint}</p></>}</div>{error && <p className="dns-error" role="alert">{error}</p>}
  </Modal>;
}
export function DnsSettings({ onToast, platform }: { onToast: (message: string) => void; platform?: string }) {
  const [profile, setProfile] = useState<OpenBoxProfile | null>(null), [filter, setFilter] = useState<DnsFilterStatus | null>(null), [error, setError] = useState<unknown>(null), [saving, setSaving] = useState(false), [applying, setApplying] = useState(false), [pending, setPending] = useState(false), [flushing, setFlushing] = useState(false), [confirmBusy, setConfirmBusy] = useState(false), [actions, setActions] = useState<HTMLElement | null>(null);
  const [upstream, setUpstream] = useState<{ row?: UpstreamRow } | null>(null), [rewriteEditor, setRewriteEditor] = useState<{ rule?: DnsRewriteRule } | null>(null), [testing, setTesting] = useState<string[]>([]), [confirm, setConfirm] = useState<{ title: string; text: string; run: () => Promise<void> } | null>(null);
  const load = async () => { const [nextProfile, nextFilter] = await Promise.all([api.profile(), api.dnsFilter()]); setProfile(nextProfile); setFilter(nextFilter); setError(null); };
  useEffect(() => { let active = true; setActions(document.getElementById("settings-header-actions")); void Promise.all([api.profile(), api.dnsFilter()]).then(([nextProfile, nextFilter]) => { if (active) { setProfile(nextProfile); setFilter(nextFilter); } }, reason => { if (active) setError(reason); }); return () => { active = false; }; }, []);
  useEffect(() => { const timer = window.setInterval(() => { if (!document.hidden && !applying && !saving) void api.dnsFilter().then(setFilter).catch(() => undefined); }, 10_000); return () => window.clearInterval(timer); }, [applying, saving]);
  const busy = saving || applying || confirmBusy;
  useEffect(() => {
    const escape = (event: KeyboardEvent) => { if (event.key === "Escape" && !event.defaultPrevented && !document.querySelector('[data-slot="select-content"]') && !saving && !applying && !confirmBusy) { setUpstream(null); setRewriteEditor(null); setConfirm(null); } };
    window.addEventListener("keydown", escape); return () => window.removeEventListener("keydown", escape);
  }, [saving, applying, confirmBusy]);
  const persist = async (patch: Partial<OpenBoxProfile["dns"]>, restart = true, message = T.dfSaved as string) => {
    setSaving(true); try { setProfile(await api.saveProfile({ dns: patch })); if (restart) setPending(true); onToast(message); return true; } catch (reason) { onToast(dnsError(reason)); return false; } finally { setSaving(false); }
  };
  const apply = async (update = false, id = "") => { setApplying(true); onToast(update ? T.dfUpdating : T.dfApplying); try { await api.applyDnsFilter(update, id); await load(); setPending(false); onToast(T.dfApplied); } catch (reason) { onToast(dnsError(reason)); } finally { setApplying(false); } };
  const flush = async () => { setFlushing(true); try { await api.flushDns(); onToast(T.flushDNSCacheSuccess); } catch (reason) { onToast(dnsError(reason)); } finally { setFlushing(false); } };
  if (error) return <ErrorState title="DNS 设置加载失败" error={error} onRetry={() => void load().catch(setError)} />;
  if (!profile || !filter) return <div className="surface dns-loading"><span className="loading-spinner" />正在读取 DNS 设置…</div>;
  const dns = profile.dns, rows = upstreamRows(dns), rewrite = dns.rewrite || { enabled: true, initialized: 1, rules: [] };
  const mode = platform === "systemd" && (!dns.mode || dns.mode === "dnsmasq") ? "hijack" : dns.mode || "dnsmasq";
  const saveRewrites = (rules: DnsRewriteRule[], enabled = rewrite.enabled !== false) => { const next = { enabled, initialized: 1, rules }; const restart = rewriteNeedsRestart(rewrite, next); return persist({ rewrite: next }, restart, restart ? T.dnsRewriteSavedRestart : T.dnsRewriteSavedLive); };
  const ask = (title: string, text: string, run: () => Promise<void>) => setConfirm({ title, text, run });
  const test = async (row: UpstreamRow) => {
    const key = `${row.side}-${row.index}`; setTesting(current => [...current, key]);
    try { const result = await api.testDnsUpstream({ side: row.side, server: row.server, protocol: row.protocol, port: row.port }); onToast(result.ok ? dnsMessage("dnsUpstreamTestOk", { side: row.side === "direct" ? T.dnsUpstreamDirect : T.dnsUpstreamProxy, detail: testDetail(result) }) : dnsMessage("dnsUpstreamTestFailed", { side: row.server, message: result.error || "测试失败" })); } catch (reason) { onToast(dnsError(reason)); } finally { setTesting(current => current.filter(item => item !== key)); }
  };
  return <>
    {actions && createPortal(<IconButton label={T.flushDNSCache} tooltip portalTooltip disabled={flushing || busy} onClick={() => void flush()}>{flushing ? <span className="loading-spinner" /> : <TrashIcon />}</IconButton>, actions)}
    <div className="dns-settings routing-policy-stack">
      {(pending || filter.pending) && <div className="dns-pending"><span>{T.dfApplyHint}</span><button type="button" className="primary-button" disabled={busy} onClick={() => void apply()}>{applying && <span className="loading-spinner" />}{T.dfApply}</button></div>}
      <section className="surface dns-card dns-mode-card"><header><div><h2>{T.dnsModeTitle}<small>（{dnsMessage("dnsModePort", { port: 7853 })}）</small></h2><p className="dns-hint">{T.dnsModeDescription}</p></div><SelectControl label={T.dnsModeTitle} className="dns-mode-select" value={mode} disabled={busy} options={[...(platform === "systemd" ? [] : [{ value: "dnsmasq", label: T.dnsModeDnsmasq }]), { value: "hijack", label: T.dnsModeHijack }, { value: "off", label: T.dnsModeOff }]} onValueChange={value => void persist({ mode: value }, true, T.dnsModeSaved)} /></header>
        <p className="dns-hint">{mode === "dnsmasq" ? T.dnsModeDnsmasqNote : mode === "hijack" ? T.dnsModeHijackNote : T.dnsModeOffNote}</p><p className="dns-hint">{dnsMessage("dnsModeUpstreamHint", { addr: `${location.hostname}:7853` })}</p>
        {mode !== "off" && <div className="dns-fake-row"><SwitchControl label={T.dnsFakeIpTitle} checked={dns.fakeIpForProxy === true} disabled={busy} onCheckedChange={value => void persist({ fakeIpForProxy: value }, true, T.dnsModeSaved)} /><div><span>{T.dnsFakeIpTitle}</span><p className="dns-hint">{T.dnsFakeIpNote}</p></div></div>}
      </section>
      <section className="surface dns-card"><header><h2>{T.dnsUpstreamTitle}</h2><div className="dns-actions"><IconButton label={T.dnsUpstreamRestore} tooltip portalTooltip disabled={busy} onClick={() => ask(T.dnsUpstreamRestore, T.dnsUpstreamRestoreConfirm, async () => { if (JSON.stringify(upstreamPatch(rows)) === JSON.stringify(upstreamPatch(defaultUpstreams))) onToast(T.dnsUpstreamAlreadyDefault); else await persist(upstreamPatch(defaultUpstreams), true, T.dnsUpstreamRestored); })}><ArrowUturnLeftIcon /></IconButton><IconButton label={T.dnsUpstreamAdd} className="dns-add-button" tooltip portalTooltip disabled={busy} onClick={() => setUpstream({})}><PlusIcon /></IconButton></div></header><p className="dns-hint dns-description">{T.dnsUpstreamDescription}</p>
        <div className="dns-upstream-rows">{rows.map(row => <div className="dns-upstream-row" key={`${row.side}-${row.index}`}><span className="dns-badge">{row.side === "direct" ? T.dnsUpstreamDirect : T.dnsUpstreamProxy}</span><span className="dns-mono"><span className="dns-hint">{row.protocol.toUpperCase()}</span> {row.server}:{row.port}</span>{row.index > 0 && <small className="dns-hint">{T.dnsUpstreamBackupTag}</small>}<div className="dns-actions"><IconButton label={T.dnsUpstreamTest} tooltip portalTooltip disabled={busy || testing.includes(`${row.side}-${row.index}`)} onClick={() => void test(row)}>{testing.includes(`${row.side}-${row.index}`) ? <span className="loading-spinner" /> : <BoltIcon />}</IconButton><IconButton label="修改分组" tooltip portalTooltip disabled={busy} onClick={() => setUpstream({ row })}><PencilSquareIcon /></IconButton><IconButton label={rows.filter(item => item.side === row.side).length <= 1 ? T.dnsUpstreamLastOne : "删除"} tooltip portalTooltip disabled={busy || rows.filter(item => item.side === row.side).length <= 1} onClick={() => void persist(upstreamPatch(rows.filter(item => item !== row)))}><TrashIcon /></IconButton></div></div>)}</div><p className="dns-hint">{T.dnsUpstreamExtrasNote}</p>
      </section>
      <section className="surface dns-card"><header><div className="dns-title"><SwitchControl label="启用 DNS 重写" checked={rewrite.enabled !== false} disabled={busy} onCheckedChange={value => void saveRewrites(rewrite.rules, value)} /><h2>{T.dnsRewriteTitle}</h2><span className={`dns-badge ${pending ? "warning" : rewrite.enabled !== false ? "success" : ""}`}>{pending ? T.dfPending : rewrite.enabled !== false ? T.dfEnabled : T.dfDisabled}</span></div><div className="dns-actions"><IconButton label={T.dnsRewriteRestore} tooltip portalTooltip disabled={busy} onClick={() => ask(T.dnsRewriteRestore, T.dnsRewriteRestoreConfirm, async () => { try { const next = restoreRewrites(rewrite.rules, await api.dnsRewriteDefaults()); if (JSON.stringify(next) === JSON.stringify(rewrite.rules)) onToast(T.dnsRewriteAlreadyDefault); else await saveRewrites(next); } catch (reason) { onToast(dnsError(reason)); } })}><ArrowUturnLeftIcon /></IconButton><IconButton label={T.dnsRewriteAdd} className="dns-add-button" aria-label="添加 DNS 重写" tooltip portalTooltip disabled={busy} onClick={() => setRewriteEditor({})}><PlusIcon /></IconButton></div></header><p className="dns-hint dns-description">{T.dnsRewriteDescription}</p>
        {!rewrite.rules.length && <p className="dns-hint">{T.dnsRewriteEmpty}</p>}<div>{rewrite.rules.map(rule => <div className="dns-rewrite-row" key={rule.id} data-disabled={rule.enabled === false}><SwitchControl label={`启用 ${rule.source}`} checked={rule.enabled !== false} disabled={busy} onCheckedChange={enabled => void saveRewrites(rewrite.rules.map(item => item.id === rule.id ? { ...item, enabled } : item))} /><span className="dns-mono">{rule.source} → {rule.domain || rule.addresses.join(", ")}</span><div className="dns-actions"><IconButton label="修改分组" aria-label={`编辑重写 ${rule.source}`} tooltip portalTooltip disabled={busy} onClick={() => setRewriteEditor({ rule })}><PencilSquareIcon /></IconButton><IconButton label="删除" tooltip portalTooltip disabled={busy} onClick={() => ask(T.dnsRewriteDeleteTitle, dnsMessage("dnsRewriteDeleteConfirm", { source: rule.source }), async () => { await saveRewrites(rewrite.rules.filter(item => item.id !== rule.id)); })}><TrashIcon /></IconButton></div></div>)}</div><p className="dns-hint">{T.dnsRewriteNote}</p>
      </section>
      <DnsFilterSettings status={filter} busy={busy} onSave={async config => { setSaving(true); try { await api.saveDnsFilter(config); await load(); onToast(T.dfSaved); return true; } catch (reason) { onToast(dnsError(reason)); return false; } finally { setSaving(false); } }} onApply={apply} onToast={onToast} />
      <DnsQueryRecords status={filter} onToast={onToast} />
    </div>
    {upstream && <UpstreamEditor original={upstream.row} rows={rows} onClose={() => setUpstream(null)} onSave={(next, message) => persist(upstreamPatch(next), true, message)} />}
    {rewriteEditor && <RewriteEditor original={rewriteEditor.rule} rules={rewrite.rules} onClose={() => setRewriteEditor(null)} onSave={saveRewrites} />}
    {confirm && <Modal title={confirm.title} className="dns-dialog routing-dialog" onClose={() => { if (!busy) setConfirm(null); }} footer={<><button type="button" className="compact-button" disabled={busy} onClick={() => setConfirm(null)}>取消</button><button type="button" className="primary-button" disabled={busy} onClick={() => { setConfirmBusy(true); void confirm.run().then(() => setConfirm(null)).finally(() => setConfirmBusy(false)); }}>确定</button></>}><p>{confirm.text}</p></Modal>}
  </>;
}
