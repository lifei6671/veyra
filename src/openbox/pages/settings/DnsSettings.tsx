import { useEffect, useState } from "react";
import { ArrowPathIcon } from "@heroicons/react/24/outline";
import { api } from "../../api/client";
import type { DnsFilterConfig, OpenBoxProfile } from "../../api/types";
import { ErrorState, SettingRow } from "../../components/shared";
import { SelectControl, SwitchControl } from "../../ui/controls";

export function DnsSettings({ onToast }: { onToast: (message: string) => void }) {
  const [profile, setProfile] = useState<OpenBoxProfile | null>(null);
  const [filter, setFilter] = useState<DnsFilterConfig>({});
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const load = async () => {
    setError(null);
    try { const [profileValue, filterValue] = await Promise.all([api.profile(), api.dnsFilter()]); setProfile(profileValue); setFilter(filterValue); }
    catch (reason) { setError(reason); }
  };
  useEffect(() => { void load(); }, []);

  const save = async () => {
    if (!profile) return;
    setBusy(true);
    try { const [next] = await Promise.all([api.saveProfile(profile), api.saveDnsFilter(filter)]); setProfile(next); onToast("DNS 设置已保存，重启内核后应用"); }
    catch (reason) { onToast(reason instanceof Error ? reason.message : "保存失败"); }
    finally { setBusy(false); }
  };

  if (error) return <ErrorState title="DNS 设置加载失败" error={error} onRetry={() => void load()} />;
  if (!profile) return <section className="surface settings-placeholder">正在读取 DNS 设置…</section>;
  const updateDns = (patch: Partial<OpenBoxProfile["dns"]>) => setProfile({ ...profile, dns: { ...profile.dns, ...patch } });
  return <div className="settings-content-stack"><section className="surface setting-card"><header><div><h2>DNS 设置</h2><p>配置直连和代理 DNS，保存后由后端生成 sing-box 配置。</p></div></header>
    <SettingRow label="DNS 分流" note="按路由选择直连或代理 DNS"><SwitchControl label="DNS 分流" checked={profile.dns.split} onCheckedChange={split => updateDns({ split })} /></SettingRow>
    <SettingRow label="DNS 模式"><SelectControl label="DNS 模式" value={profile.dns.mode} onValueChange={mode => updateDns({ mode })} options={[{ value: "default", label: "默认" }, { value: "fakeip", label: "FakeIP" }, { value: "hosts", label: "Hosts" }]} /></SettingRow>
    <SettingRow label="代理域名使用 FakeIP"><SwitchControl label="代理域名使用 FakeIP" checked={profile.dns.fakeIpForProxy} onCheckedChange={fakeIpForProxy => updateDns({ fakeIpForProxy })} /></SettingRow>
    <SettingRow label="直连 DNS"><input value={profile.dns.direct} onChange={event => updateDns({ direct: event.target.value })} /></SettingRow>
    <SettingRow label="代理 DNS"><input value={profile.dns.proxy} onChange={event => updateDns({ proxy: event.target.value })} /></SettingRow>
    <SettingRow label="DNS 过滤"><SwitchControl label="DNS 过滤" checked={Boolean(filter.enabled)} onCheckedChange={enabled => setFilter(current => ({ ...current, enabled }))} /></SettingRow>
  </section><div className="settings-footer-actions"><button type="button" className="compact-button" disabled={busy} onClick={() => void api.flushDns().then(() => onToast("DNS 缓存已刷新")).catch(reason => onToast(reason instanceof Error ? reason.message : "刷新失败"))}><ArrowPathIcon />刷新缓存</button><button type="button" className="primary-button" disabled={busy} onClick={() => void save()}>{busy ? "保存中…" : "保存 DNS 设置"}</button></div></div>;
}
