import { useEffect, useState } from "react";
import { api } from "../../api/client";
import type { OpenBoxProfile } from "../../api/types";
import { ErrorState } from "../../components/shared";

type StructuredSection = "share";

const copy: Record<StructuredSection, { title: string; description: string; field: "servers" }> = {
  share: { title: "共享网络", description: "管理供局域网设备使用的入站服务器定义。", field: "servers" },
};

export function StructuredSettings({ section, onToast }: { section: StructuredSection; onToast: (message: string) => void }) {
  const info = copy[section];
  const [profile, setProfile] = useState<OpenBoxProfile | null>(null);
  const [source, setSource] = useState("[]");
  const [error, setError] = useState<unknown>(null);
  const [validation, setValidation] = useState("");
  const [busy, setBusy] = useState(false);

  const load = async () => {
    setError(null);
    try {
      const profileValue = await api.profile();
      setProfile(profileValue);
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

  if (error) return <ErrorState title={`${info.title}加载失败`} error={error} onRetry={() => void load()} />;
  return <div className="settings-content-stack">
    <section className="surface settings-json-block"><header><div><h2>{info.title}</h2><p>{info.description}</p></div></header><label><span>后端 Profile JSON</span><textarea spellCheck={false} value={source} onChange={event => setSource(event.target.value)} /></label>{validation && <p className="field-error" role="alert">{validation}</p>}<div className="settings-footer-actions"><button type="button" className="primary-button" disabled={busy || !profile} onClick={() => void save()}>{busy ? "保存中…" : `保存${info.title}`}</button></div></section>
  </div>;
}
