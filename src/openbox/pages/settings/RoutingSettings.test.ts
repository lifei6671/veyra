import { describe, expect, it } from "vitest";
import { applyPolicyRules, filterGeoCategories, GEO_CATALOG, geoDescription, importRuleRows, policyRuleRows, policySummary, rulesValidation, sortPolicyRules, splitRuleRows, type RuleRow } from "./RoutingSettings.helpers";

const row = (key: number, type: RuleRow["type"], value: string, note = "", outbound = "直连"): RuleRow => ({ key, type, value, note, outbound });

describe("target routing editor", () => {
  it("round-trips all rule fields, geosite/geoip tags, legacy tags and notes without losing backend fields", () => {
    const policy = { id: "ai", name: "AI", enabled: false, icon: "brand:openai-light", iconScale: 2, customBackendField: 7, rulesets: ["geosite-category-ai-!cn", "geoip-cn", "legacy.list"], ruleUrls: ["https://example.com/rules.list"], domainSuffix: ["example.com"], domain: ["www.example.com"], domainKeyword: ["google"], ipCidr: ["8.8.8.8/32"], notes: { "ruleset:geosite-category-ai-!cn": "AI 服务", "ruleUrl:https://example.com/rules.list": "远端名单", "domainSuffix:example.com": "后缀" } };
    const rows = policyRuleRows(policy);
    expect(rows.map(value => value.type)).toEqual(["ruleUrl", "geosite", "geoip", "ruleset", "domainSuffix", "domain", "domainKeyword", "ipCidr"]);
    expect(rows[1]).toMatchObject({ value: "category-ai-!cn", note: "AI 服务" });
    expect(applyPolicyRules(policy, rows)).toEqual(policy);
  });
  it("splits multi-value text on the original separators, retains the first note and inherits each custom exit", () => {
    const rows = [row(1, "domainSuffix", " a.com，b.com; c.com\nd.com、e.com；f.com ", "备注", "所有-自动"), row(2, "port", "51820,1000-2000", "端口"), row(3, "ruleUrl", "https://example.com/list?q=a,b")];
    const result = splitRuleRows(rows);
    expect(result.slice(0, 6).map(value => value.value)).toEqual(["a.com", "b.com", "c.com", "d.com", "e.com", "f.com"]);
    expect(result.slice(0, 6).map(value => value.note)).toEqual(["备注", "", "", "", "", ""]);
    expect(result.slice(0, 6).every(value => value.outbound === "所有-自动")).toBe(true);
    expect(new Set(result.map(value => value.key)).size).toBe(result.length);
    expect(result.slice(6)).toEqual(rows.slice(1));
  });
  it("groups regular policy rows by source priority while retaining order within a type", () => {
    const rows = [row(1, "domain", "a"), row(2, "geoip", "cn"), row(3, "ipCidr", "8.8.8.8/32"), row(4, "geosite", "google"), row(5, "ruleUrl", "https://example.com/list"), row(6, "domain", "b")];
    expect(sortPolicyRules(rows).map(value => value.key)).toEqual([5, 2, 4, 1, 6, 3]);
    expect(rows.map(value => value.key)).toEqual([1, 2, 3, 4, 5, 6]);
  });
  it("requires rules for a site set, allows an empty custom list and validates exits, rule tags and ports", () => {
    expect(rulesValidation([row(1, "domain", " ")], false)).toContain("至少");
    expect(rulesValidation([], true)).toBe("");
    expect(rulesValidation([row(1, "domain", "a.com", "", "")], true)).toContain("出口");
    expect(rulesValidation([row(1, "geosite", "category-ai-!cn@ads")], false)).toBe("");
    expect(rulesValidation([row(1, "geoip", "bad/name")], false)).toContain("规则集");
    expect(rulesValidation([row(1, "port", "51820,1000-2000")], true)).toBe("");
    expect(rulesValidation([row(1, "port", "443/tcp")], true)).toContain("端口");
  });
  it("imports only supported entries, keeps URLs, removes blank rows, and enforces the 200-entry limit", () => {
    const rows = [row(1, "ruleUrl", "https://example.com/list"), row(2, "domain", "", "", "所有-手动")];
    const imported = importRuleRows(rows, [{ type: "domain_regex", value: ".*" }, { type: "domain", value: "a.com" }, { type: "domain_suffix", value: "b.com" }, { type: "ip_cidr", value: "8.8.8.8/32" }, { type: "domain_keyword", value: "" }]);
    expect(imported.map(value => value.type)).toEqual(["ruleUrl", "domain", "domainSuffix", "ipCidr"]);
    expect(imported.slice(1).every(value => value.outbound === "所有-手动")).toBe(true);
    expect(importRuleRows([], Array.from({ length: 200 }, () => ({ type: "domain", value: "a.com" })))).toHaveLength(200);
    expect(() => importRuleRows([], Array.from({ length: 201 }, () => ({ type: "domain", value: "a.com" })))).toThrow("超过 200");
  });
  it("renders summaries in the original field order", () => {
    expect(policySummary({ rulesets: ["geosite-google"], ruleUrls: ["https://example.com/list"], domainSuffix: ["a.com", "b.com"] })).toBe("规则集: geosite-google · 规则集链接: https://example.com/list · 域名后缀: a.com, b.com");
    expect(policySummary({})).toBe("没有匹配条件");
  });
});

describe("original geosite/geoip catalog", () => {
  it("preserves the full catalog and multilingual search with exact codes ranked first", () => {
    expect(GEO_CATALOG.geosite).toHaveLength(1899); expect(GEO_CATALOG.geoip).toHaveLength(260);
    expect(filterGeoCategories("geosite", "YouTube", [], "", "all")[0][0]).toBe("youtube");
    expect(filterGeoCategories("geosite", "中国大陆以外的 AI", [], "", "all").some(value => value[0] === "category-ai-!cn")).toBe(true);
    expect(geoDescription("geosite", "youtube@ads")).toBe("YouTube 视频服务 · 广告域名");
  });
  it("excludes categories used by other rows but keeps the current selection and geoip scope", () => {
    expect(filterGeoCategories("geosite", "youtube", ["youtube", "youtube@ads"], "youtube", "all").map(value => value[0])).toEqual(["youtube", "youtube@cn"]);
    expect(filterGeoCategories("geoip", "", [], "", "region").every(value => /^[a-z]{2}$/.test(value[0]))).toBe(true);
    expect(filterGeoCategories("geoip", "", [], "", "other").every(value => !/^[a-z]{2}$/.test(value[0]))).toBe(true);
  });
});
