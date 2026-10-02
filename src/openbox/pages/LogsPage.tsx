import { useMemo, useRef, useState } from "react";
import { ArrowDownTrayIcon, PauseIcon, PlayIcon, SparklesIcon, XMarkIcon } from "@heroicons/react/24/outline";
import type { ControllerLogFrame } from "../api/types";
import { useControllerSocket } from "../hooks/useControllerSocket";
import { SearchInputGroup, SelectControl } from "../ui/controls";
import { IconButton } from "../components/shared";

type LiveLog = ControllerLogFrame & { id: number; time: string; category: string };

const logLevels = ["trace", "debug", "info", "warning", "error", "fatal", "panic", "silent"];

export function LogsPage() {
  const [items, setItems] = useState<LiveLog[]>([]);
  const [level, setLevel] = useState("info");
  const [filter, setFilter] = useState("all");
  const [query, setQuery] = useState("");
  const [paused, setPaused] = useState(false);
  const pausedRef = useRef(paused);
  pausedRef.current = paused;
  const counter = useRef(0);

  useControllerSocket<ControllerLogFrame>("logs", {
    query: { level },
    onMessage: value => {
      const id = ++counter.current;
      if (pausedRef.current) return;
      const item: LiveLog = {
        ...value,
        id,
        time: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
        category: getLogCategory(value.payload),
      };
      setItems(current => [item, ...current].slice(0, 1000));
    },
  });

  const filters = useMemo(() => {
    const presentLevels = new Set(items.map(item => item.type));
    const presentCategories = new Set(items.map(item => item.category).filter(Boolean));
    return {
      levels: logLevels.filter(value => presentLevels.has(value)),
      categories: [...presentCategories].sort(),
    };
  }, [items]);

  const search = useMemo(() => compileLogSearch(query), [query]);
  const selectedFilterLabel = filter === "all" ? "全部" : filter.slice(filter.indexOf(":") + 1);

  const visible = items.filter(item => {
    if (search.invalid) return false;
    const matchesFilter = filter === "all"
      || (filter.startsWith("level:") && item.type === filter.slice(6))
      || (filter.startsWith("type:") && item.payload.includes(filter.slice(5)));
    const matchesSearch = !search.regex || [item.payload, item.time, item.type].some(value => search.regex?.test(value));
    return matchesFilter && matchesSearch;
  });

  const changeLevel = (value: string) => {
    setLevel(value);
    setFilter("all");
    setItems([]);
    counter.current = 0;
  };

  const exportLogs = () => {
    const text = visible.map(item => `${String(item.id).padEnd(5, " ")}\t${item.time}\t${item.type.padEnd(7, " ")}\t${item.payload}`).join("\n");
    const url = URL.createObjectURL(new Blob([text], { type: "text/plain;charset=utf-8" }));
    const link = document.createElement("a");
    link.href = url;
    link.download = `${new Date().toISOString().slice(0, 19).replace(/[T:]/g, "-")}.log`;
    link.click();
    URL.revokeObjectURL(url);
  };

  return <main className="page logs-page">
    <div className="route-controls log-controls">
      <SelectControl label="日志级别" value={level} onValueChange={changeLevel} className="log-level-select" options={logLevels.map(value => ({ value, label: value }))} />
      <div className="log-filter-cluster">
        <SelectControl className="log-category-select" label="日志类型" value={filter} placeholder={selectedFilterLabel} onValueChange={setFilter} options={[{ value: "all", label: "全部" }, ...filters.levels.map(value => ({ value: `level:${value}`, label: value, group: "日志等级" })), ...filters.categories.map(value => ({ value: `type:${value}`, label: value, group: "日志类型" }))]} />
        <SearchInputGroup label="搜索日志" value={query} onChange={setQuery} onClear={() => setQuery("")} placeholder="搜索 | Regex" className={search.invalid ? "search-invalid" : ""} />
        <IconButton label="格式化查询" onClick={() => setQuery(normalizeLogQuery(query))}><SparklesIcon /></IconButton>
      </div>
      <div className="route-tools">
        <IconButton label="下载日志" tooltip disabled={!visible.length} onClick={exportLogs}><ArrowDownTrayIcon /></IconButton>
        <IconButton label={paused ? "继续接收" : "暂停接收"} tooltip active={paused} onClick={() => setPaused(value => !value)}>{paused ? <PlayIcon /> : <PauseIcon />}</IconButton>
        <IconButton label="清空当前日志" tooltip disabled={!items.length} onClick={() => setItems([])}><XMarkIcon /></IconButton>
      </div>
    </div>
    <section className="log-list" aria-live={paused ? "off" : "polite"}>
      {visible.map(item => (
        <article className="log-row" key={item.id}>
          <span className="log-number">{String(item.id).padStart(2, "0")}.</span>
          <time>{item.time}</time>
          <span className={`level level-${item.type}`}>{item.type}</span>
          <p>{item.payload}</p>
        </article>
      ))}
      {!visible.length && <div className="surface log-empty">{search.invalid ? "正则表达式格式不正确" : items.length ? "没有匹配的日志" : "暂无日志"}</div>}
    </section>
  </main>;
}

export function getLogCategory(payload: string) {
  if (payload.startsWith("[")) {
    const contentStart = payload.indexOf("]") + 2;
    const colon = payload.indexOf(":", contentStart);
    return colon >= contentStart ? payload.slice(contentStart, colon + 1) : "";
  }
  const space = payload.indexOf(" ");
  return space === -1 ? payload : payload.slice(0, space);
}

export function compileLogSearch(query: string) {
  if (!query) return { regex: null, invalid: false };
  try {
    return { regex: new RegExp(query, "i"), invalid: false };
  } catch {
    return { regex: null, invalid: true };
  }
}

export function normalizeLogQuery(value: string) {
  const trimmed = value.trim();
  if (!trimmed) return "";
  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed)
    ? trimmed
    : trimmed.startsWith("//") ? `http:${trimmed}` : `http://${trimmed}`;
  try {
    const url = new URL(candidate);
    return url.hostname.toLocaleLowerCase();
  } catch {
    return trimmed.replace(/:\d+$/, "");
  }
}
