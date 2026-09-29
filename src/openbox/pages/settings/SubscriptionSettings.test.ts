import { describe, expect, it } from "vitest";
import type { GroupsResponse, ProxiesResponse, Subscription } from "../../api/types";
import { emptyForm, formFromSubscription, subscriptionNodesFromApi, subscriptionNodeTestPayload, subscriptionPayload } from "./SubscriptionSettings";

const subscription: Subscription = {
  id: "subscription-id",
  name: "订阅 A",
  format: "auto",
  nodeCount: 3,
  createdAt: 1,
  updatedAt: 2,
  kernelStale: 0,
};

const groups: GroupsResponse = {
  groups: [],
  types: [],
  availableGroups: [],
  availableNodes: [
    { name: "节点慢", subscription: "订阅 A" },
    { name: "节点快", subscription: "订阅 A" },
    { name: "节点缺失", subscription: "订阅 A" },
    { name: "其他节点", subscription: "订阅 B" },
  ],
};

const proxies: ProxiesResponse = {
  proxies: {
    节点慢: { name: "节点慢", type: "VLESS", udp: true },
    节点快: { name: "节点快", type: "Hysteria2", udp: true },
    其他节点: { name: "其他节点", type: "VLESS" },
  },
};

describe("subscription settings API projection", () => {
  it("uses the original add-dialog rule defaults without inheriting API subscription state", () => {
    const form = emptyForm();

    expect(form.renameOptions).toEqual({
      enabled: true,
      usePrefix: true,
      template: "{region}-{feature}-{seq}",
      seqPad: 2,
      unknownLabel: "其他",
      featureKeywords: ["iepl", "iplc", "ipv6", "专线", "家宽", "2x"],
      excludeKeywords: ["官网", "工单", "客服"],
      disabled: [],
      overrides: {},
      regionDict: expect.arrayContaining([
        expect.objectContaining({ code: "US", name: "美国" }),
        expect.objectContaining({ code: "CN", name: "中国" }),
      ]),
    });
    expect(form.renameOptions.regionDict).toHaveLength(9);

    form.sourceType = "paste";
    form.content = "vless://node-from-editor";
    expect(subscriptionPayload(form)).toEqual({
      name: "订阅",
      content: "vless://node-from-editor",
      autoUpdate: { enabled: false, days: 1, hour: 4 },
      nodeDns: null,
      renameOptions: form.renameOptions,
    });
  });

  it("joins only backend-declared subscription nodes and orders them by API latency", () => {
    const result = subscriptionNodesFromApi(subscription, groups, proxies, {
      节点慢: [{ time: "2026-09-29T10:00:00Z", delay: 320 }],
      节点快: [{ time: "2026-09-29T10:00:00Z", delay: 180 }],
    });

    expect(result.map(node => node.name)).toEqual(["节点快", "节点慢"]);
    expect(result.map(node => node.history?.at(-1)?.delay)).toEqual([180, 320]);
    expect(result.some(node => node.name === "节点缺失")).toBe(false);
    expect(result.some(node => node.name === "其他节点")).toBe(false);
  });

  it("projects an API subscription into the editor and PATCH payload without invented fields", () => {
    const source: Subscription = {
      ...subscription,
      name: "API 订阅",
      urls: ["https://example.com/a", "https://example.com/b"],
      autoUpdate: { enabled: true, days: 7, hour: 4 },
      nodeDns: { url: "https://dns.example/dns-query", bootstrap: "1.1.1.1" },
      renameOptions: { enabled: true, usePrefix: true, featureKeywords: ["ipv6"] },
    };

    const form = formFromSubscription(source);
    expect(form.sourceType).toBe("url");
    expect(form.urls).toEqual(source.urls);
    expect(subscriptionPayload(form)).toEqual({
      name: source.name,
      urls: source.urls,
      autoUpdate: source.autoUpdate,
      nodeDns: source.nodeDns,
      renameOptions: expect.objectContaining(source.renameOptions),
    });
  });

  it("uses the backend content contract for pasted-node subscriptions", () => {
    const form = formFromSubscription({ ...subscription, content: "vless://node", autoUpdate: null });
    expect(subscriptionPayload(form)).toEqual(expect.objectContaining({
      name: subscription.name,
      content: "vless://node",
      autoUpdate: { enabled: false, days: 1, hour: 4 },
      nodeDns: null,
    }));
    expect(subscriptionPayload(form)).not.toHaveProperty("urls");
  });

  it("uses the original node-latency API payload without save-only fields", () => {
    const form = formFromSubscription({
      ...subscription,
      name: "API 订阅",
      urls: ["https://example.com/a", "https://example.com/a", ""],
      autoUpdate: { enabled: true, days: 7, hour: 4 },
      nodeDns: { url: "https://dns.example/dns-query", bootstrap: "1.1.1.1" },
      renameOptions: { enabled: true, usePrefix: true, overrides: { 原名: "新名" }, disabled: ["禁用节点"] },
    });

    expect(subscriptionNodeTestPayload(form, ["原名"], "https://example.com/generate_204", 5000)).toEqual({
      name: "API 订阅",
      urls: ["https://example.com/a"],
      renameOptions: form.renameOptions,
      nodeDns: form.nodeDns,
      tags: ["原名"],
      testUrl: "https://example.com/generate_204",
      timeoutMs: 5000,
    });
    expect(subscriptionNodeTestPayload(form, ["原名"], "https://example.com/generate_204", 5000)).not.toHaveProperty("autoUpdate");
  });
});
