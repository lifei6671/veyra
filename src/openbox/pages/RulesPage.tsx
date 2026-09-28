import { useEffect, useMemo, useState } from "react";
import { ArrowPathIcon, FunnelIcon } from "@heroicons/react/24/outline";
import { api } from "../api/client";
import type { ControllerRule } from "../api/types";
import { matchesQuery } from "../lib/format";
import { SearchInputGroup, SegmentedGroup, SegmentedItem } from "../ui/controls";
import { EmptyState, ErrorState, IconButton, PageToolbar } from "../components/shared";

export function RulesPage() {
  const [items, setItems] = useState<ControllerRule[]>([]);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<"all" | "route" | "reject">("all");
  const [error, setError] = useState<unknown>(null);
  const [loading, setLoading] = useState(false);

  const load = async () => {
    setLoading(true);
    setError(null);
    try { setItems((await api.rules()).rules); } catch (reason) { setError(reason); } finally { setLoading(false); }
  };
  useEffect(() => { void load(); }, []);

  const visible = useMemo(() => items.filter(item => {
    const isReject = /reject/i.test(item.proxy);
    return (filter === "all" || (filter === "reject" ? isReject : !isReject)) && matchesQuery([item.type, item.payload, item.proxy], query);
  }), [filter, items, query]);

  return <main className="page rules-page">
    <PageToolbar title="规则" subtitle={`${items.length} 条运行中规则`}><IconButton label="刷新规则" disabled={loading} onClick={() => void load()}><ArrowPathIcon /></IconButton></PageToolbar>
    <div className="route-controls"><SegmentedGroup value={filter} onValueChange={value => setFilter(value as typeof filter)} aria-label="规则类型"><SegmentedItem value="all">全部 ({items.length})</SegmentedItem><SegmentedItem value="route">分流</SegmentedItem><SegmentedItem value="reject">拒绝</SegmentedItem></SegmentedGroup><SearchInputGroup label="搜索规则" value={query} onChange={setQuery} onClear={() => setQuery("")} placeholder="搜索规则、目标或类型" /></div>
    {Boolean(error) && <ErrorState title="规则加载失败" error={error} onRetry={() => void load()} />}
    <section className="rule-list">{visible.map((item, index) => <article className="surface rule-row" key={`${index}-${item.type}-${item.proxy}`}><div className="rule-main"><span className="rule-index">{index + 1}.</span><span className="rule-type">{item.type}</span><code>{item.payload || "默认规则"}</code></div><div className={`rule-result ${/reject/i.test(item.proxy) ? "reject" : ""}`}>{item.proxy}</div></article>)}</section>
    {!visible.length && <EmptyState icon={<FunnelIcon />} title="没有匹配的规则" text="试试其他关键词或取消筛选。" />}
  </main>;
}
