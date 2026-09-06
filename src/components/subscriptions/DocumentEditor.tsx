import { useEffect, useRef, useState } from "react";
import "monaco-editor/editor/editor.main";
import * as monaco from "monaco-editor/editor/editor.api";
import "monaco-editor/languages/definitions/yaml/register";
import { jsonDefaults } from "monaco-editor/languages/features/json/register";
import EditorWorker from "monaco-editor/editor/editor.worker?worker";
import JsonWorker from "monaco-editor/language/json/json.worker?worker";
import "./DocumentEditor.css";

globalThis.MonacoEnvironment = { getWorker: (_module, label) => label === "json" ? new JsonWorker() : new EditorWorker() };
jsonDefaults.setDiagnosticsOptions({ validate: true, enableSchemaRequest: false, schemas: [] });

type Props = {
  title: string;
  content: string;
  format: "json" | "yaml";
  readOnly?: boolean;
  onSave: (content: string) => Promise<void>;
  onFormat: (content: string) => Promise<string>;
  onClose: () => void;
};

export default function DocumentEditor({ title, content, format, readOnly = false, onSave, onFormat, onClose }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const dialog = useRef<HTMLElement>(null);
  const instance = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const [busy, setBusy] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const busyRef = useRef(false);
  const mounted = useRef(true);
  const actions = useRef({ onSave, onFormat, onClose });
  useEffect(() => { actions.current = { onSave, onFormat, onClose }; }, [onSave, onFormat, onClose]);

  useEffect(() => {
    mounted.current = true;
    const returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const model = monaco.editor.createModel(content, format);
    const editor = monaco.editor.create(host.current!, {
      model, readOnly, automaticLayout: true, theme: media.matches ? "vs-dark" : "vs",
      minimap: { enabled: true }, fontSize: 14, tabSize: 2, scrollBeyondLastLine: false,
      wordWrap: "off", ariaLabel: `${format.toUpperCase()} 配置编辑器`,
    });
    instance.current = editor;
    editor.focus();
    const changes = model.onDidChangeContent(() => setDirty(model.getValue() !== content));
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => { if (!readOnly) void save(); });
    const theme = () => monaco.editor.setTheme(media.matches ? "vs-dark" : "vs");
    media.addEventListener("change", theme);
    return () => {
      mounted.current = false;
      media.removeEventListener("change", theme);
      changes.dispose(); editor.dispose(); model.dispose(); instance.current = null;
      returnFocus?.focus();
    };
  }, [content, format, readOnly]);

  useEffect(() => { instance.current?.updateOptions({ readOnly: readOnly || busy }); }, [busy, readOnly]);
  useEffect(() => {
    if (notice === null) return;
    const timer = window.setTimeout(() => setNotice(null), 6000);
    return () => window.clearTimeout(timer);
  }, [notice]);

  async function perform(action: () => Promise<void>) {
    if (busyRef.current) return;
    busyRef.current = true; setBusy(true); setNotice(null);
    try { await action(); }
    catch (error) { if (mounted.current) setNotice(error instanceof Error ? error.message : "操作失败，请重试"); }
    finally { busyRef.current = false; if (mounted.current) setBusy(false); }
  }
  async function save() {
    if (readOnly || instance.current?.getValue() === content) return;
    await perform(async () => {
      await actions.current.onSave(instance.current!.getValue());
      if (mounted.current) actions.current.onClose();
    });
  }
  async function formatDocument() {
    await perform(async () => {
      const value = await actions.current.onFormat(instance.current!.getValue());
      if (!mounted.current) return;
      const editor = instance.current!;
      editor.pushUndoStop();
      const model = editor.getModel()!;
      model.pushEditOperations([], [{ range: model.getFullModelRange(), text: value }], () => null);
      editor.pushUndoStop(); editor.focus();
    });
  }

  return <div className="document-backdrop" onClick={(event) => { if (event.target === event.currentTarget && !busy) onClose(); }}>
    <section ref={dialog} className={`document-dialog${fullscreen ? " document-fullscreen" : ""}`} role="dialog" aria-modal="true" aria-labelledby="document-title" aria-busy={busy} onKeyDown={(event) => {
      if (event.key === "Escape" && !busy) { event.stopPropagation(); onClose(); }
      if (event.key === "Tab") {
        const controls = Array.from(dialog.current!.querySelectorAll<HTMLElement>("button:not(:disabled), textarea, [tabindex='0']")).filter(el => el.getClientRects().length > 0);
        const first = controls[0], last = controls.at(-1);
        if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
        else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
      }
    }}>
      <header><h2 id="document-title">{title}</h2><span>{format.toUpperCase()}</span></header>
      <div className="document-editor-host" ref={host} />
      <footer>
        <div className="document-tools">
          <button type="button" title="复制" aria-label="复制文件内容" disabled={busy} onClick={() => void perform(async () => { await navigator.clipboard.writeText(instance.current!.getValue()); })}><svg viewBox="0 0 24 24"><path d="M9 5H5v16h14V5h-4M9 3h6v4H9z" /></svg></button>
          {!readOnly && <button type="button" title="格式化" aria-label="格式化文件" disabled={busy} onClick={() => void formatDocument()}><svg viewBox="0 0 24 24"><path d="M3 4h15v6H3zM18 7h3v6h-9v8H9v-8" /></svg></button>}
          <button type="button" title={fullscreen ? "退出全屏" : "全屏"} aria-label={fullscreen ? "退出编辑器全屏" : "编辑器全屏"} aria-pressed={fullscreen} disabled={busy} onClick={() => setFullscreen(value => !value)}><svg viewBox="0 0 24 24"><path d="M8 3H3v5M16 21h5v-5M3 3l7 7m11 11-7-7M16 3h5v5M8 21H3v-5" /></svg></button>
        </div>
        <div className="document-actions"><button type="button" className="secondary-button" disabled={busy} onClick={onClose}>{readOnly ? "关闭" : "取消"}</button>{!readOnly && <button type="button" className="primary-button" disabled={busy || !dirty} onClick={() => void save()}>{busy ? "处理中" : "保存"}</button>}</div>
      </footer>
      {notice && <div className="subscription-notice subscription-notice-error" role="alert">{notice}</div>}
    </section>
  </div>;
}
