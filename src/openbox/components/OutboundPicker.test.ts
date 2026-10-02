import { describe, expect, it } from "vitest";
import { resolveOutboundProxy } from "./OutboundPicker.helpers";

describe("shared outbound latency", () => {
  it("resolves the currently selected leaf through groups and stops on a controller cycle", () => {
    const leaf = { name: "美国节点", type: "Vless", history: [{ time: "2026-10-02", delay: 123 }] };
    const proxies = { proxies: { leaf, auto: { name: "auto", type: "URLTest", now: "leaf" }, manual: { name: "manual", type: "Selector", now: "auto" }, cycle: { name: "cycle", type: "Selector", now: "cycle" } } };
    expect(resolveOutboundProxy("manual", proxies)).toBe(leaf);
    expect(resolveOutboundProxy("cycle", proxies)).toBeUndefined();
  });
});
