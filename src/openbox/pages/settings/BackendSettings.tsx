import { useEffect, useState } from "react";
import { ArrowPathIcon, PlayIcon, StopIcon } from "@heroicons/react/24/outline";
import { api } from "../../api/client";
import type { OpenBoxProfile, ServiceStatus, UpdateStatus } from "../../api/types";
import { ErrorState, IconButton, SettingRow } from "../../components/shared";
import { SelectControl, SwitchControl } from "../../ui/controls";

export function BackendSettings({ onToast }: { onToast: (message: string) => void }) {
  const [service, setService] = useState<ServiceStatus | null>(null);
  const [update, setUpdate] = useState<UpdateStatus | null>(null);
  const [profile, setProfile] = useState<OpenBoxProfile | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const load = async () => {
    setError(null);
    try { const [serviceValue, updateValue, profileValue] = await Promise.all([api.serviceStatus(), api.updateStatus(), api.profile()]); setService(serviceValue); setUpdate(updateValue); setProfile(profileValue); }
    catch (reason) { setError(reason); }
  };
  useEffect(() => { void load(); }, []);

  const action = async (value: "start" | "stop" | "restart") => {
    setBusy(true);
    try { await api.serviceAction(value); await load(); onToast(`内核${value === "start" ? "启动" : value === "stop" ? "停止" : "重启"}命令已执行`); }
    catch (reason) { onToast(reason instanceof Error ? reason.message : "操作失败"); }
    finally { setBusy(false); }
  };
  const saveProfile = async () => {
    if (!profile) return;
    setBusy(true);
    try { setProfile(await api.saveProfile(profile)); onToast("后端设置已保存"); }
    catch (reason) { onToast(reason instanceof Error ? reason.message : "保存失败"); }
    finally { setBusy(false); }
  };

  if (error) return <ErrorState title="后端设置加载失败" error={error} onRetry={() => void load()} />;
  return <div className="settings-content-stack">
    <section className="surface backend-version"><div><strong>{update?.version ?? "—"}</strong><span>Open-Box</span></div><div><strong>{update?.singboxVersion ?? "—"}</strong><span>内核版本</span></div><div><strong>{update ? `${update.geoCounts.geosite} / ${update.geoCounts.geoip}` : "—"}</strong><span>GeoSite / GeoIP</span></div></section>
    <section className="surface setting-card"><header><div><h2>内核服务</h2><p>启动和重启会按当前 Profile 重新生成配置。</p></div><IconButton label="刷新服务状态" onClick={() => void load()}><ArrowPathIcon /></IconButton></header>
      <SettingRow label="内核"><strong className={service?.core.running ? "success" : "danger"}>{service?.core.running ? "运行中" : "已停止"}</strong></SettingRow>
      <SettingRow label="面板"><strong className={service?.panel.running ? "success" : "danger"}>{service?.panel.running ? "运行中" : "已停止"}</strong></SettingRow>
      <SettingRow label="开机自启"><strong>{service?.core.autostart ? "开启" : "关闭"}</strong></SettingRow>
      <div className="service-actions"><button type="button" className="compact-button" disabled={busy || service?.core.running} onClick={() => void action("start")}><PlayIcon />启动</button><button type="button" className="compact-button" disabled={busy || !service?.core.running} onClick={() => void action("stop")}><StopIcon />停止</button><button type="button" className="primary-button" disabled={busy} onClick={() => void action("restart")}><ArrowPathIcon />重启</button></div>
    </section>
    {profile && <section className="surface setting-card"><header><div><h2>更新与历史</h2><p>设置由后端持久化。</p></div></header><SettingRow label="自动更新 Open-Box"><SwitchControl label="自动更新 Open-Box" checked={profile.updates.openbox.auto} onCheckedChange={auto => setProfile({ ...profile, updates: { ...profile.updates, openbox: { ...profile.updates.openbox, auto } } })} /></SettingRow><SettingRow label="更新通道"><SelectControl label="更新通道" value={profile.updates.openbox.channel} onValueChange={channel => setProfile({ ...profile, updates: { ...profile.updates, openbox: { ...profile.updates.openbox, channel } } })} options={[{ value: "stable", label: "稳定版" }, { value: "direct", label: "直连" }, { value: "github", label: "GitHub" }]} /></SettingRow><SettingRow label="流量保留"><label className="number-field"><input type="number" min={1} max={24} value={profile.traffic.keepMonths} onChange={event => setProfile({ ...profile, traffic: { keepMonths: event.target.valueAsNumber } })} /><span>月</span></label></SettingRow><div className="settings-footer-actions"><button type="button" className="primary-button" disabled={busy} onClick={() => void saveProfile()}>保存后端设置</button></div></section>}
  </div>;
}
