import { useEffect, useRef, useState } from "react";
import appLogo from "../src-tauri/icons/128x128.png";
import { bootstrapStatus, type BootstrapStatus } from "./lib/bootstrap";
import { formatTraffic, trafficTrend } from "./lib/traffic-trend";
import { SubscriptionPage } from "./components/subscriptions/SubscriptionPage";
import { SidebarTraffic } from "./components/layout/SidebarTraffic";
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
  subscriptionSelectionRequired: "请先在订阅页选择要使用的订阅",
  busy: "操作进行中，请稍后重试",
  stopped: "停止操作完成",
  alreadyStopped: "无需重复停止",
  stopFailed: "停止失败，请查看运行状态",
};

const navigationItems = [
  { id: "home", label: "首页", icon: "M16.3334 23.8333V19.6666H19.6667V23.8333C19.6667 24.2916 20.0417 24.6666 20.5001 24.6666H23.0001C23.4584 24.6666 23.8334 24.2916 23.8334 23.8333V18H25.2501C25.6334 18 25.8167 17.525 25.5251 17.275L18.5584 11C18.2417 10.7166 17.7584 10.7166 17.4417 11L10.4751 17.275C10.1917 17.525 10.3667 18 10.7501 18H12.1667V23.8333C12.1667 24.2916 12.5417 24.6666 13.0001 24.6666H15.5001C15.9584 24.6666 16.3334 24.2916 16.3334 23.8333Z" },
  { id: "outbounds", label: "出口组", icon: "M9.7167 16.3834C10.1417 16.8084 10.8167 16.85 11.275 16.4667C15.1667 13.2667 20.8167 13.2667 24.7167 16.4583C25.1834 16.8417 25.8667 16.8084 26.2917 16.3834C26.7834 15.8917 26.75 15.075 26.2084 14.6333C21.45 10.7417 14.5667 10.7417 9.80003 14.6333C9.25836 15.0667 9.2167 15.8834 9.7167 16.3834ZM16.1834 22.85L17.4084 24.075C17.7334 24.4 18.2584 24.4 18.5834 24.075L19.8084 22.85C20.2 22.4583 20.1167 21.7833 19.6167 21.525C18.6 21 17.3834 21 16.3584 21.525C15.8834 21.7833 15.7917 22.4583 16.1834 22.85ZM13.075 19.7417C13.4834 20.15 14.125 20.1917 14.6 19.85C16.6334 18.4084 19.3667 18.4084 21.4 19.85C21.875 20.1834 22.5167 20.15 22.925 19.7417L22.9334 19.7333C23.4334 19.2333 23.4 18.3834 22.825 17.975C19.9584 15.9 16.05 15.9 13.175 17.975C12.6 18.3917 12.5667 19.2333 13.075 19.7417Z" },
  { id: "subscriptions", label: "订阅", icon: "M23.8333 18.8333H12.1667C11.25 18.8333 10.5 19.5833 10.5 20.5V23.8333C10.5 24.75 11.25 25.5 12.1667 25.5H23.8333C24.75 25.5 25.5 24.75 25.5 23.8333V20.5C25.5 19.5833 24.75 18.8333 23.8333 18.8333ZM13.8333 23.8333C12.9167 23.8333 12.1667 23.0833 12.1667 22.1667C12.1667 21.25 12.9167 20.5 13.8333 20.5C14.75 20.5 15.5 21.25 15.5 22.1667C15.5 23.0833 14.75 23.8333 13.8333 23.8333ZM23.8333 10.5H12.1667C11.25 10.5 10.5 11.25 10.5 12.1667V15.5C10.5 16.4167 11.25 17.1667 12.1667 17.1667H23.8333C24.75 17.1667 25.5 16.4167 25.5 15.5V12.1667C25.5 11.25 24.75 10.5 23.8333 10.5ZM13.8333 15.5C12.9167 15.5 12.1667 14.75 12.1667 13.8333C12.1667 12.9167 12.9167 12.1667 13.8333 12.1667C14.75 12.1667 15.5 12.9167 15.5 13.8333C15.5 14.75 14.75 15.5 13.8333 15.5Z" },
  { id: "connections", label: "连接", icon: "M17.9917 9.66675C13.3917 9.66675 9.66669 13.4001 9.66669 18.0001C9.66669 22.6001 13.3917 26.3334 17.9917 26.3334C22.6 26.3334 26.3334 22.6001 26.3334 18.0001C26.3334 13.4001 22.6 9.66675 17.9917 9.66675ZM23.7667 14.6667H21.3084C21.0417 13.6251 20.6584 12.6251 20.1584 11.7001C21.6917 12.2251 22.9667 13.2917 23.7667 14.6667ZM18 11.3667C18.6917 12.3667 19.2334 13.4751 19.5917 14.6667H16.4084C16.7667 13.4751 17.3084 12.3667 18 11.3667ZM11.55 19.6667C11.4167 19.1334 11.3334 18.5751 11.3334 18.0001C11.3334 17.4251 11.4167 16.8667 11.55 16.3334H14.3667C14.3 16.8834 14.25 17.4334 14.25 18.0001C14.25 18.5667 14.3 19.1167 14.3667 19.6667H11.55ZM12.2334 21.3334H14.6917C14.9584 22.3751 15.3417 23.3751 15.8417 24.3001C14.3084 23.7751 13.0334 22.7167 12.2334 21.3334ZM14.6917 14.6667H12.2334C13.0334 13.2834 14.3084 12.2251 15.8417 11.7001C15.3417 12.6251 14.9584 13.6251 14.6917 14.6667ZM18 24.6334C17.3084 23.6334 16.7667 22.5251 16.4084 21.3334H19.5917C19.2334 22.5251 18.6917 23.6334 18 24.6334ZM19.95 19.6667H16.05C15.975 19.1167 15.9167 18.5667 15.9167 18.0001C15.9167 17.4334 15.975 16.8751 16.05 16.3334H19.95C20.025 16.8751 20.0834 17.4334 20.0834 18.0001C20.0834 18.5667 20.025 19.1167 19.95 19.6667ZM20.1584 24.3001C20.6584 23.3751 21.0417 22.3751 21.3084 21.3334H23.7667C22.9667 22.7084 21.6917 23.7751 20.1584 24.3001ZM21.6334 19.6667C21.7 19.1167 21.75 18.5667 21.75 18.0001C21.75 17.4334 21.7 16.8834 21.6334 16.3334H24.45C24.5834 16.8667 24.6667 17.4251 24.6667 18.0001C24.6667 18.5751 24.5834 19.1334 24.45 19.6667H21.6334Z" },
  { id: "routing", label: "分流", icon: "M15.5 24.6666C15.5 25.125 15.875 25.5 16.3333 25.5C16.7917 25.5 17.1667 25.125 17.1667 24.6666V22.1666C17.775 20.0166 19.725 19.275 21.475 19.6666L20.7416 20.4C20.4166 20.725 20.4166 21.25 20.7416 21.575C21.0666 21.9 21.5917 21.9 21.9167 21.575L24.075 19.4166C24.4 19.0916 24.4 18.5666 24.075 18.2416L21.9167 16.0833C21.8396 16.006 21.748 15.9447 21.6472 15.9029C21.5464 15.8611 21.4383 15.8396 21.3291 15.8396C21.22 15.8396 21.1119 15.8611 21.0111 15.9029C20.9103 15.9447 20.8187 16.006 20.7416 16.0833C20.4166 16.4083 20.4166 16.9333 20.7416 17.2583L21.475 18C20.2166 17.725 18.3667 18.0666 17.1667 19.1333V13.6916L17.9 14.425C18.225 14.75 18.75 14.75 19.075 14.425C19.4 14.1 19.4 13.575 19.075 13.25L16.9167 11.0916C16.8396 11.0144 16.748 10.9531 16.6472 10.9113C16.5464 10.8694 16.4383 10.8479 16.3292 10.8479C16.22 10.8479 16.1119 10.8694 16.0111 10.9113C15.9103 10.9531 15.8187 11.0144 15.7417 11.0916L13.5917 13.2416C13.2667 13.5666 13.2667 14.0916 13.5917 14.4166C13.9167 14.7416 14.4417 14.7416 14.7667 14.4166L15.5 13.6916V24.6666Z" },
  { id: "logs", label: "日志", icon: "M18.8334 22.1667H12.1667C11.7084 22.1667 11.3334 22.5417 11.3334 23.0001C11.3334 23.4584 11.7084 23.8334 12.1667 23.8334H18.8334C19.2917 23.8334 19.6667 23.4584 19.6667 23.0001C19.6667 22.5417 19.2917 22.1667 18.8334 22.1667ZM23.8334 15.5001H12.1667C11.7084 15.5001 11.3334 15.8751 11.3334 16.3334C11.3334 16.7917 11.7084 17.1667 12.1667 17.1667H23.8334C24.2917 17.1667 24.6667 16.7917 24.6667 16.3334C24.6667 15.8751 24.2917 15.5001 23.8334 15.5001ZM12.1667 20.5001H23.8334C24.2917 20.5001 24.6667 20.1251 24.6667 19.6667C24.6667 19.2084 24.2917 18.8334 23.8334 18.8334H12.1667C11.7084 18.8334 11.3334 19.2084 11.3334 19.6667C11.3334 20.1251 11.7084 20.5001 12.1667 20.5001ZM11.3334 13.0001C11.3334 13.4584 11.7084 13.8334 12.1667 13.8334H23.8334C24.2917 13.8334 24.6667 13.4584 24.6667 13.0001C24.6667 12.5417 24.2917 12.1667 23.8334 12.1667H12.1667C11.7084 12.1667 11.3334 12.5417 11.3334 13.0001Z" },
  { id: "settings", label: "设置", icon: "M24.25 18.0001C24.25 17.8084 24.2416 17.6251 24.225 17.4334L25.775 16.2584C26.1083 16.0084 26.2 15.5417 25.9916 15.1751L24.4333 12.4834C24.225 12.1167 23.775 11.9667 23.3916 12.1334L21.6 12.8917C21.2916 12.6751 20.9666 12.4834 20.625 12.3251L20.3833 10.4001C20.3333 9.98341 19.975 9.66675 19.5583 9.66675H16.45C16.025 9.66675 15.6666 9.98341 15.6166 10.4001L15.375 12.3251C15.0333 12.4834 14.7083 12.6751 14.4 12.8917L12.6083 12.1334C12.225 11.9667 11.775 12.1167 11.5666 12.4834L10.0083 15.1834C9.79997 15.5501 9.89163 16.0084 10.225 16.2667L11.775 17.4417C11.7583 17.6251 11.75 17.8084 11.75 18.0001C11.75 18.1917 11.7583 18.3751 11.775 18.5667L10.225 19.7417C9.89163 19.9917 9.79997 20.4584 10.0083 20.8251L11.5666 23.5167C11.775 23.8834 12.225 24.0334 12.6083 23.8667L14.4 23.1084C14.7083 23.3251 15.0333 23.5167 15.375 23.6751L15.6166 25.6001C15.6666 26.0167 16.025 26.3334 16.4416 26.3334H19.55C19.9666 26.3334 20.325 26.0167 20.375 25.6001L20.6166 23.6751C20.9583 23.5167 21.2833 23.3251 21.5916 23.1084L23.3833 23.8667C23.7666 24.0334 24.2166 23.8834 24.425 23.5167L25.9833 20.8251C26.1916 20.4584 26.1 20.0001 25.7666 19.7417L24.2166 18.5667C24.2416 18.3751 24.25 18.1917 24.25 18.0001ZM18.0333 20.9167C16.425 20.9167 15.1166 19.6084 15.1166 18.0001C15.1166 16.3917 16.425 15.0834 18.0333 15.0834C19.6416 15.0834 20.95 16.3917 20.95 18.0001C20.95 19.6084 19.6416 20.9167 18.0333 20.9167Z" },
] as const;

