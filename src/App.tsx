import { useEffect, useState } from "react";
import { bootstrapStatus, type BootstrapStatus } from "./lib/bootstrap";

const initialStatus: BootstrapStatus = {
  application: "Veyra",
  status: "loading",
};

export default function App() {
  const [status, setStatus] = useState(initialStatus);
  const [hasError, setHasError] = useState(false);

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

  return (
    <main className="app-shell">
      <p className="eyebrow">桌面工程基线</p>
      <h1>{status.application}</h1>
      <p>{hasError ? "启动信息不可用" : `状态：${status.status}`}</p>
    </main>
  );
}
