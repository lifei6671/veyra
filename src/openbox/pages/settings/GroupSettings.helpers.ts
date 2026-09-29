import dartIcon from "../../assets/group-dart.svg";
import earthAsiaIcon from "../../assets/group-earth-asia.svg";
import earthMeridiansIcon from "../../assets/group-earth-meridians.svg";
import crossIcon from "../../assets/group-cross.svg";
import type { GroupsResponse, OpenBoxGroup } from "../../api/types";

export type GroupType = "urltest" | "selector" | "failover";
export type MemberOption = { kind: "group" | "node"; name: string; subscription: string };
export type IconCategory = "region" | "brand" | "other";

export const GROUP_TYPE_LABELS: Record<string, string> = {
  urltest: "自动择优（url-test）",
  selector: "手动选择（select）",
  failover: "故障转移（failover）",
};

export const COUNTRY_OPTIONS = [
  { code: "HK", name: "香港", keywords: ["hk", "hong kong", "hongkong", "香港", "深港"] },
  { code: "TW", name: "台湾", keywords: ["tw", "taiwan", "台湾", "台灣", "臺灣", "台北"] },
  { code: "SG", name: "新加坡", keywords: ["sg", "singapore", "新加坡", "狮城", "獅城"] },
  { code: "JP", name: "日本", keywords: ["jp", "japan", "日本", "东京", "東京", "大阪"] },
  { code: "KR", name: "韩国", keywords: ["kr", "korea", "韩国", "韓國", "首尔", "首爾"] },
  { code: "US", name: "美国", keywords: ["us", "united states", "america", "美国", "美國", "洛杉矶", "洛杉磯", "硅谷", "圣何塞", "西雅图", "纽约"] },
  { code: "GB", name: "英国", keywords: ["uk", "gb", "united kingdom", "britain", "英国", "英國", "伦敦", "倫敦"] },
  { code: "DE", name: "德国", keywords: ["de", "germany", "德国", "德國", "法兰克福", "法蘭克福"] },
  { code: "CN", name: "中国", keywords: ["cn", "china", "中国", "中國", "回国", "回國", "back to china"] },
] as const;

export const DEFAULT_AUTO_COUNTRIES = ["HK", "TW", "SG", "JP", "KR", "US"];

export const ICON_ASSETS: Record<string, string> = {
  "misc:dart": dartIcon,
  "globe:earth-asia": earthAsiaIcon,
  "globe:earth-meridians": earthMeridiansIcon,
  "misc:cross": crossIcon,
};

export const ICON_OPTIONS: Array<{ code: string; label: string; category: IconCategory }> = [
  { code: "globe:earth-meridians", label: "地球·彩色", category: "other" },
  { code: "globe:earth-asia", label: "地球·彩色(亚洲)", category: "other" },
  { code: "misc:dart", label: "靶心", category: "other" },
  { code: "misc:cross", label: "拒绝(叉)", category: "other" },
  ...COUNTRY_OPTIONS.map(country => ({ code: country.code, label: country.name, category: "region" as const })),
];

export function dynamicGroupMembers(group: Pick<OpenBoxGroup, "keywords">, nodes: GroupsResponse["availableNodes"]) {
  const keywords = group.keywords.map(keyword => keyword.trim().toLowerCase()).filter(Boolean);
  return nodes.map(node => node.name).filter(name => !keywords.length || keywords.some(keyword => name.toLowerCase().includes(keyword)));
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
        enabled: true,
        icon: code,
        iconScale: 0,
        keywords: [...country.keywords],
        members: [],
        ...(type === "urltest" ? { interval: "300s", tolerance: 100 } : {}),
      });
    });
  });
  return output;
}

export function mergeAutoGroups(existing: OpenBoxGroup[], generated: OpenBoxGroup[]) {
  const countryCodes = [...new Set(generated.map(group => /^[A-Za-z]{2}$/.test(group.icon) ? group.icon.toUpperCase() : "").filter(Boolean))];
  const typeOrder: Record<string, number> = { urltest: 0, selector: 1, failover: 2 };
  let result = [...existing];
  countryCodes.forEach(code => {
    const block = [...result.filter(item => item.icon.toUpperCase() === code), ...generated.filter(item => item.icon.toUpperCase() === code)]
      .sort((a, b) => (typeOrder[a.type] ?? 9) - (typeOrder[b.type] ?? 9));
    const index = result.findIndex(item => item.icon.toUpperCase() === code);
    result = result.filter(item => item.icon.toUpperCase() !== code);
    result.splice(index < 0 ? result.length : index, 0, ...block);
  });
  return [...result, ...generated.filter(group => !countryCodes.includes(group.icon.toUpperCase()))];
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
  return nodes.filter(node => keywords.some(keyword => node.name.toLowerCase().includes(keyword.toLowerCase()))).length;
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

export function countryFlag(code: string) {
  return String.fromCodePoint(...[...code].map(character => 127397 + character.charCodeAt(0)));
}

export function errorMessage(reason: unknown, fallback: string) {
  return reason instanceof Error ? reason.message : fallback;
}
