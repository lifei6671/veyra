import { useEffect, useRef, useState } from "react";
import { bootstrapStatus, type BootstrapStatus } from "./lib/bootstrap";
import { formatTraffic, trafficTrend } from "./lib/traffic-trend";
import {
  acceptNewerObservation,
  runtimeObservationSnapshot,
  startManagedObservationRuntime,
  stopManagedObservationRuntime,
  subscribeRuntimeObservationDelta,
  type RuntimeObservation,
  type ManagedRuntimeStartResult,
  type ManagedRuntimeStopResult,
} from "./lib/observability";

const initialStatus: BootstrapStatus = {
  application: "Veyra",
  status: "loading",
};

const actionMessages: Record<ManagedRuntimeStartResult | ManagedRuntimeStopResult, string> = {
  started: "内核已启动",
  alreadyRunning: "内核已在运行",
  stateUnavailable: "配置状态不可用",
  configurationFailed: "配置生成失败，未应用新配置",
  startFailed: "内核启动失败，请查看运行状态",
  busy: "操作进行中，请稍后重试",
  stopped: "停止操作完成",
  alreadyStopped: "无需重复停止",
  stopFailed: "停止失败，请查看运行状态",
};

export default function App() {
  const [status, setStatus] = useState(initialStatus);
  const [hasError, setHasError] = useState(false);
  const [{ observation, observationError }, setObservationState] = useState<{
    observation: RuntimeObservation | null;
    observationError: boolean;
  }>({ observation: null, observationError: false });
  const [runtimeAction, setRuntimeAction] = useState("未操作");
  const [runtimeActionPending, setRuntimeActionPending] = useState(false);
  const actionGeneration = useRef(0);
  const actionInFlight = useRef(false);
  const [failureToast, setFailureToast] = useState<{ message: string } | null>(null);

  useEffect(() => {
    if (failureToast === null) return;
    const timer = window.setTimeout(() => setFailureToast(null), 6000);
    return () => window.clearTimeout(timer);
  }, [failureToast]);

  useEffect(() => {
    void bootstrapStatus()
      .then((nextStatus) => {
        document.title = `${nextStatus.application} · ${nextStatus.status}`;
        setStatus(nextStatus);
      })
      .catch(() => {
        setHasError(true);
      });
  }, []);

  const runManagedRuntimeAction = async (action: "start" | "stop") => {
    if (actionInFlight.current) return;
    actionInFlight.current = true;
    actionGeneration.current += 1;
    const actionStartRevision = observation?.revision ?? -1;
    setRuntimeActionPending(true);
    setFailureToast(null);
    setObservationState((current) => ({ ...current, observationError: true }));
    try {
      const result = await (action === "start" ? startManagedObservationRuntime() : stopManagedObservationRuntime());
      const message = actionMessages[result];
      setRuntimeAction(message);
      if (result !== "started" && result !== "alreadyRunning" && result !== "stopped" && result !== "alreadyStopped") {
        setFailureToast({ message });
      }
    } catch {
      const message = "操作结果不可用，请查看运行状态";
      setRuntimeAction(message);
      setFailureToast({ message });
    }

    // 观测失败不能覆盖动作结果，也不能把动作前的快照当作当前停止证明。
    try {
      const next = await runtimeObservationSnapshot();
      setObservationState((current) => ({
        observation: acceptNewerObservation(current.observation, next),
        observationError: current.observation !== null &&
          next.revision < current.observation.revision && current.observation.revision <= actionStartRevision,
      }));
    } catch {
      setObservationState((current) => ({ ...current, observationError: true }));
    } finally {
      actionInFlight.current = false;
      setRuntimeActionPending(false);
    }
  };

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    const initialGeneration = actionGeneration.current;
    const applyObservation = (next: RuntimeObservation) => {
      if (!active) {
        return;
      }
      const canConfirmState = !actionInFlight.current;
      setObservationState((current) => {
        const accepted = acceptNewerObservation(current.observation, next);
        return accepted === current.observation ? current : {
          observation: accepted,
          observationError: !canConfirmState,
        };
      });
    };

    void subscribeRuntimeObservationDelta(applyObservation)
      .then((stop) => {
        unlisten = stop;
        return runtimeObservationSnapshot().then((next) => {
          if (initialGeneration === actionGeneration.current) applyObservation(next);
        });
      })
      .catch(() => {
        if (active && initialGeneration === actionGeneration.current) {
          setObservationState((current) => ({ ...current, observationError: true }));
        }
      });

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  const nowMs = useObservationClock(observation);
  const summary = observationError ? (runtimeActionPending ? "运行状态待确认" : "运行观测不可用，当前状态待确认") :
    observation === null ? "正在读取运行观测" :
      observation.sidecarLifecycle === "recoveryRequired" ? "停止未完成" :
        observation.source === "runtime" && observation.sidecarLifecycle === "stopped" ? "服务已停止" :
          observation.sidecarLifecycle === "ready" ? "服务运行中" : "尚未连接运行内核";

  return (
    <div className="app-shell">
      <aside className="sidebar" aria-label="主导航与即时网速">
        <div className="brand"><span className="brand-mark" aria-hidden="true">V</span><span>{status.application}</span></div>
        <nav aria-label="主导航"><a href="#home" aria-current="page"><span aria-hidden="true">⌂</span> 首页</a></nav>
        <section className="sidebar-traffic" aria-label="左下角即时网速">
          <TrafficChart observation={observation} unavailable={observationError} nowMs={nowMs} windowMs={60000} />
          <TrafficRates observation={observation} unavailable={observationError} compact />
        </section>
      </aside>
      <main className="home-content" id="home">
        <header className="home-header"><div><p className="eyebrow">网络概览</p><h1>首页</h1></div><span className="home-status">{summary}</span></header>
        <section className="traffic-panel" aria-label="sing-box 聚合流量">
          <div className="traffic-heading"><h2>流量统计</h2><span className="eyebrow">sing-box 聚合 · 含直连</span></div>
          <TrafficChart observation={observation} unavailable={observationError} nowMs={nowMs} windowMs={600000} />
          <TrafficRates observation={observation} unavailable={observationError} />
          <div className="traffic-totals"><span className="eyebrow">本次内核累计</span>
            <span>上传 {observationError || observation === null ? "—" : formatTraffic(observation.uploadTotalBytes)}</span>
            <span>下载 {observationError || observation === null ? "—" : formatTraffic(observation.downloadTotalBytes)}</span>
          </div>
        </section>
        <section className="observation" aria-label="运行观测">
          <div className="traffic-heading"><h2>运行状态</h2><span className="eyebrow">{observationError || observation === null ? "连接数待确认" : `当前连接 ${observation.connectionCount}`}</span></div>
          <p>{summary}</p>
          <div className="runtime-controls">
            <button type="button" disabled={runtimeActionPending} onClick={() => void runManagedRuntimeAction("start")}>启动受管观测运行时</button>
            <button type="button" disabled={runtimeActionPending} onClick={() => void runManagedRuntimeAction("stop")}>停止受管观测运行时</button>
            <span>操作：{runtimeAction}</span>
          </div>
          <details className="runtime-diagnostics"><summary>诊断信息</summary>
            <p>{hasError ? "启动信息不可用" : `状态：${status.status}`}</p>
            {observation !== null ? <>
              <p>{observation.source === "runtime" ? "受管运行时观测" : "内存 Mock 观测"}</p>
              <p>捕获：{observation.captureMode}；运行时：{observation.sidecarLifecycle}；修订：{observation.revision}</p>
              <ul>{observation.logSummary.map((entry) => <li key={`${entry.category}-${entry.level}`}>{entry.category}/{entry.level}: {entry.occurrences}</li>)}</ul>
            </> : null}
          </details>
        </section>
      </main>
      {failureToast !== null ? <div className="failure-toast" role="alert"><span>{failureToast.message}</span><button type="button" aria-label="关闭失败提示" onClick={() => setFailureToast(null)}>关闭</button></div> : null}
    </div>
  );
}

