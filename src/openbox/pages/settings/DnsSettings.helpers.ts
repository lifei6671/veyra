import type { DnsRewriteRule, DnsUpstream, OpenBoxProfile } from "../../api/types";
import { DNS_TEXT } from "./DnsSettings.messages";
export type UpstreamRow = DnsUpstream & { side: "direct" | "proxy"; index: number };
export const dnsMessage = (key: keyof typeof DNS_TEXT, args: Record<string, string | number> = {}) => DNS_TEXT[key].replace(/\{(\w+)\}/g, (_, name: string) => String(args[name] ?? `{${name}}`));
export const dnsError = (reason: unknown) => reason instanceof Error ? reason.message : String(reason);
export const normalizeDomain = (value: string) => value.trim().toLowerCase().replace(/\.+$/, "");
export const isDnsAddress = (value: string) => value !== "0.0.0.0" && value !== "::" && isRewriteAddress(value);
export function isRewriteAddress(value: string) {
  return /^(?:\d{1,3}\.){3}\d{1,3}$/.test(value) ? value.split(".").every(part => Number(part) <= 255) : value.includes(":") && /^[0-9a-f:]+$/i.test(value);
}
export function upstreamRows(dns: OpenBoxProfile["dns"]): UpstreamRow[] {
  return (["direct", "proxy"] as const).flatMap(side => [{ server: dns[side], protocol: dns[`${side}Protocol`] === "udp" ? "udp" as const : dns[`${side}Protocol`] === "tcp" || side === "proxy" ? "tcp" as const : "udp" as const, port: dns[`${side}Port`] || 53 }, ...(dns[`${side}Extras`] || []).slice(0, 3)].filter(row => row.server).map((row, index) => ({ ...row, side, index })));
}
export function upstreamPatch(rows: UpstreamRow[]): Partial<OpenBoxProfile["dns"]> {
  return Object.assign({}, ...(["direct", "proxy"] as const).map(side => {
    const values = rows.filter(row => row.side === side).map(({ server, protocol, port }) => ({ server, protocol, port }));
    if (!values.length) return {};
    return { [side]: values[0].server, [`${side}Protocol`]: values[0].protocol, [`${side}Port`]: values[0].port, [`${side}Extras`]: values.slice(1) };
  }));
}
export const defaultUpstreams: UpstreamRow[] = [{ side: "direct", index: 0, server: "223.5.5.5", protocol: "udp", port: 53 }, { side: "proxy", index: 0, server: "1.1.1.1", protocol: "tcp", port: 53 }];
export function upstreamValidation(row: UpstreamRow, rows: UpstreamRow[], original?: UpstreamRow): string {
  if (!isDnsAddress(row.server)) return DNS_TEXT.dnsUpstreamInvalid;
  if (!Number.isInteger(row.port) || row.port < 1 || row.port > 65535) return DNS_TEXT.dnsUpstreamInvalidPort;
  const others = rows.filter(item => !original || item.side !== original.side || item.index !== original.index).filter(item => item.side === row.side);
  if (others.some(item => item.server === row.server)) return DNS_TEXT.dnsUpstreamExtraDuplicate;
  if (others.length >= 4) return dnsMessage("dnsUpstreamTooMany", { max: 4 });
  return "";
}
const domainPattern = /^(?:\*\.)?(?:(?!-)[a-z0-9_-]{1,63}(?<!-)\.)*(?!-)[a-z0-9_-]{1,63}(?<!-)$/i;
export function rewriteValidation(rule: DnsRewriteRule, rules: DnsRewriteRule[], kind: string): string {
  if (!domainPattern.test(rule.source)) return DNS_TEXT.dnsRewriteBadSource;
  if (rules.some(item => item.id !== rule.id && normalizeDomain(item.source) === rule.source)) return dnsMessage("dnsRewriteDuplicate", { source: rule.source });
  if (kind === "domain" && (!domainPattern.test(rule.domain) || rule.domain.includes("*") || rule.domain === rule.source)) return DNS_TEXT.dnsRewriteBadDomain;
  if (kind === "ip" && (!rule.addresses.length || rule.addresses.some(value => !isRewriteAddress(value)))) return DNS_TEXT.dnsRewriteBadAddress;
  return "";
}
export function rewriteNeedsRestart(before: NonNullable<OpenBoxProfile["dns"]["rewrite"]>, after: NonNullable<OpenBoxProfile["dns"]["rewrite"]>) {
  const shape = (rules: DnsRewriteRule[]) => rules.map(rule => `${rule.id}\0${normalizeDomain(rule.source)}\0${rule.enabled !== false}`).sort().join("\n");
  return (before.enabled !== false) !== (after.enabled !== false) || shape(before.rules) !== shape(after.rules);
}
export function restoreRewrites(rules: DnsRewriteRule[], defaults: DnsRewriteRule[]) {
  const next = rules.map(rule => defaults.find(item => item.id === rule.id) || rule);
  for (const rule of defaults) if (!next.some(item => item.id === rule.id || normalizeDomain(item.source) === normalizeDomain(rule.source))) next.push(rule);
  return next;
}
export const parseAddresses = (value: string) => [...new Set(value.trim().toLowerCase().split(/[\n,;\s]+/).filter(Boolean))];
export const parseAllowDomains = (value: string) => [...new Set(value.split("\n").map(item => item.trim()).filter(Boolean))];
