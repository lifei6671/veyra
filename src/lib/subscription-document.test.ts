import { beforeEach, describe, expect, it, vi } from "vitest";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
import { activateSubscription, documentErrorMessage, formatSubscriptionDocument, getRunningConfiguration, getSubscriptionDocument, saveSubscriptionDocument } from "./subscriptions";
const revision = "a".repeat(64);
const summary = { id: "one", name: "One", description: "", sourceKind: "remote", nodeCount: 1, skippedNodeCount: 0, lastSuccessAtMs: 1, traffic: null, allowAutoUpdate: true, updateIntervalMinutes: 1440, active: true, activeConfigurationGeneration: 2, shareable: true };
beforeEach(() => invoke.mockReset());
describe("document command boundaries", () => {
  it("preserves exact YAML edits and distinguishes persisted application failure", async () => {
    const content = "# 保留注释\nproxies: []\n";
    invoke.mockResolvedValue({ status: "ok", subscription: summary, documentRevision: revision, apply: { status: "failed", operationId: "save-one", generation: 2, error: "startFailed" } });
    const request = { id: "one", content, format: "yaml" as const, expectedRevision: "b".repeat(64) };
    const result = await saveSubscriptionDocument(request);
    expect(invoke).toHaveBeenCalledExactlyOnceWith("save_subscription_document", { request });
    expect(result).toMatchObject({ status: "ok", documentRevision: revision, apply: { status: "failed", error: "startFailed" } });
  });
  it("rejects uncertain save receipts and unrecognized error payloads", async () => {
    for (const value of [{ status: "pending", operationId: "unknown" }, { status: "error", error: "constructor" }, { status: "error", error: "formatFailed", rawContent: "secret-fixture" }]) {
      invoke.mockResolvedValue(value);
      await expect(saveSubscriptionDocument({ id: "one", content: "{}", format: "json", expectedRevision: revision })).rejects.toThrow("invalid subscription response");
    }
  });
  it("only reads the explicitly requested subscription document", async () => {
    const document = { id: "one", format: "yaml", content: "# comment\nproxies: []", revision, sourceKind: "remote", localOverride: true };
    invoke.mockResolvedValue({ status: "ok", document });
    expect(await getSubscriptionDocument("one")).toEqual({ status: "ok", document });
    expect(invoke).toHaveBeenCalledExactlyOnceWith("get_subscription_document", { request: { id: "one" } });
    await expect(getSubscriptionDocument("different")).rejects.toThrow();
  });
  it("formats only through the explicit format command and reports safe locations", async () => {
    invoke.mockResolvedValue({ status: "error", error: "formatFailed", location: { line: 3, column: 2 } });
    const result = await formatSubscriptionDocument("invalid yaml", "yaml");
    expect(invoke).toHaveBeenCalledExactlyOnceWith("format_subscription_document", { request: { content: "invalid yaml", format: "yaml" } });
    if (result.status !== "error") throw Error("expected format failure");
    expect(documentErrorMessage(result)).toContain("第 3 行，第 2 列");
    invoke.mockResolvedValue({ ...result, location: { line: -1, column: 2 } });
    await expect(formatSubscriptionDocument("invalid", "yaml")).rejects.toThrow();
  });
  it("running configuration requires applied identity and refuses path-bearing payloads", async () => {
    const result = { status: "ok", format: "json", content: "{}", appliedSubscriptionId: "one", appliedConfigurationGeneration: 2 };
    invoke.mockResolvedValue(result); expect(await getRunningConfiguration()).toEqual(result);
    invoke.mockResolvedValue({ ...result, path: "private" }); await expect(getRunningConfiguration()).rejects.toThrow();
  });
  it("force activation is explicit and retains its distinct outcome", async () => {
    invoke.mockResolvedValue({ status: "ok", outcome: "reactivated", operationId: "force-one", subscription: summary, activeConfigurationGeneration: 2 });
    expect(await activateSubscription("one", true)).toMatchObject({ status: "ok", outcome: "reactivated" });
    expect(invoke).toHaveBeenCalledExactlyOnceWith("activate_subscription", { request: { id: "one", force: true } });
  });
});