function useObservationClock(observation: RuntimeObservation | null) {
  const [clock, setClock] = useState({ observation, nowMs: observation?.observedAtMs ?? 0 });
  useEffect(() => {
    if (observation === null) return;
    const receivedAt = performance.now();
    setClock({ observation, nowMs: observation.observedAtMs });
    // 两幅图共用显示时钟；不追加采样点，不增加 IPC 或网络请求。
    const timer = window.setInterval(() => {
      setClock({ observation, nowMs: observation.observedAtMs + performance.now() - receivedAt });
    }, 1000);
    return () => window.clearInterval(timer);
  }, [observation]);
  return clock.observation === observation ? clock.nowMs : observation?.observedAtMs ?? 0;
}

function TrafficRates({ observation, unavailable, compact = false }: { observation: RuntimeObservation | null; unavailable: boolean; compact?: boolean }) {
  return <dl className={`traffic-rates${compact ? " traffic-rates-compact" : ""}`} aria-label="实时网速">
    <div><dt className="upload-key">↑ 上传</dt><dd>{unavailable || observation === null ? "—" : formatTraffic(observation.uploadRateBps, true)}</dd></div>
    <div><dt className="download-key">↓ 下载</dt><dd>{unavailable || observation === null ? "—" : formatTraffic(observation.downloadRateBps, true)}</dd></div>
  </dl>;
}

