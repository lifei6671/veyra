import type { GroupsResponse, OpenBoxGroup } from "../../api/types";
import countries from "../../assets/group-countries.json";

export type GroupType = "urltest" | "selector" | "failover";
export type MemberOption = { kind: "group" | "node"; name: string; subscription: string };
export function memberFilterOptions(items: MemberOption[]) {
  return [
    { value: "", label: "全部" },
    ...(items.some(item => item.kind === "group") ? [{ value: "kind:group", label: "全部节点组" }] : []),
    ...(items.some(item => item.kind === "node") ? [{ value: "kind:node", label: "全部节点" }] : []),
    ...[...new Set(items.filter(item => item.kind === "node").map(item => item.subscription).filter(Boolean))].map(subscription => ({ value: `sub:${subscription}`, label: subscription })),
  ];
}

export const GROUP_TYPE_LABELS: Record<string, string> = {
  urltest: "自动择优（url-test）",
  selector: "手动选择（select）",
  failover: "故障转移（failover）",
};

export const COUNTRY_OPTIONS = countries;

export const DEFAULT_AUTO_COUNTRIES = ["HK", "TW", "SG", "JP", "KR", "US"];

function normalizeNodeName(value: string) {
  return value.replace(/[\u{1F1E6}-\u{1F1FF}]{2}/gu, flag => ` ${[...flag].map(letter => String.fromCharCode(letter.codePointAt(0)! - 127462 + 97)).join("")} `).toLowerCase();
}

function matchesKeyword(name: string, keyword: string) {
  const normalized = normalizeNodeName(keyword).trim();
  if (!normalized) return false;
  if (/^[a-z]{2,3}$/i.test(normalized)) {
    const cn2 = normalized === "cn" ? "(?!2(?![0-9]))" : "";
    return new RegExp(`(^|[^a-z])${normalized}${cn2}([^a-z]|$)`, "i").test(name);
  }
  return name.includes(normalized);
}

export function dynamicGroupMembers(group: Pick<OpenBoxGroup, "keywords">, nodes: GroupsResponse["availableNodes"]) {
  return nodes.map(node => node.name).filter(name => !group.keywords.length || group.keywords.some(keyword => matchesKeyword(normalizeNodeName(name), keyword)));
}

export function groupSummary(group: OpenBoxGroup, nodes: GroupsResponse["availableNodes"]) {
  if (group.kind === "direct") return "流量不经代理,直接从路由器出去。内核离不开它,停用只是站点集里选不到。";
  if (group.kind === "block") return "命中的流量直接丢弃。";
  if (group.type === "failover") {
    const lanes = group.lanes ?? [];
    const count = new Set(lanes.flatMap(lane => lane.members)).size;
    return `故障转移 · ${lanes.length} 个页签 · ${count} 个节点`;
  }
  const count = group.mode === "dynamic" ? dynamicGroupMembers(group, nodes).length : group.members.length;
  const base = `${group.mode === "dynamic" ? "动态组" : "静态组"} · 当前 ${count} 个节点`;
  return group.type === "urltest" ? `${base} · 检测间隔 ${group.interval || "300s"} · 容差 ${group.tolerance ?? 100}ms` : base;
}

export function buildAutoGroups(existing: OpenBoxGroup[], countries: string[], types: GroupType[], now = Date.now()) {
  const names = new Set(existing.map(group => group.name));
  const output: OpenBoxGroup[] = [];
  countries.forEach(code => {
    const country = COUNTRY_OPTIONS.find(item => item.code === code);
    if (!country) return;
    types.forEach(type => {
      const suffix = type === "urltest" ? "自动" : type === "selector" ? "手动" : "主备";
      const name = `${country.name}-${suffix}`;
      if (names.has(name)) return;
      names.add(name);
      output.push({
        id: `auto-${code.toLowerCase()}-${type}-${now}-${output.length}`,
        name,
        type,
        mode: "dynamic",
        icon: code,
        keywords: [...country.keywords],
        members: [],
        ...(type === "urltest" ? { interval: "300s", tolerance: 100 } : {}),
      });
    });
  });
  return output;
}