type NavigationPage = (typeof navigationItems)[number]["id"];

export default function App() {
  const [activePage, setActivePage] = useState<NavigationPage>("home");
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
    document.getElementById(`page-${activePage}`)?.querySelector<HTMLElement>(".page-scroll")?.scrollTo(0, 0);
  }, [activePage]);

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
        <div className="brand"><img className="brand-mark" src={appLogo} width={32} height={32} alt="" /><span>{status.application}</span></div>
        <nav aria-label="主导航">
          {navigationItems.map((item) => <button
            key={item.id}
            type="button"
            aria-current={activePage === item.id ? "page" : undefined}
            aria-controls={`page-${item.id}`}
            onClick={() => setActivePage(item.id)}
          ><svg className="nav-icon" viewBox={item.id === "routing" || item.id === "logs" ? "10 10 16 16" : "8 8 20 20"} aria-hidden="true"><path d={item.icon} fill="currentColor" stroke="none" /></svg><span className="nav-label">{item.label}</span></button>)}
        </nav>
        <section className="sidebar-traffic" aria-label="左下角即时网速">
          <SidebarTraffic observation={observation} unavailable={observationError} />
        </section>
      </aside>
      <main className="home-content" id="page-home" hidden={activePage !== "home"} aria-labelledby="home-title">
        <header className="home-header"><h1 id="home-title">首页</h1><span className="home-status">{summary}</span></header>
        <div className="page-scroll">
          <section className="traffic-panel" aria-label="sing-box 聚合流量">
            <div className="traffic-heading"><h2>流量统计</h2><span className="eyebrow">sing-box 聚合 · 含直连</span></div>
            <TrafficChart observation={observation} unavailable={observationError} nowMs={nowMs} />
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
              <button type="button" disabled={runtimeActionPending} onClick={() => void runManagedRuntimeAction("start")}>启动内核</button>
              <button type="button" disabled={runtimeActionPending} onClick={() => void runManagedRuntimeAction("stop")}>停止内核</button>
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
        </div>
      </main>
      <SubscriptionPage active={activePage === "subscriptions"} observation={observation} />
      {navigationItems.filter((item) => item.id !== "home" && item.id !== "subscriptions").map((item) => <main
        key={item.id}
        className="home-content page-placeholder"
        id={`page-${item.id}`}
        hidden={activePage !== item.id}
        aria-labelledby={`page-${item.id}-title`}
      >
        <header className="home-header"><h1 id={`page-${item.id}-title`}>{item.label}</h1></header>
        <div className="page-scroll"><section className="placeholder-card" aria-label={`${item.label}页面状态`}>
          <p>{item.id === "settings" ? "真实设置能力尚未定义" : "此功能暂未开放"}</p>
        </section></div>
      </main>)}
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

