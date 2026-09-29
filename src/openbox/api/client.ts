import type {
  AuthStatus,
  ClientDevice,
  ClientsResponse,
  ControllerConfig,
  DnsFilterConfig,
  GeoIpInfo,
  GroupsResponse,
  OpenBoxGroup,
  OpenBoxProfile,
  ProxiesResponse,
  ProxyLatencyHistoryResponse,
  RulesResponse,
  ServiceStatus,
  SiteLatencyHistory,
  StorageResponse,
  Subscription,
  SubscriptionShare,
  TestSite,
  TrafficDay,
  TrafficDimension,
  TrafficDrill,
  TrafficMonth,
  UpdateStatus,
} from "./types";

const configuredBase = (import.meta.env.VITE_OPENBOX_API_BASE as string | undefined)?.trim().replace(/\/$/, "") ?? "";

export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly code?: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

function apiUrl(path: string) {
  return `${configuredBase}${path}`;
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set("Accept", "application/json");
  headers.set("Accept-Language", localStorage.getItem("config/language") || navigator.language || "zh-CN");
  headers.set("X-Zashboard-Locale", localStorage.getItem("config/language") || navigator.language || "zh-CN");
  if (init.body && !(init.body instanceof FormData)) headers.set("Content-Type", "application/json");

  const response = await fetch(apiUrl(path), { credentials: configuredBase ? "include" : "same-origin", ...init, headers });
  const contentType = response.headers.get("content-type") ?? "";
  const rawBody = await response.text().catch(() => "");
  const body = parseResponseBody(rawBody, contentType);
  if (!response.ok) {
    const errorBody = body && typeof body === "object" ? body as Record<string, unknown> : null;
    const code = errorBody ? String(errorBody.code ?? errorBody.error ?? "") : undefined;
    const message = errorBody ? String(errorBody.message ?? errorBody.error ?? "") : String(body || "");
    if (response.status === 401) window.dispatchEvent(new CustomEvent("openbox:unauthorized"));
    throw new ApiError(message || `请求失败 (${response.status})`, response.status, code);
  }
  return body as T;
}

export function parseResponseBody(rawBody: string, contentType: string) {
  const trimmed = rawBody.trim();
  if (!trimmed) return null;
  if (contentType.includes("application/json") || trimmed.startsWith("{") || trimmed.startsWith("[")) {
    try {
      return JSON.parse(trimmed) as unknown;
    } catch {
      return rawBody;
    }
  }
  return rawBody;
}

const json = (value: unknown): RequestInit => ({ body: JSON.stringify(value) });

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" ? value as Record<string, unknown> : {};
}

export function normalizeGeoIpInfo(provider: string, value: unknown, fallbackIp: string): GeoIpInfo {
  const body = objectValue(value);
  const connection = objectValue(body.connection);
  const location = objectValue(body.location);
  const asn = objectValue(body.asn);
  const numericAsn = Number(provider === "ipwho.is" ? connection.asn : provider === "ipapi.is" ? asn.asn : body.asn);
  return {
    ip: String(body.ip ?? fallbackIp),
    asn: Number.isFinite(numericAsn) && numericAsn > 0 ? numericAsn : null,
    countryCode: String(body.country_code ?? location.country_code ?? "").toUpperCase(),
    country: String(body.country ?? location.country ?? ""),
    organization: String(
      provider === "ipwho.is"
        ? connection.org ?? connection.isp ?? ""
        : provider === "ipapi.is"
          ? asn.org ?? asn.organization ?? ""
          : body.organization ?? body.asn_organization ?? body.isp ?? "",
    ),
  };
}

async function geoIp(ip: string, provider: string): Promise<GeoIpInfo> {
  const target = provider === "ipwho.is"
    ? `https://ipwho.is/${encodeURIComponent(ip)}`
    : provider === "ipapi.is"
      ? `https://api.ipapi.is/?q=${encodeURIComponent(ip)}`
      : `https://api.ip.sb/geoip/${encodeURIComponent(ip)}?t=${Date.now()}`;
  const response = await fetch(target, { headers: { Accept: "application/json" } });
  if (!response.ok) throw new ApiError(`IP 信息查询失败 (${response.status})`, response.status);
  return normalizeGeoIpInfo(provider, await response.json(), ip);
}

export function controllerSocketUrl(channel: "connections" | "logs" | "memory" | "traffic", query?: Record<string, string>) {
  const origin = configuredBase ? new URL(configuredBase, window.location.origin).origin : window.location.origin;
  const url = new URL(`/api/controller-ws/${channel}`, origin);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  Object.entries(query ?? {}).forEach(([key, value]) => url.searchParams.set(key, value));
  return url.toString();
}

