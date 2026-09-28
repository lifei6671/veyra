import { useEffect, useMemo, useState } from "react";
import { ArrowDownTrayIcon, BoltIcon, LinkIcon, PauseIcon, PlayIcon, XMarkIcon } from "@heroicons/react/24/outline";
import { api } from "../api/client";
import type { ConnectionsFrame, ControllerConnection } from "../api/types";
import { formatBytes, formatDateTime, formatRate, matchesQuery } from "../lib/format";
import { SearchInputGroup, SegmentedGroup, SegmentedItem, SelectControl } from "../ui/controls";
import { EmptyState, IconButton, PageToolbar } from "../components/shared";

export function ConnectionsPage({ frame, onToast }: { frame: ConnectionsFrame; onToast: (message: string) => void }) {
  const [paused, setPaused] = useState(false);
  const [snapshot, setSnapshot] = useState<ControllerConnection[]>(frame.connections);
  const [closed, setClosed] = useState<ControllerConnection[]>([]);
  const [tab, setTab] = useState<"active" | "closed">("active");
  const [query, setQuery] = useState("");
  const [chain, setChain] = useState("all");
  const [busy, setBusy] = useState(false);

  useEffect(() => { if (!paused) setSnapshot(frame.connections); }, [frame, paused]);

  const chainOptions = useMemo(() => ["all", ...new Set(snapshot.flatMap(item => item.chains))], [snapshot]);
  const source = tab === "active" ? snapshot : closed;
  const visible = source.filter(item => (chain === "all" || item.chains.includes(chain)) && matchesQuery([
    item.metadata.sourceIP, item.metadata.sourcePort, item.metadata.host, item.metadata.destinationIP,
    item.metadata.destinationPort, item.metadata.network, item.rule, item.rulePayload, ...item.chains,
  ], query));

  const closeOne = async (connection: ControllerConnection) => {
    setBusy(true);
    try {
      await api.closeConnection(connection.id);
      setClosed(items => [{ ...connection }, ...items].slice(0, 100));
      setSnapshot(items => items.filter(item => item.id !== connection.id));
      onToast("连接已关闭");
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "关闭连接失败");
    } finally {
      setBusy(false);
    }
  };

  const closeAll = async () => {
    if (!window.confirm(`确定关闭当前 ${snapshot.length} 个连接吗？`)) return;
    setBusy(true);
    try {
      await api.closeAllConnections();
      setClosed(items => [...snapshot, ...items].slice(0, 100));
      setSnapshot([]);
      onToast("全部连接已关闭");
    } catch (reason) {
      onToast(reason instanceof Error ? reason.message : "关闭全部连接失败");
    } finally {
      setBusy(false);
    }
  };

  const exportConnections = () => {
    const blob = new Blob([JSON.stringify(visible, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `openbox-connections-${Date.now()}.json`;
    link.click();
    URL.revokeObjectURL(url);
    onToast("当前连接已导出");
  };

  return <main className="page connections-page">
    <PageToolbar title="连接" subtitle={`${snapshot.length} 个活跃连接 · 数据来自实时 WebSocket`} />
    <div className="route-controls connection-controls"><SegmentedGroup className="proxy-tabs connection-tabs" value={tab} onValueChange={value => setTab(value as "active" | "closed")} aria-label="连接状态"><SegmentedItem value="active">活跃 ({snapshot.length})</SegmentedItem><SegmentedItem value="closed">已关闭 ({closed.length})</SegmentedItem></SegmentedGroup><SelectControl label="代理链" value={chain} onValueChange={setChain} options={chainOptions.map(value => ({ value, label: value === "all" ? "全部" : value }))} /><SearchInputGroup label="搜索连接" value={query} onChange={setQuery} onClear={() => setQuery("")} placeholder="搜索 | 多个关键词用空格分隔" /><IconButton label="复制当前页面链接" onClick={() => void navigator.clipboard.writeText(window.location.href).then(() => onToast("页面链接已复制"))}><LinkIcon /></IconButton><div className="route-tools"><IconButton label="导出当前连接" onClick={exportConnections}><ArrowDownTrayIcon /></IconButton><IconButton label={paused ? "继续刷新" : "暂停刷新"} active={paused} onClick={() => setPaused(value => !value)}>{paused ? <PlayIcon /> : <PauseIcon />}</IconButton><IconButton label="关闭全部连接" disabled={busy || !snapshot.length} onClick={() => void closeAll()}><XMarkIcon /></IconButton></div></div>
    <section className="connection-table" role="table"><div className="connection-head" role="row"><span>来源</span><span>代理链</span><span>目标</span><span>下载速度</span><span>上传速度</span><span>下载</span><span>上传</span><span>连接时间</span><span /></div>{visible.map(item => <ConnectionRow key={item.id} item={item} closed={tab === "closed"} busy={busy} onClose={() => void closeOne(item)} />)}</section>
    {!visible.length && <EmptyState icon={<BoltIcon />} title={tab === "closed" ? "还没有关闭记录" : "当前没有匹配连接"} text={tab === "closed" ? "本页会保留由当前 Web 界面关闭的最近 100 条真实连接。" : "调整筛选条件，或等待新的网络连接。"} />}
  </main>;
}

function ConnectionRow({ item, closed, busy, onClose }: { item: ControllerConnection; closed: boolean; busy: boolean; onClose: () => void }) {
  const host = item.metadata.host || item.metadata.destinationIP;
  return <div className="connection-row" role="row"><div><b>{item.metadata.sourceIP}:{item.metadata.sourcePort}</b><span>{item.metadata.type} · {item.metadata.network}</span></div><div className="chain-cell">{item.chains.join(" → ") || "直连"}</div><div><b>{host}:{item.metadata.destinationPort}</b><span>{item.rule}{item.rulePayload ? ` · ${item.rulePayload}` : ""}</span></div><strong className="down">{formatRate(item.downloadSpeed)}</strong><strong className="up">{formatRate(item.uploadSpeed)}</strong><span>{formatBytes(item.download)}</span><span>{formatBytes(item.upload)}</span><time>{formatDateTime(item.start)}</time><IconButton label={closed ? "连接已关闭" : `关闭 ${host}`} disabled={closed || busy} onClick={onClose}><XMarkIcon /></IconButton></div>;
}
