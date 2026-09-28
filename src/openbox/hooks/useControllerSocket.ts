import { useEffect, useRef, useState } from "react";
import { controllerSocketUrl } from "../api/client";

export type SocketState = "connecting" | "open" | "closed" | "error";

export function useControllerSocket<T>(
  channel: "connections" | "logs" | "memory" | "traffic",
  options: { enabled?: boolean; query?: Record<string, string>; onMessage?: (value: T) => void } = {},
) {
  const { enabled = true, query, onMessage } = options;
  const [data, setData] = useState<T | null>(null);
  const [state, setState] = useState<SocketState>(enabled ? "connecting" : "closed");
  const messageHandler = useRef(onMessage);
  messageHandler.current = onMessage;
  const queryKey = JSON.stringify(query ?? {});

  useEffect(() => {
    if (!enabled) {
      setState("closed");
      return;
    }

    let socket: WebSocket | null = null;
    let reconnectTimer: number | undefined;
    let disposed = false;

    const connect = () => {
      if (disposed) return;
      setState("connecting");
      socket = new WebSocket(controllerSocketUrl(channel, JSON.parse(queryKey) as Record<string, string>));
      socket.addEventListener("open", () => setState("open"));
      socket.addEventListener("message", event => {
        try {
          const value = JSON.parse(String(event.data).trim()) as T;
          setData(value);
          messageHandler.current?.(value);
        } catch {
          setState("error");
        }
      });
      socket.addEventListener("error", () => setState("error"));
      socket.addEventListener("close", () => {
        if (disposed) return;
        setState("closed");
        reconnectTimer = window.setTimeout(connect, 2_000);
      });
    };

    connect();
    return () => {
      disposed = true;
      if (reconnectTimer) window.clearTimeout(reconnectTimer);
      socket?.close();
    };
  }, [channel, enabled, queryKey]);

  return { data, state };
}
