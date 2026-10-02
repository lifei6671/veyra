import type { ClientRoutingRule, KnownClient } from "../../api/types";

export type ClientRouteDraft = { id: string; enabled: boolean; name: string; match: "ip" | "mac"; sourcesText: string; macsText: string; mode: "route" | "bypass" | "admit"; outbound: string };

export function clientMatch(rule: ClientRoutingRule): "ip" | "mac" {
  return rule.match === "ip" || rule.match === "mac" ? rule.match : (rule.bypass || rule.admit) && rule.macs?.length ? "mac" : "ip";
}

export function clientRouteDraft(rule?: ClientRoutingRule): ClientRouteDraft {
  return { id: rule?.id || `c${Date.now().toString(36)}${Math.floor(Math.random() * 1e6).toString(36)}`, enabled: rule?.enabled !== false, name: rule?.name ?? "", match: rule ? clientMatch(rule) : "ip", sourcesText: (rule?.sources ?? []).join("\n"), macsText: (rule?.macs ?? []).join("\n"), mode: rule?.admit ? "admit" : rule?.bypass ? "bypass" : "route", outbound: rule?.outbound ?? "" };
}

export function saveClientRoute(draft: ClientRouteDraft): ClientRoutingRule {
  const name = draft.name.trim();
  if (!name) throw new Error("名称不能为空");
  let sources: string[] = [], macs: string[] = [];
  if (draft.match === "ip") {
    sources = [...new Set(draft.sourcesText.split(/[\n,;]+/).map(value => value.trim()).filter(Boolean))];
    if (!sources.length) throw new Error("至少填一个终端");
    const invalid = sources.find(value => {
      const [address, prefix, ...rest] = value.split("/");
      const ipv4 = /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/.test(address);
      const ipv6 = !ipv4 && /^[0-9a-f:]+$/i.test(address) && address.includes(":") && address.split("::").length <= 2;
      return rest.length || !ipv4 && !ipv6 || prefix !== undefined && (!/^\d+$/.test(prefix) || Number(prefix) > (ipv4 ? 32 : 128));
    });
    if (invalid) throw new Error(`无效的 IP / 网段:${invalid}`);
  } else {
    macs = [...new Set(draft.macsText.split(/[\n,;\s]+/).map(value => value.trim().toLowerCase().replace(/-/g, ":")).filter(Boolean))];
    if (!macs.length) throw new Error("按 MAC 认终端,至少填一个 MAC 地址");
    const invalid = macs.find(value => !/^([0-9a-f]{2}[:-]){5}[0-9a-f]{2}$/i.test(value));
    if (invalid) throw new Error(`无效的 MAC 地址:${invalid}`);
  }
  const base = { id: draft.id, enabled: draft.enabled, name, match: draft.match, sources, macs };
  if (draft.mode === "bypass") return { ...base, outbound: "", bypass: true };
  if (draft.mode === "admit") return { ...base, outbound: "", admit: true };
  if (!draft.outbound) throw new Error("请选择出站");
  return { ...base, outbound: draft.outbound };
}

export function knownClientSelected(draft: ClientRouteDraft, client: KnownClient) {
  if (draft.match === "mac") return !!client.mac && draft.macsText.split(/[\n,;\s]+/).map(value => value.toLowerCase()).includes(client.mac.toLowerCase());
  const sources = draft.sourcesText.split("\n").map(value => value.trim()).filter(Boolean);
  return sources.includes(client.ip) || sources.includes(`${client.ip}/32`);
}

export function addKnownClient(draft: ClientRouteDraft, client: KnownClient): ClientRouteDraft {
  if (knownClientSelected(draft, client)) return draft;
  if (draft.match === "mac") return client.mac ? { ...draft, macsText: [...draft.macsText.split(/[\n,;\s]+/).map(value => value.trim().toLowerCase()).filter(Boolean), client.mac.toLowerCase()].join("\n") } : draft;
  return { ...draft, sourcesText: [...draft.sourcesText.split("\n").map(value => value.trim()).filter(Boolean), client.ip].join("\n") };
}

export function clientRouteSummary(rule: ClientRoutingRule, clients: KnownClient[]) {
  if (clientMatch(rule) === "mac") return (rule.macs ?? []).map(mac => {
    const client = clients.find(value => value.mac?.toLowerCase() === mac.toLowerCase());
    return client?.name ? `${mac} (${client.name})` : mac;
  }).join(" · ");
  return rule.sources.map(source => {
    const ip = source.replace(/\/(32|128)$/, ""), client = clients.find(value => value.ip === ip);
    return client?.name ? `${ip} (${client.name})` : source;
  }).join(" · ");
}

export function admittedClientCount(rules: ClientRoutingRule[]) {
  return new Set(rules.filter(rule => rule.enabled !== false && rule.admit).flatMap(rule => clientMatch(rule) === "mac" ? (rule.macs ?? []).map(value => value.toLowerCase().replace(/-/g, ":")) : rule.sources)).size;
}