export const api = {
  authStatus: () => request<AuthStatus>("/api/auth/status", { cache: "no-store" }),
  login: (password: string) => request<AuthStatus>("/api/auth/login", { method: "POST", ...json({ password }) }),
  setupPassword: (password: string) => request<AuthStatus>("/api/auth/setup", { method: "POST", ...json({ password }) }),
  changePassword: (currentPassword: string, newPassword: string) => request<AuthStatus>("/api/auth/change-password", { method: "POST", ...json({ currentPassword, newPassword }) }),

  storage: () => request<StorageResponse>("/api/storage"),
  patchStorage: (entries: Record<string, string>, removed: string[] = []) => request<unknown>("/api/storage", { method: "PATCH", ...json({ entries, removed }) }),

  serviceStatus: () => request<ServiceStatus>("/api/openbox/service/status"),
  serviceAction: (action: "start" | "stop" | "restart") => request<ServiceStatus>(`/api/openbox/service/core/${action}`, { method: "POST" }),
  controllerConfig: () => request<ControllerConfig>("/api/controller/configs"),
  version: () => request<{ version: string }>("/api/controller/version"),
  updateStatus: () => request<UpdateStatus>("/api/openbox/update/status"),

  proxies: () => request<ProxiesResponse>("/api/controller/proxies"),
  proxyLatencyHistory: () => request<ProxyLatencyHistoryResponse>("/api/openbox/latency-history"),
  selectProxy: (group: string, name: string) => request<void>(`/api/controller/proxies/${encodeURIComponent(group)}`, { method: "PUT", ...json({ name }) }),
  testProxy: (name: string, url: string, timeout: number) => request<{ delay: number }>(`/api/controller/proxies/${encodeURIComponent(name)}/delay?url=${encodeURIComponent(url)}&timeout=${timeout}`),
  testProxyGroup: (group: string, url: string, timeout: number) => request<Record<string, number>>(`/api/controller/group/${encodeURIComponent(group)}/delay?url=${encodeURIComponent(url)}&timeout=${timeout}`),
  geoIp,
  rules: () => request<RulesResponse>("/api/controller/rules"),
  closeConnection: (id: string) => request<void>(`/api/controller/connections/${encodeURIComponent(id)}`, { method: "DELETE" }),
  closeAllConnections: () => request<void>("/api/controller/connections", { method: "DELETE" }),

  siteLatencyHistory: () => request<SiteLatencyHistory>("/api/openbox/site-latency/history"),
  testSites: (sites: TestSite[]) => request<{ results?: Record<string, number> }>("/api/openbox/site-latency", { method: "POST", signal: AbortSignal.timeout(30_000), ...json({ sites }) }),
  trafficMonth: (month: string, countDirect = true) => request<TrafficMonth>(`/api/openbox/traffic/month?month=${encodeURIComponent(month)}${countDirect ? "" : "&direct=0"}`),
  trafficDay: (day: string, hour?: number | null, countDirect = true) => request<TrafficDay>(`/api/openbox/traffic/day?day=${encodeURIComponent(day)}&limit=500${hour == null ? "" : `&hour=${hour}`}${countDirect ? "" : "&direct=0"}`),
  trafficDrill: (day: string, kind: TrafficDimension, key: string, by: TrafficDimension, hour?: number | null, countDirect = true) => request<TrafficDrill>(`/api/openbox/traffic/drill?day=${encodeURIComponent(day)}&kind=${kind}&key=${encodeURIComponent(key)}&by=${by}&limit=200${hour == null ? "" : `&hour=${hour}`}${countDirect ? "" : "&direct=0"}`),

  groups: () => request<GroupsResponse>("/api/openbox/groups"),
  saveGroups: (groups: OpenBoxGroup[]) => request<GroupsResponse>("/api/openbox/groups", { method: "PUT", ...json({ groups }) }),

  subscriptions: async () => (await request<{ subscriptions: Subscription[] }>("/api/openbox/subscriptions")).subscriptions,
  subscriptionShares: async () => (await request<{ shares: SubscriptionShare[] }>("/api/openbox/subscription-shares")).shares,
  createSubscription: (value: Partial<Subscription>) => request<Subscription>("/api/openbox/subscriptions", { method: "POST", ...json(value) }),
  updateSubscription: (id: string, value: Partial<Subscription>) => request<Subscription>(`/api/openbox/subscriptions/${encodeURIComponent(id)}`, { method: "PATCH", ...json(value) }),
  refreshSubscription: (id: string) => request<void>(`/api/openbox/subscriptions/${encodeURIComponent(id)}/refresh`, { method: "POST", ...json({}) }),
  deleteSubscription: (id: string) => request<void>(`/api/openbox/subscriptions/${encodeURIComponent(id)}`, { method: "DELETE" }),

  profile: async () => (await request<{ profile: OpenBoxProfile }>("/api/openbox/profile")).profile,
  saveProfile: async (profile: OpenBoxProfile) => (await request<{ profile: OpenBoxProfile }>("/api/openbox/profile", { method: "PUT", ...json(profile) })).profile,
  clients: () => request<ClientsResponse>("/api/openbox/clients"),
  devices: async () => (await request<{ devices: ClientDevice[] }>("/api/openbox/client-config/devices")).devices,
  createDevice: (name: string) => request<ClientDevice>("/api/openbox/client-config/devices", { method: "POST", ...json({ name }) }),
  deleteDevice: (name: string) => request<void>(`/api/openbox/client-config/devices/${encodeURIComponent(name)}`, { method: "DELETE" }),
  dnsFilter: () => request<DnsFilterConfig>("/api/openbox/dns-filter"),
  saveDnsFilter: (config: DnsFilterConfig) => request<DnsFilterConfig>("/api/openbox/dns-filter", { method: "PUT", ...json(config) }),
  flushDns: () => request<void>("/api/openbox/dns/flush-cache", { method: "POST" }),
};
