import { useEffect, useState } from "react";
import { PlusIcon, TrashIcon } from "@heroicons/react/24/outline";
import { api } from "../../api/client";
import type { ClientDevice, OpenBoxProfile } from "../../api/types";
import { EmptyState, ErrorState, IconButton, Modal } from "../../components/shared";

type StructuredSection = "clients" | "chain" | "share";

const copy: Record<StructuredSection, { title: string; description: string; field: "clientRoutes" | "chainProxies" | "servers" }> = {
  clients: { title: "终端分流", description: "管理终端配置与按终端应用的路由规则。", field: "clientRoutes" },
  chain: { title: "链式代理", description: "编辑后端 Profile 中的链式代理定义。", field: "chainProxies" },
  share: { title: "共享网络", description: "管理供局域网设备使用的入站服务器定义。", field: "servers" },
};

export function StructuredSettings({ section, onToast }: { section: StructuredSection; onToast: (message: string) => void }) {
  const info = copy[section];
  const [profile, setProfile] = useState<OpenBoxProfile | null>(null);
  const [devices, setDevices] = useState<ClientDevice[]>([]);
  const [source, setSource] = useState("[]");
  const [deviceName, setDeviceName] = useState<string | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [validation, setValidation] = useState("");
  const [busy, setBusy] = useState(false);

  const load = async () => {
    setError(null);
    try {
      const [profileValue, deviceValues] = await Promise.all([api.profile(), section === "clients" ? api.devices().catch(() => []) : Promise.resolve([])]);
      setProfile(profileValue);
      setDevices(deviceValues);
      setSource(JSON.stringify(profileValue[info.field] ?? [], null, 2));
    } catch (reason) { setError(reason); }
  };
  useEffect(() => { void load(); }, [section]);

  const save = async () => {
    if (!profile) return;
    let parsed: Array<Record<string, unknown>>;
    try {
      const value = JSON.parse(source) as unknown;
      if (!Array.isArray(value)) throw new Error("配置必须是 JSON 数组");
      parsed = value as Array<Record<string, unknown>>;
    } catch (reason) {
      setValidation(reason instanceof Error ? reason.message : "JSON 格式不正确");
      return;
    }
    setValidation("");
    setBusy(true);
    try {
      const next = await api.saveProfile({ ...profile, [info.field]: parsed });
      setProfile(next);
      setSource(JSON.stringify(next[info.field] ?? [], null, 2));
      onToast(`${info.title}已保存，重启内核后应用`);
    } catch (reason) { onToast(reason instanceof Error ? reason.message : "保存失败"); }
    finally { setBusy(false); }
  };

  const createDevice = async () => {
    if (!deviceName?.trim()) return;
    setBusy(true);
    try { await api.createDevice(deviceName.trim()); setDeviceName(null); setDevices(await api.devices()); onToast("终端配置已创建"); }
    catch (reason) { onToast(reason instanceof Error ? reason.message : "创建失败"); }
    finally { setBusy(false); }
  };

  const removeDevice = async (name: string) => {
    if (!window.confirm(`确定删除终端“${name}”吗？`)) return;
    setBusy(true);
    try { await api.deleteDevice(name); setDevices(await api.devices()); onToast("终端已删除"); }
    catch (reason) { onToast(reason instanceof Error ? reason.message : "删除失败"); }
    finally { setBusy(false); }
  };

  if (error) return <ErrorState title={`${info.title}加载失败`} error={error} onRetry={() => void load()} />;
  return <div className="settings-content-stack">
    {section === "clients" && <section className="surface settings-list-block"><header><div><h2>终端配置</h2><p>生成并管理真实的终端配置文件。</p></div><button type="button" className="compact-button" onClick={() => setDeviceName("")}><PlusIcon />新增终端</button></header><div className="settings-card-list">{devices.map(device => <article className="settings-data-card" key={device.name}><div><h3>{device.name}</h3><p>配置由 Open-Box 后端生成</p></div><IconButton label={`删除 ${device.name}`} disabled={busy} onClick={() => void removeDevice(device.name)}><TrashIcon /></IconButton></article>)}</div>{!devices.length && <EmptyState icon="◇" title="没有终端配置" text="新增终端后可以生成独立配置。" compact />}</section>}
    <section className="surface settings-json-block"><header><div><h2>{info.title}</h2><p>{info.description}</p></div></header><label><span>后端 Profile JSON</span><textarea spellCheck={false} value={source} onChange={event => setSource(event.target.value)} /></label>{validation && <p className="field-error" role="alert">{validation}</p>}<div className="settings-footer-actions"><button type="button" className="primary-button" disabled={busy || !profile} onClick={() => void save()}>{busy ? "保存中…" : `保存${info.title}`}</button></div></section>
    {deviceName !== null && <Modal title="新增终端" onClose={() => setDeviceName(null)} footer={<><button type="button" className="compact-button" onClick={() => setDeviceName(null)}>取消</button><button type="button" className="primary-button" disabled={busy || !deviceName.trim()} onClick={() => void createDevice()}>创建</button></>}><div className="form-stack"><label><span>终端名称</span><input autoFocus value={deviceName} onChange={event => setDeviceName(event.target.value)} placeholder="例如：客厅电视" /></label></div></Modal>}
  </div>;
}
