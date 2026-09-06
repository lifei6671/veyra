import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  activateSubscription,
  deleteSubscription,
  editSubscription,
  formatRelativeTime,
  getSubscriptionSettings,
  getSubscriptionShareUrl,
  importSubscription,
  listSubscriptions,
  normalizedSubscriptionName,
  safeFileDisplayName,
  subscriptionErrorMessage,
  subscribeSubscriptionStateChanged,
  updateSubscription,
  unicodeScalarLength,
  utf8Length,
} from "./subscriptions";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const summary = {
  id: "sub-fixture",
  name: "示例订阅",
  description: "",
  sourceKind: "remote",
  nodeCount: 2,
  skippedNodeCount: 0,
  lastSuccessAtMs: 1_700_000_000_000,
  traffic: { upload: 1, download: 2, total: 10, expireAtMs: null },
  allowAutoUpdate: true,
  updateIntervalMinutes: null,
  active: false,
  activeConfigurationGeneration: null,
  shareable: true,
};

describe("subscription IPC", () => {
  beforeEach(() => vi.mocked(invoke).mockReset());

  it("没有可导入节点时返回安全提示且不自动切换 UA", async () => {
    vi.mocked(invoke).mockResolvedValue({ status: "error", error: "parseFailed" });
    await expect(importSubscription({ name: "fixture", description: "", source: { kind: "manual", content: "proxies: []" } }))
      .resolves.toEqual({ status: "error", error: "parseFailed" });
    expect(subscriptionErrorMessage("parseFailed")).toContain("没有可导入的节点");
    expect(vi.mocked(invoke)).toHaveBeenCalledTimes(1);
  });

  it("使用三个固定命令和封闭 request 参数", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({ status: "ok", subscriptions: [summary] })
      .mockResolvedValueOnce({ status: "ok", subscription: summary })
      .mockResolvedValueOnce({ status: "ok", subscription: summary, contentChanged: false });

    await expect(listSubscriptions()).resolves.toMatchObject({ status: "ok" });
    const request = { name: "远程订阅", description: "", source: { kind: "remote" as const, url: "https://example.test/sub", options: { timeoutSeconds: 30, proxyMode: "direct" as const, verifyTls: true, allowAutoUpdate: true, intervalMinutes: null } } };
    await expect(importSubscription(request)).resolves.toMatchObject({ status: "ok" });
    await expect(updateSubscription({ id: "sub-fixture" })).resolves.toMatchObject({ status: "ok", contentChanged: false });

    expect(invoke).toHaveBeenNthCalledWith(1, "list_subscriptions");
    expect(invoke).toHaveBeenNthCalledWith(2, "import_subscription", {
      request,
    });
    expect(invoke).toHaveBeenNthCalledWith(3, "update_subscription", { request: { id: "sub-fixture" } });
  });

  it("保留 null 流量字段而不伪造零", async () => {
    vi.mocked(invoke).mockResolvedValue({
      status: "ok",
      subscriptions: [{ ...summary, lastSuccessAtMs: null, traffic: { upload: null, download: null, total: null, expireAtMs: null } }],
    });
    await expect(listSubscriptions()).resolves.toEqual({
      status: "ok",
      subscriptions: [{ ...summary, lastSuccessAtMs: null, traffic: { upload: null, download: null, total: null, expireAtMs: null } }],
    });
  });

  it.each([
    { ...summary, sourceKind: "secret" },
    { ...summary, nodeCount: -1 },
    { ...summary, skippedNodeCount: -1 },
    { ...summary, skippedNodeCount: 1.5 },
    { ...summary, skippedNodeCount: 0x1_0000_0000 },
    { ...summary, skippedNodeCount: undefined },
    { ...summary, traffic: { upload: 1, download: 2, total: 10, expireAtMs: null, url: "secret" } },
    { ...summary, url: "https://credential.test" },
  ])("拒绝未知或越界安全摘要 %j", async (payload) => {
    vi.mocked(invoke).mockResolvedValue({ status: "ok", subscriptions: [payload] });
    await expect(listSubscriptions()).rejects.toThrow("invalid subscription response");
  });

  it("只接受固定错误码且映射固定脱敏文案", async () => {
    vi.mocked(invoke).mockResolvedValue({ status: "error", error: "saveFailed" });
    await expect(listSubscriptions()).resolves.toEqual({ status: "error", error: "saveFailed" });
    expect(subscriptionErrorMessage("saveFailed")).toBe("订阅保存失败，原有内容未更改");

    vi.mocked(invoke).mockResolvedValue({ status: "error", error: "saveFailed: secret" });
    await expect(listSubscriptions()).rejects.toThrow("invalid subscription response");
  });

  it("使用五个新增固定命令并封装 request", async () => {
    const settings = {
      id: summary.id, name: summary.name, description: "", sourceKind: "remote",
      urlPreview: "https://example.test/sub", hasCustomUserAgent: false,
      remoteRequest: { timeoutSeconds: 30, proxyMode: "direct", verifyTls: true },
      updatePolicy: { allowAutoUpdate: true, intervalMinutes: null }, shareable: true,
    };
    const active = { ...summary, active: true, activeConfigurationGeneration: 2 };
    vi.mocked(invoke)
      .mockResolvedValueOnce({ status: "ok", settings })
      .mockResolvedValueOnce({ status: "ok", subscription: summary, contentChanged: false })
      .mockResolvedValueOnce({ status: "ok", outcome: "activated", operationId: "operation-1", subscription: active, activeConfigurationGeneration: 2 })
      .mockResolvedValueOnce({ status: "ok", url: "https://example.test/sub?token=fixture" })
      .mockResolvedValueOnce({ status: "ok", deletedId: summary.id });
    await expect(getSubscriptionSettings(summary.id)).resolves.toEqual({ status: "ok", settings });
    await expect(editSubscription({ id: summary.id, description: "说明" })).resolves.toMatchObject({ status: "ok" });
    await expect(activateSubscription(summary.id)).resolves.toMatchObject({ status: "ok", outcome: "activated" });
    await expect(getSubscriptionShareUrl(summary.id)).resolves.toMatchObject({ status: "ok" });
    await expect(deleteSubscription(summary.id)).resolves.toEqual({ status: "ok", deletedId: summary.id });
    for (let index = 0; index < 5; index += 1) {
      expect(vi.mocked(invoke).mock.calls[index][1]).toEqual({ request: expect.objectContaining({ id: summary.id }) });
    }
  });

  it("接受 8192 字节分享链接并拒绝更长响应", async () => {
    const prefix = "https://share.test/";
    const accepted = prefix + "a".repeat(8192 - utf8Length(prefix));
    vi.mocked(invoke).mockResolvedValueOnce({ status: "ok", url: accepted });
    await expect(getSubscriptionShareUrl(summary.id)).resolves.toEqual({ status: "ok", url: accepted });

    vi.mocked(invoke).mockResolvedValueOnce({ status: "ok", url: `${accepted}a` });
    await expect(getSubscriptionShareUrl(summary.id)).rejects.toThrow("invalid subscription response");
  });

  it("严格拒绝旧 selected 摘要和 active/generation 不一致", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ status: "ok", subscriptions: [{ ...summary, selected: false }] });
    await expect(listSubscriptions()).rejects.toThrow("invalid subscription response");
    vi.mocked(invoke).mockResolvedValueOnce({ status: "ok", subscriptions: [{ ...summary, active: true }] });
    await expect(listSubscriptions()).rejects.toThrow("invalid subscription response");
  });

  it("穷举接受 activate pending/error 并拒绝非法组合", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ status: "pending", operationId: "operation-2" });
    await expect(activateSubscription(summary.id)).resolves.toEqual({ status: "pending", operationId: "operation-2" });
    vi.mocked(invoke).mockResolvedValueOnce({ status: "error", operationId: "operation-busy", error: "busy" });
    await expect(activateSubscription(summary.id)).resolves.toMatchObject({ status: "error", error: "busy" });
    vi.mocked(invoke).mockResolvedValueOnce({ status: "pending", operationId: "operation-3", error: "busy" });
    await expect(activateSubscription(summary.id)).rejects.toThrow("invalid subscription response");
    vi.mocked(invoke).mockResolvedValueOnce({ status: "error", operationId: null, error: "busy" });
    await expect(activateSubscription(summary.id)).rejects.toThrow("invalid subscription response");
    vi.mocked(invoke).mockResolvedValueOnce({ status: "error", operationId: "operation-4", error: "identityFailed" });
    await expect(activateSubscription(summary.id)).rejects.toThrow("invalid subscription response");
  });

  it("订阅变更事件只接受固定安全字段", async () => {
    vi.mocked(listen).mockResolvedValue(vi.fn());
    const received: unknown[] = [];
    await subscribeSubscriptionStateChanged((event) => received.push(event));
    const handler = vi.mocked(listen).mock.calls[0][1] as (event: { payload: unknown }) => void;
    handler({ payload: { id: "sub-fixture", change: "edited" } });
    expect(received).toEqual([{ id: "sub-fixture", change: "edited" }]);
    expect(() => handler({ payload: { id: "sub-fixture", change: "edited", url: "secret" } })).toThrow("invalid subscription response");
  });
});

