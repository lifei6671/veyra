import { useEffect, useState } from "react";
import { api } from "../../api/client";
import type { GroupsResponse, OpenBoxProfile } from "../../api/types";
import { ErrorState, SettingRow } from "../../components/shared";
import { SelectControl, SwitchControl } from "../../ui/controls";

export function RoutingSettings({ onToast }: { onToast: (message: string) => void }) {
  const [profile, setProfile] = useState<OpenBoxProfile | null>(null);
  const [groups, setGroups] = useState<GroupsResponse | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);

  const load = async () => {
    setError(null);
    try { const [profileValue, groupValue] = await Promise.all([api.profile(), api.groups()]); setProfile(profileValue); setGroups(groupValue); }
    catch (reason) { setError(reason); }
  };
  useEffect(() => { void load(); }, []);

  const patch = (value: Partial<OpenBoxProfile>) => setProfile(current => current ? { ...current, ...value } : current);
  const save = async () => {
    if (!profile) return;
    setBusy(true);
    try { setProfile(await api.saveProfile(profile)); onToast("目标分流配置已保存，重启内核后应用"); }
    catch (reason) { onToast(reason instanceof Error ? reason.message : "保存失败"); }
    finally { setBusy(false); }
  };

  if (error) return <ErrorState title="目标分流加载失败" error={error} onRetry={() => void load()} />;
  if (!profile) return <section className="surface settings-placeholder">正在读取目标分流…</section>;
  const routeOptions = ["直连", "拒绝", ...(groups?.availableGroups ?? [])].map(value => ({ value, label: value }));

  return <div className="settings-content-stack">
    <section className="surface setting-card"><header><div><h2>基础分流</h2><p>这些选项会写入 Open-Box Profile，并在内核重启时生成配置。</p></div></header>
      <SettingRow label="启用 IPv6" note="允许生成 IPv6 路由与 DNS 配置"><SwitchControl label="启用 IPv6" checked={profile.ipv6} onCheckedChange={ipv6 => patch({ ipv6 })} /></SettingRow>
      <SettingRow label="节点地址直连" note="连接节点服务器时绕过代理链"><SwitchControl label="节点地址直连" checked={profile.directForNodes} onCheckedChange={directForNodes => patch({ directForNodes })} /></SettingRow>
      <SettingRow label="拒绝 QUIC" note="拒绝命中分流策略的 UDP/443 流量"><SwitchControl label="拒绝 QUIC" checked={profile.rejectQuic} onCheckedChange={rejectQuic => patch({ rejectQuic })} /></SettingRow>
      <SettingRow label="默认分流" note="未命中策略时使用的出站"><SelectControl label="默认分流" value={profile.routing.fallbackDefault || "直连"} onValueChange={fallbackDefault => setProfile({ ...profile, routing: { ...profile.routing, fallbackDefault } })} options={routeOptions} /></SettingRow>
    </section>
    <section className="surface settings-list-block"><header><div><h2>当前策略</h2><p>{profile.routing.policies.length} 条目标分流策略</p></div></header><div className="routing-policy-list">{profile.routing.policies.map((policy, index) => <article key={`${policy.name ?? "policy"}-${index}`}><span>{index + 1}</span><div><h3>{policy.name ?? `策略 ${index + 1}`}</h3><p>{String(policy.outbound ?? policy.target ?? policy.type ?? "已配置")}</p></div><code>{policy.enabled === false ? "已停用" : "已启用"}</code></article>)}</div></section>
    <div className="settings-footer-actions"><button type="button" className="primary-button" disabled={busy} onClick={() => void save()}>{busy ? "保存中…" : "保存目标分流"}</button></div>
  </div>;
}
