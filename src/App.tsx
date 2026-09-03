import { useEffect, useState } from "react";
import { bootstrapStatus, type BootstrapStatus } from "./lib/bootstrap";
import {
  acceptNewerObservation,
  runtimeObservationSnapshot,
  subscribeRuntimeObservationDelta,
  type RuntimeObservation,
} from "./lib/observability";

const initialStatus: BootstrapStatus = {
  application: "Veyra",
  status: "loading",
};

export default function App() {
  const [status, setStatus] = useState(initialStatus);
  const [hasError, setHasError] = useState(false);
  const [observation, setObservation] = useState<RuntimeObservation | null>(null);
  const [observationError, setObservationError] = useState(false);

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

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    const applyObservation = (next: RuntimeObservation) => {
      if (!active) {
        return;
      }
      setObservation((current) => acceptNewerObservation(current, next));
    };

    void subscribeRuntimeObservationDelta(applyObservation)
      .then((stop) => {
        unlisten = stop;
        return runtimeObservationSnapshot();
      })
      .then(applyObservation)
      .catch(() => {
        if (active) {
          setObservationError(true);
        }
      });

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  return (
    <main className="app-shell">
      <p className="eyebrow">桌面工程基线</p>
      <h1>{status.application}</h1>
      <p>{hasError ? "启动信息不可用" : `状态：${status.status}`}</p>
      <section className="observation" aria-label="运行观测">
        <p className="eyebrow">内存 Mock 观测</p>
        {observationError ? (
          <p>运行观测不可用</p>
        ) : observation === null ? (
          <p>正在读取运行观测</p>
        ) : (
          <>
            <p>
              捕获：{observation.captureMode}；运行时：{observation.sidecarLifecycle}；修订：
              {observation.revision}
            </p>
            <p>
              上行 {observation.uploadRateBps} B/s / 下行 {observation.downloadRateBps} B/s；连接
              {observation.connectionCount}
            </p>
            <p>
              累计上行 {observation.uploadTotalBytes} B / 下行 {observation.downloadTotalBytes} B
            </p>
            <ul>
              {observation.logSummary.map((entry) => (
                <li key={`${entry.category}-${entry.level}`}>
                  {entry.category}/{entry.level}: {entry.occurrences}
                </li>
              ))}
            </ul>
          </>
        )}
      </section>
    </main>
  );
}
