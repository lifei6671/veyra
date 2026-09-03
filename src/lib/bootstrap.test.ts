import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { bootstrapStatus } from "./bootstrap";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("bootstrapStatus", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("只调用固定的启动信息命令", async () => {
    const expected = { application: "Veyra", status: "ready" };
    vi.mocked(invoke).mockResolvedValue(expected);

    await expect(bootstrapStatus()).resolves.toEqual(expected);
    expect(invoke).toHaveBeenCalledWith("bootstrap_status");
  });
});
