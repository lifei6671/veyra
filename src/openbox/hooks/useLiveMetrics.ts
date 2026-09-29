import { useMemo, useRef, useState } from "react";
import type { ConnectionsFrame, ControllerConnection, MemoryFrame, TrafficFrame } from "../api/types";
import { useControllerSocket } from "./useControllerSocket";

export function useLiveMetrics(enabled: boolean) {
  const [connections, setConnections] = useState<ConnectionsFrame>({ connections: [], closedConnections: [] });
  const [traffic, setTraffic] = useState({ up: 0, down: 0, upRate: 0, downRate: 0 });
  const [memory, setMemory] = useState(0);
  const previousConnections = useRef<{ time: number; values: Map<string, ControllerConnection> } | null>(null);
  const previousTraffic = useRef<{ time: number; value: TrafficFrame } | null>(null);

  const connectionSocket = useControllerSocket<ConnectionsFrame>("connections", {
    enabled,
    onMessage: frame => {
      const now = Date.now();
      const previous = previousConnections.current;
      const elapsed = previous ? Math.max(0.1, (now - previous.time) / 1000) : 1;
      const values = frame.connections.map(connection => {
        const before = previous?.values.get(connection.id);
        return {
          ...connection,
          downloadSpeed: before ? Math.max(0, (connection.download - before.download) / elapsed) : 0,
          uploadSpeed: before ? Math.max(0, (connection.upload - before.upload) / elapsed) : 0,
        };
      });
      const currentIds = new Set(values.map(connection => connection.id));
      const disappeared = previous ? [...previous.values.values()].filter(connection => !currentIds.has(connection.id)) : [];
      setConnections(current => {
        const retained = (current.closedConnections ?? []).filter(connection => !currentIds.has(connection.id) && !disappeared.some(item => item.id === connection.id));
        return { ...frame, connections: values, closedConnections: [...disappeared, ...retained].slice(0, 100) };
      });
      previousConnections.current = { time: now, values: new Map(values.map(connection => [connection.id, connection])) };
    },
  });

  const trafficSocket = useControllerSocket<TrafficFrame>("traffic", {
    enabled,
    onMessage: value => {
      const now = Date.now();
      const previous = previousTraffic.current;
      const elapsed = previous ? Math.max(0.1, (now - previous.time) / 1000) : 1;
      setTraffic({
        up: value.up,
        down: value.down,
        upRate: previous ? Math.max(0, (value.up - previous.value.up) / elapsed) : 0,
        downRate: previous ? Math.max(0, (value.down - previous.value.down) / elapsed) : 0,
      });
      previousTraffic.current = { time: now, value };
    },
  });

  const memorySocket = useControllerSocket<MemoryFrame>("memory", { enabled, onMessage: value => setMemory(value.inuse) });

  return useMemo(() => ({
    connections,
    traffic,
    memory,
    connected: [connectionSocket.state, trafficSocket.state, memorySocket.state].some(state => state === "open"),
  }), [connections, traffic, memory, connectionSocket.state, trafficSocket.state, memorySocket.state]);
}
