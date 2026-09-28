import { useMemo, useRef, useState } from "react";
import { ArrowDownTrayIcon, ClipboardDocumentIcon, PauseIcon, PlayIcon, TrashIcon } from "@heroicons/react/24/outline";
import type { ControllerLogFrame } from "../api/types";
import { useControllerSocket } from "../hooks/useControllerSocket";
import { matchesQuery } from "../lib/format";
import { SearchInputGroup, SelectControl } from "../ui/controls";
import { EmptyState, IconButton, PageToolbar } from "../components/shared";

type LiveLog = ControllerLogFrame & { id: number; time: string; category: string };

export function LogsPage({ onToast }: { onToast: (message: string) => void }) {
  const [items, setItems] = useState<LiveLog[]>([]);
  const [level, setLevel] = useState("info");
  const [category, setCategory] = useState("all");
  const [query, setQuery] = useState("");
  const [paused, setPaused] = useState(false);
  const pausedRef = useRef(paused);
  pausedRef.current = paused;
  const counter = useRef(0);

  const socket = useControllerSocket<ControllerLogFrame>("logs", {
    query: { level },
    onMessage: value => {
      if (pausedRef.current) return;
      const item: LiveLog = { ...value, id: ++counter.current, time: new Date().toLocaleTimeString("zh-CN", { hour12: false }), category: logCategory(value.payload) };
      setItems(current => [item, ...current].slice(0, 1000));
    },
  });

  const categories = useMemo(() => ["all", ...new Set(items.map(item => item.category))], [items]);
  const visible = items.filter(item => (category === "all" || item.category === category) && matchesQuery([item.type, item.category, item.payload], query));
  const changeLevel = (value: string) => {
    setLevel(value);
    setCategory("all");
    setItems([]);
  };

  const exportLogs = () => {
    const text = [...visible].reverse().map(item => `${item.time} [${item.type}] [${item.category}] ${item.payload}`).join("\n");
    const url = URL.createObjectURL(new Blob([text], { type: "text/plain;charset=utf-8" }));
    const link = document.createElement("a");
    link.href = url;
    link.download = `openbox-${Date.now()}.log`;
    link.click();
    URL.revokeObjectURL(url);
    onToast("当前日志已导出");
  };

  return <main className="page logs-page">
    <PageToolbar title="日志" subtitle={`实时日志 WebSocket：${socket.state === "open" ? "已连接" : socket.state === "connecting" ? "连接中" : "已断开"}`} />
    <div className="route-controls log-controls"><SelectControl label="日志级别" value={level} onValueChange={changeLevel} className="log-level-select" options={["debug", "info", "warning", "error"].map(value => ({ value, label: value }))} /><SelectControl label="日志分类" value={category} onValueChange={setCategory} className="log-category-select" options={categories.map(value => ({ value, label: value === "all" ? "全部" : value }))} /><SearchInputGroup label="搜索日志" value={query} onChange={setQuery} onClear={() => setQuery("")} placeholder="搜索日志" /><div className="route-tools"><IconButton label="下载日志" disabled={!visible.length} onClick={exportLogs}><ArrowDownTrayIcon /></IconButton><IconButton label={paused ? "继续接收" : "暂停接收"} active={paused} onClick={() => setPaused(value => !value)}>{paused ? <PlayIcon /> : <PauseIcon />}</IconButton><IconButton label="清空当前日志" disabled={!items.length} onClick={() => setItems([])}><TrashIcon /></IconButton></div></div>
    <section className="log-list" aria-live={paused ? "off" : "polite"}>{visible.map(item => <article className="log-row" key={item.id}><span className="log-number">{item.id}.</span><time>{item.time}</time><span className={`level level-${item.type}`}>{item.type}</span><span className="log-category">{item.category}</span><p>{item.payload}</p><button type="button" aria-label="复制日志" onClick={() => void navigator.clipboard.writeText(item.payload).then(() => onToast("日志内容已复制"))}><ClipboardDocumentIcon /></button></article>)}</section>
    {!visible.length && <EmptyState icon="≡" title={items.length ? "没有匹配的日志" : "等待实时日志"} text={items.length ? "调整筛选条件。" : "后端产生新日志时会立即显示在这里。"} />}
  </main>;
}

function logCategory(payload: string) {
  const match = payload.match(/\b(inbound|outbound|dns|route|router|transport|service|subscription|latency)\b/i);
  return match?.[1]?.toLocaleLowerCase() ?? "system";
}
