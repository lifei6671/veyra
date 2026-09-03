import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const SNAPSHOT_COMMAND = "runtime_observation_snapshot";
const DELTA_EVENT = "runtime_observation_delta";

export type RuntimeObservation = {
  source: "mock";
  revision: number;
  captureMode: "off" | "systemProxy" | "recoveryRequired";
  sidecarLifecycle: "notAttached" | "ready" | "recoveryRequired";
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

export async function runtimeObservationSnapshot(): Promise<RuntimeObservation> {
  return parseRuntimeObservation(await invoke<unknown>(SNAPSHOT_COMMAND));
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
  return current === null || next.revision > current.revision ? next : current;
}

function parseRuntimeObservation(value: unknown): RuntimeObservation {
  if (!isRecord(value) || !hasExactKeys(value, observationKeys)) {
    throw new Error("invalid runtime observation payload");
  }

  if (
    value.source !== "mock" ||
    !isNonNegativeInteger(value.revision) ||
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

  return {
    source: value.source,
    revision: value.revision,
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
  return value === "notAttached" || value === "ready" || value === "recoveryRequired";
}

function isLogCategory(value: unknown): value is LogSummary["category"] {
  return value === "runtime" || value === "proxy" || value === "subscription";
}

function isLogLevel(value: unknown): value is LogSummary["level"] {
  return value === "info" || value === "warn" || value === "error";
}
