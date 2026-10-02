import { describe, expect, it } from "vitest";
import type { GroupsResponse, OpenBoxGroup } from "../../api/types";
import {
  availableAutoCountries,
  buildAutoGroups,
  COUNTRY_OPTIONS,
  countryNodeCount,
  dynamicGroupMembers,
  groupSummary,
  mergeAutoGroups,
  memberFilterOptions,
  moveItem,
  normalizeEditorGroup,
} from "./GroupSettings.helpers";

const nodes: GroupsResponse["availableNodes"] = [
  { name: "🇺🇸 US-01", subscription: "订阅 A" },
  { name: "🇭🇰 HK-01", subscription: "订阅 A" },
  { name: "新加坡 SG-01", subscription: "订阅 B" },
];

const dynamicGroup: OpenBoxGroup = {
  id: "all-auto",
  name: "所有-自动",
  type: "urltest",
  mode: "dynamic",
  enabled: true,
  icon: "globe:earth-asia",
  iconScale: 0,
  keywords: [],
  members: [],
  interval: "300s",
  tolerance: 100,
};

describe("outbound group API projection", () => {
  it("offers member filters only for the kinds and subscriptions present in each pane", () => {
    expect(memberFilterOptions([])).toEqual([{ value: "", label: "全部" }]);
    expect(memberFilterOptions([{ kind: "group", name: "直连", subscription: "" }])).toEqual([
      { value: "", label: "全部" }, { value: "kind:group", label: "全部节点组" },
    ]);
    expect(memberFilterOptions(nodes.map(node => ({ ...node, kind: "node" })))).toEqual([
      { value: "", label: "全部" }, { value: "kind:node", label: "全部节点" },
      { value: "sub:订阅 A", label: "订阅 A" }, { value: "sub:订阅 B", label: "订阅 B" },
    ]);
  });
  it("renders dynamic counts from backend availableNodes without mock members", () => {
    expect(dynamicGroupMembers(dynamicGroup, nodes)).toEqual(nodes.map(node => node.name));
    expect(groupSummary(dynamicGroup, nodes)).toBe("动态组 · 当前 3 个节点 · 检测间隔 300s · 容差 100ms");
    expect(dynamicGroupMembers({ keywords: ["hk", "新加坡"] }, nodes)).toEqual(["🇭🇰 HK-01", "新加坡 SG-01"]);
  });

  it("keeps the original built-in descriptions and immutable kind marker", () => {
    const direct: OpenBoxGroup = { ...dynamicGroup, id: "builtin-direct", name: "直连", type: "direct", kind: "direct" };
    const normalized = normalizeEditorGroup({ ...direct, name: "  直连出口  " });

    expect(groupSummary(direct, nodes)).toBe("流量不经代理,直接从路由器出去。内核离不开它,停用只是站点集里选不到。");
    expect(normalized.kind).toBe("direct");
    expect(normalized.name).toBe("直连出口");
  });

  it("preserves the failover API lane contract and renders its real lane summary", () => {
    const failover = normalizeEditorGroup({
      ...dynamicGroup,
      id: "failover",
      name: "  主备出口  ",
      type: "failover",
      mode: "dynamic",
      keywords: ["us"],
      members: ["旧成员"],
      lanes: [
        { id: "primary", name: " 主用 ", icon: "US", members: ["🇺🇸 US-01"] },
        { id: "backup", name: " 备用 ", icon: "HK", members: ["🇭🇰 HK-01", "🇺🇸 US-01"], manual: true },
      ],
      failover: { timeoutMs: 7000, failureThreshold: 3, restorePrimary: false, recoveryHoldMs: 90000 },
    });

    expect(failover).toMatchObject({
      name: "主备出口",
      mode: "static",
      members: [],
      keywords: [],
      lanes: [
        { id: "primary", name: "主用", icon: "US", members: ["🇺🇸 US-01"] },
        { id: "backup", name: "备用", icon: "HK", members: ["🇭🇰 HK-01", "🇺🇸 US-01"], manual: true },
      ],
      failover: { timeoutMs: 7000, failureThreshold: 3, restorePrimary: false, recoveryHoldMs: 90000 },
    });
    expect(groupSummary(failover, nodes)).toBe("故障转移 · 2 个页签 · 2 个节点");

    const selector = normalizeEditorGroup({ ...failover, type: "selector", mode: "static", members: ["🇺🇸 US-01"] });
    expect(selector).not.toHaveProperty("lanes");
    expect(selector).not.toHaveProperty("failover");
  });

  it("builds auto groups from real country keywords and does not duplicate names", () => {
    const generated = buildAutoGroups([dynamicGroup], ["US", "HK"], ["urltest", "selector"], 123);

    expect(generated.map(group => group.name)).toEqual(["美国-自动", "美国-手动", "香港-自动", "香港-手动"]);
    expect(generated[0]).toMatchObject({ id: "auto-us-urltest-123-0", mode: "dynamic", icon: "US", interval: "300s", tolerance: 100 });
    expect(generated[0]).not.toHaveProperty("enabled");
    expect(generated[0]).not.toHaveProperty("iconScale");
    expect(generated[1]).not.toHaveProperty("interval");
    expect(generated[1]).not.toHaveProperty("tolerance");
    expect(buildAutoGroups([{ ...dynamicGroup, name: "美国-自动" }], ["US"], ["urltest"], 123)).toEqual([]);
  });

  it("recognizes flag-only names, short-code boundaries and CN2 exactly like the source", () => {
    const available = ["🇺🇸 01", "US-02", "Russia", "Australia", "🇨🇳 03", "CN2 GIA 美国", "CN20", "CN2", "CN-04", "🇫🇷 Paris", "India"].map(name => ({ name, subscription: "A" }));
    expect(dynamicGroupMembers({ keywords: ["US"] }, available)).toEqual(["🇺🇸 01", "US-02"]);
    expect(dynamicGroupMembers({ keywords: ["cn"] }, available)).toEqual(["🇨🇳 03", "CN20", "CN-04"]);
    expect(dynamicGroupMembers({ keywords: ["   "] }, available)).toEqual([]);
    expect(countryNodeCount(COUNTRY_OPTIONS.find(country => country.code === "FR")!.keywords, available)).toBe(1);
    expect(countryNodeCount(COUNTRY_OPTIONS.find(country => country.code === "IN")!.keywords, available)).toBe(1);
  });

  it("offers only recognized unselected countries, ordered by node count", () => {
    const available = [...nodes, { name: "US-02", subscription: "C" }, { name: "🇫🇷 01", subscription: "C" }];
    expect(availableAutoCountries(["HK", "SG"], available).map(country => [country.code, country.count])).toEqual([["US", 2], ["FR", 1]]);
    expect(availableAutoCountries([], [])).toEqual([]);
  });

  it("allows default countries with zero nodes and skips duplicate names independently of icon and type", () => {
    const existing = [{ ...dynamicGroup, name: "法国-自动", icon: "", type: "selector" }];
    const generated = buildAutoGroups(existing, ["FR", "HK"], ["urltest", "selector"], 456);
    expect(generated.map(group => group.name)).toEqual(["法国-手动", "香港-自动", "香港-手动"]);
    expect(generated[0].id).toBe("auto-fr-selector-456-0");
    expect(buildAutoGroups([...existing, ...generated], ["FR", "HK"], ["urltest", "selector"], 456)).toEqual([]);
  });

  it("keeps unrelated groups and groups each country at its existing first position", () => {
    const existing = [
      dynamicGroup,
      { ...dynamicGroup, id: "hk-manual", name: "香港-手动", icon: "hk", type: "selector" },
      { ...dynamicGroup, id: "unrelated", name: "其他", icon: "globe:earth-asia" },
      { ...dynamicGroup, id: "hk-other", name: "香港其他", icon: "HK", type: "failover" },
    ];
    const generated = buildAutoGroups(existing, ["US", "HK"], ["selector", "urltest"], 123);
    expect(mergeAutoGroups(existing, generated).map(group => group.name)).toEqual(["所有-自动", "香港-自动", "香港-手动", "香港其他", "其他", "美国-自动", "美国-手动"]);
  });

  it("preserves country adjacency when generated groups are merged and supports reorder placeholders", () => {
    const existing = [{ ...dynamicGroup, id: "us-existing", name: "美国-手动", icon: "US", type: "selector" }];
    const generated = buildAutoGroups(existing, ["US", "HK"], ["urltest"], 123);
    const merged = mergeAutoGroups(existing, generated);

    expect(merged.map(group => group.name)).toEqual(["美国-自动", "美国-手动", "香港-自动"]);
    expect(moveItem(["直连", "所有-自动", "所有-手动", "拒绝"], 1, 2)).toEqual(["直连", "所有-手动", "所有-自动", "拒绝"]);
  });
});
