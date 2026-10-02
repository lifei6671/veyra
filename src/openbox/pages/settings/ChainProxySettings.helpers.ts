import { normalizeGeoIpInfo } from "../../api/client";
import type { ChainLatencyRequest, ChainLatencyResponse, ChainProxy, GeoIpInfo, GroupsResponse, StorageResponse } from "../../api/types";

export type ChainFields = { scheme: string; host: string; port: string; username: string; password: string };
export type ChainDraft = { id: string; enabled: boolean; name: string; link: string; upstream: string; mode: "link" | "fields"; fields: ChainFields };
export type ChainTestResult = { ok: boolean; ms?: number; error?: string; via?: string; ip?: GeoIpInfo; ipError?: string };

export function parseChainFields(link: string): ChainFields | null {
  const match = /^(https?|socks5h?|socks):\/\/(?:([^@\/?#]*)@)?(\[[^\]]+\]|[^:\/?#@]+):(\d{1,5})(?:[\/?#].*)?$/i.exec(link.trim());
  if (!match) return null;
  let auth = match[2] || "";
  try { auth = decodeURIComponent(auth); } catch { /* Keep malformed encoded credentials editable, as in the original. */ }
  const separator = auth.indexOf(":");
  return { scheme: match[1].toLowerCase().startsWith("socks") ? "socks5" : match[1].toLowerCase(), host: match[3], port: match[4], username: separator < 0 ? auth : auth.slice(0, separator), password: separator < 0 ? "" : auth.slice(separator + 1) };
}

export function chainDraft(item?: ChainProxy): ChainDraft {
  const fields = item?.link ? parseChainFields(item.link) : null;
  return { id: item?.id || `x${Date.now().toString(36)}${Math.floor(Math.random() * 1e6).toString(36)}`, enabled: item?.enabled !== false, name: item?.name ?? "", link: item?.link ?? "", upstream: item?.upstream ?? "", mode: fields || !item?.link ? "fields" : "link", fields: fields ?? { scheme: "http", host: "", port: "", username: "", password: "" } };
}

export function chainConnection(draft: ChainDraft) {
  let link = draft.link.trim();
  if (draft.mode === "fields") {
    const fields = draft.fields, port = fields.port.trim();
    if (!fields.host.trim()) throw new Error("请填 IP 或域名");
    if (!/^\d{1,5}$/.test(port) || Number(port) < 1 || Number(port) > 65535) throw new Error("端口要在 1 到 65535 之间");
    const auth = fields.username || fields.password ? `${encodeURIComponent(fields.username.trim())}:${encodeURIComponent(fields.password)}@` : "";
    link = `${fields.scheme}://${auth}${fields.host.trim()}:${port}`;
  }
  if (!link) throw new Error("请填节点内容");
  if (!draft.upstream) throw new Error("请选择上游");
  return { link, upstream: draft.upstream };
}

export function saveChainDraft(draft: ChainDraft): ChainProxy {
  const name = draft.name.trim();
  if (!name) throw new Error("名称不能为空");
  const connection = chainConnection(draft);
  if (connection.upstream === name) throw new Error("上游不能是它自己");
  return { id: draft.id, enabled: draft.enabled, name, ...connection };
}

export function chainUpstreamOptions(groups: GroupsResponse | null, item?: ChainProxy): GroupsResponse | null {
  return groups && { ...groups, groups: groups.groups.filter(group => !group.kind && group.enabled !== false), availableNodes: groups.availableNodes.filter(node => node.name !== item?.name) };
}

export function chainLatencyRequest(connection: { link: string; upstream: string }, storage: StorageResponse, now = Date.now()): ChainLatencyRequest {
  const entries = storage.entries, provider = entries["config/geoip-info-api"] || "ip.sb";
  const urls: Record<string, string> = { "ip.sb": `https://api.ip.sb/geoip?t=${now}`, "ipwho.is": `https://ipwho.is?t=${now}${(entries["config/language"] || "zh-CN") === "zh-CN" ? "&lang=zh-CN" : ""}`, "ipapi.is": `https://api.ipapi.is?t=${now}` };
  return { link: connection.link, upstream: connection.upstream, testUrl: entries["config/speedtest-url"] || "http://www.gstatic.com/generate_204", timeoutMs: Number(entries["config/speedtest-timeout"] || 5000), ipUrls: [provider, ...Object.keys(urls).filter(key => key !== provider)].map(key => urls[key] ?? urls["ip.sb"]) };
}

const FAILURE_REASONS: Record<string, string> = { timeout: "超时", closed: "连接被断开", refused: "端口拒绝连接", dns: "域名解析失败", tls: "TLS 握手失败", unreachable: "网络不可达", invalid: "节点配置无效", "not-found": "节点已不在订阅里", unstable: "时通时断", error: "连接失败" };
export function chainTestResult(response: ChainLatencyResponse): ChainTestResult {
  const result: ChainTestResult = { ok: response.ok, ms: response.ms, via: response.via, error: response.ok ? undefined : [FAILURE_REASONS[response.reason || ""] || "超时", response.error !== response.reason && response.error].filter(Boolean).join(":") };
  if (response.ip?.ok && response.ip.body) {
    try {
      const body = JSON.parse(response.ip.body);
      const hostname = new URL(response.ip.url || "https://api.ip.sb").hostname;
      const provider = hostname === "ipwho.is" ? "ipwho.is" : hostname === "api.ipapi.is" ? "ipapi.is" : "ip.sb";
      const location = body.location;
      result.ip = { ...normalizeGeoIpInfo(provider, body, ""), region: location?.state ?? body.region ?? "", city: location?.city ?? body.city ?? "" };
      if (provider === "ipapi.is" && !location) {
        const asn = /^AS(\d+)\s*(.*)$/.exec(String(body.asn || ""));
        result.ip.asn = asn ? Number(asn[1]) : null;
        result.ip.organization = asn?.[2] || body.company || "";
      }
      if (result.ip.countryCode) result.ip.country = new Intl.DisplayNames(["zh-CN"], { type: "region" }).of(result.ip.countryCode) || result.ip.country;
    } catch (reason) { result.ipError = reason instanceof Error ? reason.message : String(reason); }
  } else if (response.ip) result.ipError = response.ip.error || "failed";
  return result;
}

export function chainLocation(ip: GeoIpInfo) {
  return [ip.country, ip.city && ip.city !== ip.country ? ip.city : ip.region && ip.region !== ip.country ? ip.region : ""].filter(Boolean).join(" ");
}

export function chainIpDetailsUrl(ip: string, provider: string) {
  return provider === "ipwho.is" ? `https://ipwho.is/${encodeURIComponent(ip)}` : provider === "ipapi.is" ? `https://api.ipapi.is/?q=${encodeURIComponent(ip)}` : `https://ip.sb/geoip/${encodeURIComponent(ip)}`;
}