export function mergeAutoGroups(existing: OpenBoxGroup[], generated: OpenBoxGroup[]) {
  const countryCode = (group: OpenBoxGroup) => /^[A-Za-z]{2}$/.test(group.icon) ? group.icon.toUpperCase() : "";
  const countryCodes = [...new Set(generated.map(countryCode).filter(Boolean))];
  const typeOrder: Record<string, number> = { urltest: 0, selector: 1 };
  let result = [...existing];
  countryCodes.forEach(code => {
    const block = [...result.filter(item => countryCode(item) === code), ...generated.filter(item => countryCode(item) === code)]
      .sort((a, b) => (typeOrder[a.type] ?? 9) - (typeOrder[b.type] ?? 9));
    const index = result.findIndex(item => countryCode(item) === code);
    result = result.filter(item => countryCode(item) !== code);
    result.splice(index < 0 ? result.length : index, 0, ...block);
  });
  return [...result, ...generated.filter(group => !countryCode(group))];
}

export function cloneGroup(group: OpenBoxGroup): OpenBoxGroup {
  return {
    ...group,
    keywords: [...(group.keywords ?? [])],
    members: [...(group.members ?? [])],
    lanes: group.lanes?.map(lane => ({ ...lane, members: [...lane.members] })),
    failover: group.failover ? { ...group.failover } : undefined,
  };
}

export function normalizeEditorGroup(group: OpenBoxGroup): OpenBoxGroup {
  if (group.kind) return { ...group, name: group.name.trim(), iconScale: group.iconScale || 0 };
  const normalized = {
    ...group,
    name: group.name.trim(),
    keywords: [...group.keywords],
    members: [...group.members],
    iconScale: group.iconScale || 0,
    ...(group.type === "urltest" || group.type === "failover" ? {
      interval: `${Math.max(5, secondsValue(group.interval))}s`,
      tolerance: Math.max(0, group.tolerance ?? 100),
      testUrl: group.testUrl?.trim() ?? "",
    } : {}),
  };
  if (group.type === "failover") {
    return {
      ...normalized,
      mode: "static",
      members: [],
      keywords: [],
      lanes: (group.lanes ?? []).map(lane => ({
        id: lane.id,
        name: lane.name.trim(),
        icon: lane.icon || "",
        members: [...lane.members],
        ...(lane.manual ? { manual: true } : {}),
      })),
      failover: {
        timeoutMs: group.failover?.timeoutMs ?? 5000,
        failureThreshold: group.failover?.failureThreshold ?? 2,
        restorePrimary: group.failover?.restorePrimary ?? true,
        recoveryHoldMs: group.failover?.recoveryHoldMs ?? 60000,
      },
    };
  }
  const { lanes: _lanes, failover: _failover, ...plain } = normalized;
  return plain;
}

export function filterMembers(items: MemberOption[], search: string, filter: string) {
  const query = search.trim().toLowerCase();
  return items.filter(item => (!query || item.name.toLowerCase().includes(query)) && (!filter || filter === `kind:${item.kind}` || filter === `sub:${item.subscription}`));
}

export function countryNodeCount(keywords: readonly string[], nodes: GroupsResponse["availableNodes"]) {
  return nodes.filter(node => keywords.some(keyword => matchesKeyword(normalizeNodeName(node.name), keyword))).length;
}

export function availableAutoCountries(selected: string[], nodes: GroupsResponse["availableNodes"]) {
  return COUNTRY_OPTIONS.map(country => ({ ...country, count: countryNodeCount(country.keywords, nodes) }))
    .filter(country => country.count > 0 && !selected.includes(country.code))
    .sort((a, b) => b.count - a.count);
}

export function moveItem<T>(items: T[], from: number, to: number) {
  const next = [...items];
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}

export function commaValues(value: string) {
  return value.split(/[,，、]/).map(item => item.trim()).filter(Boolean);
}

export function secondsValue(value?: string) {
  const match = /^(\d+)(ms|s|m|h)$/.exec(value ?? "");
  if (!match) return 300;
  const amount = Number(match[1]);
  return Math.round(amount * (match[2] === "ms" ? .001 : match[2] === "m" ? 60 : match[2] === "h" ? 3600 : 1));
}

export function errorMessage(reason: unknown, fallback: string) {
  return reason instanceof Error ? reason.message : fallback;
}
