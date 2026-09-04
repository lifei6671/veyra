import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const SNAPSHOT_COMMAND = "runtime_observation_snapshot";
const START_MANAGED_RUNTIME_COMMAND = "start_managed_observation_runtime";
const STOP_MANAGED_RUNTIME_COMMAND = "stop_managed_observation_runtime";
const DELTA_EVENT = "runtime_observation_delta";

export type RuntimeObservation = {
  source: "mock" | "runtime";
  revision: number;
  observedAtMs: number;
  trafficHistory: ReadonlyArray<TrafficSample>;
  captureMode: "off" | "systemProxy" | "recoveryRequired";
  sidecarLifecycle: "notAttached" | "stopped" | "ready" | "recoveryRequired";
  uploadRateBps: number;
  downloadRateBps: number;
  uploadTotalBytes: number;
  downloadTotalBytes: number;
  connectionCount: number;
  logSummary: ReadonlyArray<LogSummary>;
};

export type LogSummary = {
  category: "runtime" | "proxy" | "subscription";
  level: "info" | "warn" | "error";
  occurrences: number;
};

export type TrafficSample = {
  sampledAtMs: number;
  uploadRateBps: number;
  downloadRateBps: number;
};

export type ManagedRuntimeStartResult =
  | "started"
  | "alreadyRunning"
  | "stateUnavailable"
  | "configurationFailed"
  | "startFailed"
  | "busy";

export type ManagedRuntimeStopResult = "stopped" | "alreadyStopped" | "stopFailed" | "busy";

export async function runtimeObservationSnapshot(): Promise<RuntimeObservation> {
  return parseRuntimeObservation(await invoke<unknown>(SNAPSHOT_COMMAND));
}

export async function startManagedObservationRuntime(): Promise<ManagedRuntimeStartResult> {
  return parseManagedRuntimeStartResult(await invoke<unknown>(START_MANAGED_RUNTIME_COMMAND));
}

export async function stopManagedObservationRuntime(): Promise<ManagedRuntimeStopResult> {
  return parseManagedRuntimeStopResult(await invoke<unknown>(STOP_MANAGED_RUNTIME_COMMAND));
}

export async function subscribeRuntimeObservationDelta(
  onDelta: (observation: RuntimeObservation) => void,
): Promise<UnlistenFn> {
  return listen<unknown>(DELTA_EVENT, ({ payload }) => {
    onDelta(parseRuntimeObservation(payload));
  });
}

export function acceptNewerObservation(
  current: RuntimeObservation | null,
  next: RuntimeObservation,
): RuntimeObservation | null {
  return current === null || next.revision > current.revision ||
    (next.revision === current.revision && next.observedAtMs > current.observedAtMs) ? next : current;
}

function parseRuntimeObservation(value: unknown): RuntimeObservation {
  if (!isRecord(value) || !hasExactKeys(value, observationKeys)) {
    throw new Error("invalid runtime observation payload");
  }

  if (
    !isObservationSource(value.source) ||
    !isNonNegativeInteger(value.revision) ||
    !isNonNegativeInteger(value.observedAtMs) ||
    !Array.isArray(value.trafficHistory) ||
    value.trafficHistory.length > 600 ||
    !isCaptureMode(value.captureMode) ||
    !isSidecarLifecycle(value.sidecarLifecycle) ||
    !isNonNegativeNumber(value.uploadRateBps) ||
    !isNonNegativeNumber(value.downloadRateBps) ||
    !isNonNegativeInteger(value.uploadTotalBytes) ||
    !isNonNegativeInteger(value.downloadTotalBytes) ||
    !isNonNegativeInteger(value.connectionCount) ||
    !Array.isArray(value.logSummary)
  ) {
    throw new Error("invalid runtime observation payload");
  }

  const trafficHistory = value.trafficHistory.map((point: unknown, index: number, points: unknown[]) => {
    if (!isRecord(point) || !hasExactKeys(point, trafficSampleKeys) ||
      !isNonNegativeInteger(point.sampledAtMs) ||
      point.sampledAtMs > (value.observedAtMs as number) ||
      (value.observedAtMs as number) - point.sampledAtMs > 600000 ||
      !isNonNegativeNumber(point.uploadRateBps) || !isNonNegativeNumber(point.downloadRateBps) ||
      (index > 0 && point.sampledAtMs <= (points[index - 1] as TrafficSample).sampledAtMs)) {
      throw new Error("invalid runtime observation payload");
    }
    return point as TrafficSample;
  });

  return {
    source: value.source,
    revision: value.revision,
    observedAtMs: value.observedAtMs,
    trafficHistory,
    captureMode: value.captureMode,
    sidecarLifecycle: value.sidecarLifecycle,
    uploadRateBps: value.uploadRateBps,
    downloadRateBps: value.downloadRateBps,
    uploadTotalBytes: value.uploadTotalBytes,
    downloadTotalBytes: value.downloadTotalBytes,
    connectionCount: value.connectionCount,
    logSummary: value.logSummary.map(parseLogSummary),
  };
}

function parseManagedRuntimeStartResult(value: unknown): ManagedRuntimeStartResult {
  if (
    value === "started" ||
    value === "alreadyRunning" ||
    value === "stateUnavailable" ||
    value === "configurationFailed" ||
    value === "startFailed" ||
    value === "busy"
  ) {
    return value;
  }
  throw new Error("invalid managed runtime start result");
}

function parseManagedRuntimeStopResult(value: unknown): ManagedRuntimeStopResult {
  if (value === "stopped" || value === "alreadyStopped" || value === "stopFailed" || value === "busy") {
    return value;
  }
  throw new Error("invalid managed runtime stop result");
}

function isObservationSource(value: unknown): value is RuntimeObservation["source"] {
  return value === "mock" || value === "runtime";
}

function parseLogSummary(value: unknown): LogSummary {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, logSummaryKeys) ||
    !isLogCategory(value.category) ||
    !isLogLevel(value.level) ||
    !isNonNegativeInteger(value.occurrences)
  ) {
    throw new Error("invalid runtime observation payload");
  }

  return value as LogSummary;
}

const observationKeys = [
  "source",
  "revision",
  "observedAtMs",
  "trafficHistory",
  "captureMode",
  "sidecarLifecycle",
  "uploadRateBps",
  "downloadRateBps",
  "uploadTotalBytes",
  "downloadTotalBytes",
  "connectionCount",
  "logSummary",
] as const;
const logSummaryKeys = ["category", "level", "occurrences"] as const;
const trafficSampleKeys = ["sampledAtMs", "uploadRateBps", "downloadRateBps"] as const;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(
  value: Record<string, unknown>,
  expected: ReadonlyArray<string>,
): boolean {
  const keys = Object.keys(value);
  return keys.length === expected.length && expected.every((key) => key in value);
}

function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isNonNegativeNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

function isCaptureMode(value: unknown): value is RuntimeObservation["captureMode"] {
  return value === "off" || value === "systemProxy" || value === "recoveryRequired";
}

function isSidecarLifecycle(
  value: unknown,
): value is RuntimeObservation["sidecarLifecycle"] {
  return (
    value === "notAttached" ||
    value === "stopped" ||
    value === "ready" ||
    value === "recoveryRequired"
  );
}

function isLogCategory(value: unknown): value is LogSummary["category"] {
  return value === "runtime" || value === "proxy" || value === "subscription";
}

function isLogLevel(value: unknown): value is LogSummary["level"] {
  return value === "info" || value === "warn" || value === "error";
}