function TrafficRates({ observation, unavailable }: { observation: RuntimeObservation | null; unavailable: boolean }) {
  return <dl className="traffic-rates" aria-label="实时网速">
    <div><dt className="upload-key">↑ 上传</dt><dd>{unavailable || observation === null ? "—" : formatTraffic(observation.uploadRateBps, true)}</dd></div>
    <div><dt className="download-key">↓ 下载</dt><dd>{unavailable || observation === null ? "—" : formatTraffic(observation.downloadRateBps, true)}</dd></div>
  </dl>;
}

function TrafficChart({ observation, unavailable, nowMs }: {
  observation: RuntimeObservation | null; unavailable: boolean; nowMs: number;
}) {
  const trend = trafficTrend(!unavailable && observation?.sidecarLifecycle === "ready" ? observation.trafficHistory : [], nowMs, 600000);
  const emptyMessage = unavailable ? "网速趋势不可用" : observation?.sidecarLifecycle === "stopped" ? "已停止" :
    observation?.sidecarLifecycle === "recoveryRequired" ? "网速趋势不可用，停止未完成" : "等待网速采样";
  return <figure className="traffic-chart" aria-label="最近 10 分钟上下行网速趋势">
    <figcaption><span>最近 10 分钟</span>
      <span className="traffic-legend"><span className="upload-key">上传 · 实线</span><span className="download-key">下载 · 虚线</span></span>
    </figcaption>
    <div className="traffic-axis-top">{formatTraffic(trend.maximum, true)}</div>
    <div className="traffic-plot">
      <svg viewBox="0 0 600 160" preserveAspectRatio="none" role="img" aria-label="上传实线与下载虚线，采样缺口保持断开">
        <path className="traffic-grid" d="M0,4 H600 M0,80 H600 M0,156 H600" />
        <path className="traffic-upload" d={trend.uploadPath} />
        <path className="traffic-download" d={trend.downloadPath} />
        {trend.isolated.map((point) => <g key={point.x}>
          <circle className="traffic-upload" cx={point.x} cy={point.uploadY} r={2} />
          <circle className="traffic-download" cx={point.x} cy={point.downloadY} r={2} />
        </g>)}
      </svg>
      {trend.points.length === 0 ? <p className="traffic-empty" role="status">{emptyMessage}</p> : null}
    </div>
    <div className="traffic-axis-zero">0 B/s</div>
    <div className="traffic-axis-time"><span>10 分钟前</span><span>5 分钟前</span><span>现在</span></div>
  </figure>;
}
