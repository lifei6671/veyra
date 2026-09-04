import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  acceptNewerObservation,
  runtimeObservationSnapshot,
  startManagedObservationRuntime,
  stopManagedObservationRuntime,
  type RuntimeObservation,
} from "./observability";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

const observation: RuntimeObservation = {
  source: "mock",
  revision: 3,
  observedAtMs: 600000,
  trafficHistory: [],
  captureMode: "off",
  sidecarLifecycle: "notAttached",
  uploadRateBps: 0,
  downloadRateBps: 0,
  uploadTotalBytes: 0,
  downloadTotalBytes: 0,
  connectionCount: 0,
  logSummary: [{ category: "runtime", level: "info", occurrences: 1 }],
};

describe("runtimeObservationSnapshot", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("仅调用固定命令且不传参数", async () => {
    vi.mocked(invoke).mockResolvedValue(observation);

    await expect(runtimeObservationSnapshot()).resolves.toEqual(observation);
    expect(invoke).toHaveBeenCalledWith("runtime_observation_snapshot");
  });

  it("拒绝含有敏感或未知字段的载荷", async () => {
    vi.mocked(invoke).mockResolvedValue({ ...observation, secret: "fixture-secret" });

    await expect(runtimeObservationSnapshot()).rejects.toThrow(
      "invalid runtime observation payload",
    );
  });

  it("接受受管运行时的既有安全 DTO", async () => {
    const runtimeObservation: RuntimeObservation = {
      ...observation,
      source: "runtime",
      sidecarLifecycle: "ready",
    };
    vi.mocked(invoke).mockResolvedValue(runtimeObservation);

    await expect(runtimeObservationSnapshot()).resolves.toEqual(runtimeObservation);
  });

  it("接受停止后的受管运行时安全生命周期", async () => {
    const stoppedObservation: RuntimeObservation = {
      ...observation,
      source: "runtime",
      sidecarLifecycle: "stopped",
    };
    vi.mocked(invoke).mockResolvedValue(stoppedObservation);

    await expect(runtimeObservationSnapshot()).resolves.toEqual(stoppedObservation);
  });

  it("只接受 revision 更大的观测", () => {
    expect(acceptNewerObservation(observation, { ...observation, revision: 2 })).toBe(observation);
    expect(acceptNewerObservation(observation, { ...observation, revision: 4 })).toEqual({
      ...observation,
      revision: 4,
    });
  });

  it("同 revision 只接受更晚的单调时间；旧 revision 即使时间更晚也拒绝", () => {
    const newerTime = { ...observation, observedAtMs: 600001 };
    expect(acceptNewerObservation(observation, newerTime)).toBe(newerTime);
    expect(acceptNewerObservation(observation, { ...observation })).toBe(observation);
    expect(acceptNewerObservation(observation, { ...observation, observedAtMs: 599999 })).toBe(observation);
    expect(acceptNewerObservation(observation, { ...newerTime, revision: 2 })).toBe(observation);
    expect(acceptNewerObservation(null, observation)).toBe(observation);
  });

  it("接受窗口两端的有效零值和最多 600 个点", async () => {
    const trafficHistory = Array.from({ length: 600 }, (_, index) => ({
      sampledAtMs: index === 599 ? 600000 : index * 1000,
      uploadRateBps: 0, downloadRateBps: 1.5,
    }));
    vi.mocked(invoke).mockResolvedValue({ ...observation, trafficHistory });
    await expect(runtimeObservationSnapshot()).resolves.toMatchObject({ trafficHistory });
  });

  it.each([
    { observedAtMs: -1 }, { observedAtMs: 0.5 }, { observedAtMs: Number.MAX_SAFE_INTEGER + 1 },
    { trafficHistory: null },
    { trafficHistory: Array.from({ length: 601 }, (_, sampledAtMs) => ({ sampledAtMs, uploadRateBps: 0, downloadRateBps: 0 })) },
    { trafficHistory: [{ sampledAtMs: 600001, uploadRateBps: 0, downloadRateBps: 0 }] },
    { observedAtMs: 600001, trafficHistory: [{ sampledAtMs: 0, uploadRateBps: 0, downloadRateBps: 0 }] },
    { trafficHistory: [{ sampledAtMs: -1, uploadRateBps: 0, downloadRateBps: 0 }] },
    { trafficHistory: [{ sampledAtMs: 0.5, uploadRateBps: 0, downloadRateBps: 0 }] },
    { trafficHistory: [{ sampledAtMs: 1, uploadRateBps: -1, downloadRateBps: 0 }] },
    { trafficHistory: [{ sampledAtMs: 1, uploadRateBps: 0, downloadRateBps: Infinity }] },
    { trafficHistory: [{ sampledAtMs: 1, uploadRateBps: NaN, downloadRateBps: 0 }] },
    { trafficHistory: [{ sampledAtMs: 1, uploadRateBps: 0 }] },
    { trafficHistory: [{ sampledAtMs: 1, uploadRateBps: 0, downloadRateBps: 0, secret: "fixture" }] },
    { trafficHistory: [1, 1].map((sampledAtMs) => ({ sampledAtMs, uploadRateBps: 0, downloadRateBps: 0 })) },
    { trafficHistory: [2, 1].map((sampledAtMs) => ({ sampledAtMs, uploadRateBps: 0, downloadRateBps: 0 })) },
  ])("拒绝不符合封闭趋势契约的 payload %j", async (change) => {
    vi.mocked(invoke).mockResolvedValue({ ...observation, ...change });
    await expect(runtimeObservationSnapshot()).rejects.toThrow("invalid runtime observation payload");
  });

  it.each(["observedAtMs", "trafficHistory"])("拒绝缺少 %s 的旧 DTO", async (field) => {
    const payload: Record<string, unknown> = { ...observation };
    delete payload[field];
    vi.mocked(invoke).mockResolvedValue(payload);
    await expect(runtimeObservationSnapshot()).rejects.toThrow("invalid runtime observation payload");
  });

  it("启动和停止只调用固定零参数命令", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("started").mockResolvedValueOnce("stopped");

    await expect(startManagedObservationRuntime()).resolves.toBe("started");
    await expect(stopManagedObservationRuntime()).resolves.toBe("stopped");

    expect(invoke).toHaveBeenNthCalledWith(1, "start_managed_observation_runtime");
    expect(invoke).toHaveBeenNthCalledWith(2, "stop_managed_observation_runtime");
  });

  it("拒绝未知的运行时控制结果", async () => {
    vi.mocked(invoke).mockResolvedValue("secret");

    await expect(startManagedObservationRuntime()).rejects.toThrow(
      "invalid managed runtime start result",
    );
  });

  it.each([
    "started", "alreadyRunning", "stateUnavailable", "configurationFailed", "startFailed", "busy",
  ])("接受封闭的启动结果 %s", async (result) => {
    vi.mocked(invoke).mockResolvedValue(result);
    await expect(startManagedObservationRuntime()).resolves.toBe(result);
    expect(invoke).toHaveBeenCalledWith("start_managed_observation_runtime");
  });

  it.each(["ConfigurationFailed", "configurationFailed: secret", { result: "configurationFailed" }, null])(
    "拒绝不属于封闭集合的启动结果 %j",
    async (result) => {
      vi.mocked(invoke).mockResolvedValue(result);
      await expect(startManagedObservationRuntime()).rejects.toThrow("invalid managed runtime start result");
    },
  );

  it("拒绝未知的停止结果", async () => {
    vi.mocked(invoke).mockResolvedValue("configurationFailed");
    await expect(stopManagedObservationRuntime()).rejects.toThrow("invalid managed runtime stop result");
  });
});