describe("subscription UI helpers", () => {
  it("空名称使用固定安全默认名且不从 URL 派生", () => {
    expect(normalizedSubscriptionName("  ", "remote")).toBe("远程订阅");
    expect(normalizedSubscriptionName("  ", "manual")).toBe("本地订阅");
    expect(normalizedSubscriptionName("  我的订阅  ", "remote")).toBe("我的订阅");
  });

  it("文件显示名移除控制字符并限制为 80 个字符", () => {
    expect(safeFileDisplayName("\u0000 本地.yaml ")).toBe("本地.yaml");
    expect(Array.from(safeFileDisplayName("订".repeat(100)))).toHaveLength(80);
  });

  it("名称边界按 Unicode 标量而非 UTF-16 码元计数", async () => {
    const emoji = "😀";
    expect(unicodeScalarLength(emoji.repeat(41))).toBe(41);
    expect(unicodeScalarLength(emoji.repeat(80))).toBe(80);
    expect(unicodeScalarLength(emoji.repeat(81))).toBe(81);

    vi.mocked(invoke).mockResolvedValueOnce({ status: "ok", subscriptions: [{ ...summary, name: emoji.repeat(80) }] });
    await expect(listSubscriptions()).resolves.toMatchObject({ status: "ok" });

    vi.mocked(invoke).mockResolvedValueOnce({ status: "ok", subscriptions: [{ ...summary, name: emoji.repeat(81) }] });
    await expect(listSubscriptions()).rejects.toThrow("invalid subscription response");
  });

  it("按 UTF-8 字节而非 UTF-16 长度执行边界", () => {
    expect(utf8Length("订阅")).toBe(6);
  });

  it("相对时间使用分钟、小时和天", () => {
    const now = 10 * 86_400_000;
    expect(formatRelativeTime(now - 10_000, now)).toBe("刚刚");
    expect(formatRelativeTime(now - 120_000, now)).toBe("2 分钟前");
    expect(formatRelativeTime(now - 7_200_000, now)).toBe("2 小时前");
    expect(formatRelativeTime(now - 172_800_000, now)).toBe("2 天前");
  });
});


it("preserves the safe skipped-node count in subscription summaries", async () => {
  vi.mocked(invoke).mockResolvedValue({ status: "ok", subscriptions: [{ ...summary, skippedNodeCount: 1 }] });
  const result = await listSubscriptions();
  expect(result.status).toBe("ok");
  if (result.status === "ok") expect(result.subscriptions[0].skippedNodeCount).toBe(1);
});