function TrafficChart({ observation, unavailable, nowMs, windowMs }: {
  observation: RuntimeObservation | null; unavailable: boolean; nowMs: number; windowMs: 60000 | 600000;
}) {
  const compact = windowMs === 60000;
  const trend = trafficTrend(!unavailable && observation?.sidecarLifecycle === "ready" ? observation.trafficHistory : [], nowMs, windowMs);
  const emptyMessage = unavailable ? "网速趋势不可用" : observation?.sidecarLifecycle === "stopped" ? "已停止" :
    observation?.sidecarLifecycle === "recoveryRequired" ? "网速趋势不可用，停止未完成" : "等待网速采样";
  return <figure className={`traffic-chart${compact ? " traffic-chart-compact" : ""}`} aria-label={compact ? "最近 60 秒上下行网速趋势" : "最近 10 分钟上下行网速趋势"}>
    <figcaption><span>{compact ? "最近 60 秒" : "最近 10 分钟"}</span>
      {!compact ? <span className="traffic-legend"><span className="upload-key">上传 · 实线</span><span className="download-key">下载 · 虚线</span></span> : null}
    </figcaption>
    <div className="traffic-axis-top">{formatTraffic(trend.maximum, true)}</div>
    <div className="traffic-plot">
      <svg viewBox="0 0 600 160" preserveAspectRatio="none" role="img" aria-label="上传实线与下载虚线，采样缺口保持断开">
        <path className="traffic-grid" d="M0,4 H600 M0,80 H600 M0,156 H600" />
        <path className="traffic-upload" d={trend.uploadPath} />
        <path className="traffic-download" d={trend.downloadPath} />
        {trend.isolated.map((point) => <g key={point.x}>
          <circle className="traffic-upload" cx={point.x} cy={point.uploadY} r={compact ? 5 : 2} />
          <circle className="traffic-download" cx={point.x} cy={point.downloadY} r={compact ? 5 : 2} />
        </g>)}
      </svg>
      {trend.points.length === 0 ? <p className="traffic-empty" role="status">{emptyMessage}</p> : null}
    </div>
    {!compact ? <div className="traffic-axis-zero">0 B/s</div> : null}
    <div className="traffic-axis-time"><span>{compact ? "60 秒前" : "10 分钟前"}</span>{!compact ? <span>5 分钟前</span> : null}<span>现在</span></div>
  </figure>;
}
