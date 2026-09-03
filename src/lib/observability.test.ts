import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  acceptNewerObservation,
  runtimeObservationSnapshot,
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

  it("只接受 revision 更大的观测", () => {
    expect(acceptNewerObservation(observation, { ...observation, revision: 2 })).toBe(observation);
    expect(acceptNewerObservation(observation, { ...observation, revision: 4 })).toEqual({
      ...observation,
      revision: 4,
    });
  });
});
