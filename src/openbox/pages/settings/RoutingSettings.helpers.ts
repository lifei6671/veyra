import type { CustomRoutingRule, RoutingPolicy, RoutingRuleType, RuleSetEntry } from "../../api/types";
import catalog from "../../assets/geo-catalog.json";

export type RuleRow = CustomRoutingRule & { key: number; note: string };
export const RULE_TYPES: Array<{ value: RoutingRuleType; label: string; placeholder: string }> = [
  { value: "domainSuffix", label: "域名后缀", placeholder: "google.com" },
  { value: "domain", label: "域名", placeholder: "www.example.com" },
  { value: "domainKeyword", label: "域名关键词", placeholder: "google" },
  { value: "ipCidr", label: "IP 段", placeholder: "8.8.8.8/32" },
  { value: "geosite", label: "geosite（域名集）", placeholder: "cn、google、netflix" },
  { value: "geoip", label: "geoip（IP 集）", placeholder: "cn、google、netflix" },
  { value: "ruleUrl", label: "规则集链接", placeholder: "https://例子/rules.list · 支持 .list / .mrs / .srs" },
  { value: "ruleset", label: "规则集", placeholder: "geosite-google（一行一个）" },
  { value: "port", label: "端口", placeholder: "51820 或 1000-2000,多个用逗号隔开" },
];
const summaryFields = ["rulesets", "ruleUrls", "domainSuffix", "domain", "domainKeyword", "ipCidr"] as const;
export function policySummary(policy: RoutingPolicy) {
  return summaryFields.flatMap(field => policy[field]?.length ? [`${field === "rulesets" ? "规则集" : RULE_TYPES.find(type => type.value === (field === "ruleUrls" ? "ruleUrl" : field))!.label}: ${policy[field]!.join(", ")}`] : []).join(" · ") || "没有匹配条件";
}
export function policyRuleRows(policy: RoutingPolicy | null): RuleRow[] {
  if (!policy) return [{ key: 1, type: "domainSuffix", value: "", outbound: "", note: "" }];
  const rows: RuleRow[] = [];
  for (const field of ["ruleUrls", "rulesets", "domainSuffix", "domain", "domainKeyword", "ipCidr"] as const) {
    for (const value of policy[field] ?? []) {
      const geo = /^(geosite|geoip)-(.+)$/.exec(value);
      rows.push({ key: rows.length + 1, type: field === "rulesets" ? geo ? geo[1] as RoutingRuleType : "ruleset" : field === "ruleUrls" ? "ruleUrl" : field, value: field === "rulesets" && geo ? geo[2] : value, outbound: "", note: policy.notes?.[`${field === "rulesets" ? "ruleset" : field === "ruleUrls" ? "ruleUrl" : field}:${value}`] ?? "" });
    }
  }
  return rows.length ? rows : policyRuleRows(null);
}
export function splitRuleRows(rows: RuleRow[]): RuleRow[] {
  let key = Math.max(0, ...rows.map(row => row.key));
  return rows.flatMap(row => {
    if (!["domainSuffix", "domain", "domainKeyword", "ipCidr"].includes(row.type)) return [row];
    const parts = row.value.split(/[\s,，;；、]+/).map(value => value.trim()).filter(Boolean);
    return parts.length ? parts.map((value, index) => ({ ...row, key: index ? ++key : row.key, value, note: index ? "" : row.note })) : [{ ...row, value: "" }];
  });
}
export function sortPolicyRules(rows: RuleRow[]) {
  const order: Partial<Record<RoutingRuleType, number>> = { ruleUrl: 0, geosite: 1, geoip: 1, ruleset: 1, domainSuffix: 2, domain: 3, domainKeyword: 4, ipCidr: 5 };
  return [...rows].sort((a, b) => (order[a.type] ?? 9) - (order[b.type] ?? 9));
}
export function rulesValidation(rows: RuleRow[], custom: boolean) {
  const active = splitRuleRows(rows).filter(row => row.value.trim());
  if (custom && active.some(row => !row.outbound)) return "每条规则都要选一个出口。";
  if (active.some(row => ["geosite", "geoip", "ruleset"].includes(row.type) && !/^[A-Za-z0-9._!@-]+$/.test(row.value.trim()))) return "规则集名称只能包含字母、数字、点、下划线、连字符、! 和 @。";
  if (custom && active.some(row => row.type === "port" && !/^\d{1,5}(-\d{1,5})?([,\s]+\d{1,5}(-\d{1,5})?)*[,\s]*$/.test(row.value.trim()))) return "端口要写成 51820 或 1000-2000,多个用逗号隔开。";
  if (!custom && !active.length) return "至少要填一条规则,否则这个站点集永远不会命中。";
  return "";
}
export function applyPolicyRules(policy: RoutingPolicy, rows: RuleRow[]): RoutingPolicy {
  const result: RoutingPolicy = { ...policy, rulesets: [], ruleUrls: [], domainSuffix: [], domain: [], domainKeyword: [], ipCidr: [], notes: {} };
  for (const row of splitRuleRows(rows)) {
    const value = row.value.trim(), note = row.note.trim();
    if (!value || row.type === "port") continue;
    const geo = row.type === "geosite" || row.type === "geoip";
    const field = geo || row.type === "ruleset" ? "rulesets" : row.type === "ruleUrl" ? "ruleUrls" : row.type as "domainSuffix" | "domain" | "domainKeyword" | "ipCidr";
    result[field]!.push(geo ? `${row.type}-${value}` : value);
    if (note) result.notes![`${geo || row.type === "ruleset" ? "ruleset" : row.type}:${geo ? `${row.type}-${value}` : value}`] = note;
  }
  return result;
}
export function importRuleRows(rows: RuleRow[], entries: RuleSetEntry[]): RuleRow[] {
  const types: Record<string, RoutingRuleType> = { domain: "domain", domain_suffix: "domainSuffix", domain_keyword: "domainKeyword", ip_cidr: "ipCidr" };
  const supported = entries.filter(entry => types[entry.type] && entry.value.trim());
  if (supported.length > 200) throw new Error(`共 ${supported.length} 条，超过 200 条不能导入明细——请改用「规则集链接」直接引用这个地址。`);
  let key = Math.max(0, ...rows.map(row => row.key));
  const outbound = rows.at(-1)?.outbound ?? "";
  return [...rows.filter(row => row.value.trim()), ...supported.map(entry => ({ key: ++key, type: types[entry.type], value: entry.value, outbound, note: "" }))];
}
export type GeoKind = "geosite" | "geoip";
export const GEO_CATALOG: Record<GeoKind, string[][]> = catalog;
const geoRows = {
  geosite: new Map(GEO_CATALOG.geosite.map(row => [row[0], row])),
  geoip: new Map(GEO_CATALOG.geoip.map(row => [row[0], row])),
};
export function geoDescription(kind: GeoKind, code: string) {
  const rows = geoRows[kind];
  const row = rows.get(code);
  if (row?.[1]) return row[1];
  const [base, attr] = code.split("@");
  const suffix: Record<string, string> = { ads: "广告域名", cn: "中国大陆部分", "!cn": "中国大陆以外" };
  const description = rows.get(base)?.[1];
  return suffix[attr] ? [description, suffix[attr]].filter(Boolean).join(" · ") : "";
}
export function filterGeoCategories(kind: GeoKind, search: string, excluded: string[], current: string, scope: string) {
  let rows = GEO_CATALOG[kind].filter(row => row[0] === current || !excluded.includes(row[0]));
  if (kind === "geoip" && scope !== "all") rows = rows.filter(row => /^[a-z]{2}$/i.test(row[0]) === (scope === "region"));
  const query = search.trim().toLowerCase();
  if (!query) return rows;
  const score = (code: string) => code === query ? 0 : code.startsWith(`${query}@`) ? 1 : code.startsWith(query) ? 2 : code.includes(query) ? 3 : 4;
  return rows.filter(row => row.some(value => value.toLowerCase().includes(query))).sort((a, b) => score(a[0]) - score(b[0]) || a[0].length - b[0].length);
}
