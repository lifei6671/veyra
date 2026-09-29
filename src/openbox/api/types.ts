export type RouteKey = "overview" | "proxies" | "connections" | "logs" | "rules" | "settings";

export type AuthStatus = {
  enabled: boolean;
  authenticated: boolean;
  passwordSet: boolean;
};

export type ServiceStatus = {
  core: { running: boolean; autostart: boolean; uptimeSeconds: number; raw?: string };
  panel: { running: boolean; raw?: string };
  conflicts: string[];
  platform: string;
};

export type ControllerConfig = {
  mode: string;
  "mode-list"?: string[];
  "log-level": string;
  ipv6: boolean;
  [key: string]: unknown;
};

export type ProxyHistory = { time: string; delay: number; node?: string };

export type ProxyLatencyHistoryResponse = {
  history: Record<string, ProxyHistory[]>;
  updatedAt: number;
};

export type ControllerProxy = {
  name: string;
  type: string;
  alive?: boolean;
  udp?: boolean;
  now?: string;
  all?: string[];
  history?: ProxyHistory[];
};

export type ProxiesResponse = { proxies: Record<string, ControllerProxy> };

export type GeoIpInfo = {
  ip: string;
  asn: number | null;
  countryCode: string;
  country: string;
  organization: string;
};

export type ControllerRule = { type: string; payload: string; proxy: string };
export type RulesResponse = { rules: ControllerRule[] };

export type RouteProbeMethod = "GET" | "HEAD" | "TCP" | "TLS";

export type PenetrationEntry = {
  type: string;
  value: string;
  source?: string;
};

export type PenetrationResult = {
  matched?: {
    index?: number;
    outbound?: string;
    entries?: PenetrationEntry[];
    entriesTotal?: number;
    rule?: Record<string, unknown>;
  };
  chain?: string[];
  finalOutbound?: string;
  resolved?: { addresses?: string[]; fakeIp?: boolean };
  owner?: { kind?: string; name?: string };
  firstLayer?: {
    dnsMode?: string;
    nativeBypass?: { enabled?: boolean; sets?: string[] };
    entryMode?: { mode?: string; reason?: string };
  };
  dns?: RouteDiagnosticDns;
};

export type TerminalTestCapability = {
  ok: boolean;
  missing?: string[];
  lan?: Record<string, unknown> | null;
};

export type RouteDiagnosticDns = {
  ruleIndex?: number;
  server?: { type?: string; tag?: string; server?: string; detour?: string };
  viaProxy?: boolean;
  fakeIpRule?: number;
  runtimeChain?: string[];
  runtimeLeaf?: string;
};

export type RouteDiagnosticResult = {
  target?: string;
  dns?: RouteDiagnosticDns;
  resolve?: {
    ok?: boolean;
    status?: number;
    answers?: string[];
    ms?: number;
    ttl?: number;
    cached?: boolean;
    error?: string;
  };
  exit?: {
    url?: string;
    connectTo?: string;
    ok?: boolean;
    status?: number;
    ms?: number;
    error?: string;
    chains?: string[];
    rule?: string;
    rulePayload?: string;
    destinationIP?: string;
    viaProxy?: boolean;
    owner?: { kind?: string; name?: string };
  };
  [key: string]: unknown;
};

export type TerminalTestResult = RouteDiagnosticResult & {
  sourceIP?: string;
  sourcePort?: number;
  port?: number;
};

export type ConnectionMetadata = {
  destinationIP: string;
  destinationPort: string;
  dnsMode?: string;
  host: string;
  inboundName?: string;
  inboundUser?: string;
  network: string;
  process?: string;
  processPath?: string;
  sniffHost?: string;
  sourceIP: string;
  sourcePort: string;
  type: string;
};

export type ControllerConnection = {
  id: string;
  chains: string[];
  download: number;
  upload: number;
  metadata: ConnectionMetadata;
  rule: string;
  rulePayload: string;
  start: string;
  downloadSpeed?: number;
  uploadSpeed?: number;
};

export type ConnectionsFrame = {
  connections: ControllerConnection[];
  closedConnections?: ControllerConnection[];
  downloadTotal?: number;
  uploadTotal?: number;
};

export type TrafficFrame = { up: number; down: number };
export type MemoryFrame = { inuse: number; oslimit?: number };
export type ControllerLogFrame = { type: string; payload: string };

export type SiteLatencyPoint = { time: string; delay: number; node?: string };
export type SiteLatencyHistory = {
  history: Record<string, SiteLatencyPoint[]>;
  timeoutMs: number;
};

export type TestSite = { id: string; icon: string; name: string; url: string };

export type TrafficSummary = { up: number; down: number; conns: number };
export type TrafficMonthDay = TrafficSummary & { day: string };
export type TrafficMonth = {
  month: string;
  today: string;
  days: TrafficMonthDay[];
  total: TrafficSummary;
  avg: { up: number; down: number };
  avgDays: number;
  direct: { excluded: boolean; tag: string };
};

export type TrafficRow = TrafficSummary & {
  key: string;
  name?: string;
  self?: { iface: string; kind: string };
};

export type TrafficHour = TrafficSummary & { hour: number };
export type TrafficDay = {
  day: string;
  hour: number | null;
  today: string;
  total: TrafficSummary;
  nodes: TrafficRow[];
  hosts: TrafficRow[];
  clients: TrafficRow[];
  hostsCount?: number;
  clientsCount?: number;
  other?: { up: number; down: number };
  hours: TrafficHour[];
  nowHour: number;
  hourDetailKeepDays?: number;
  direct: { excluded: boolean; tag: string };
};

export type TrafficDimension = "client" | "node" | "host";

export type TrafficDrill = {
  day: string;
  hour: number | null;
  kind: TrafficDimension;
  key: string;
  by: TrafficDimension;
  count: number;
  sum: { up: number; down: number };
  rows: TrafficRow[];
};

export type OpenBoxGroup = {
  id: string;
  name: string;
  type: "urltest" | "selector" | "failover" | string;
  mode: "static" | "dynamic" | string;
  enabled: boolean;
  icon: string;
  iconScale: number;
  keywords: string[];
  members: string[];
  kind?: string;
  testUrl?: string;
  interval?: string;
  tolerance?: number;
  idleTimeout?: string;
};

export type GroupsResponse = {
  groups: OpenBoxGroup[];
  types: string[];
  availableNodes: Array<{ name: string; subscription: string }>;
  availableGroups: string[];
};

export type Subscription = {
  id: string;
  name: string;
  url?: string;
  urls?: string[];
  format: string;
  nodeCount: number;
  renameOptions?: Record<string, unknown>;
  autoUpdate?: string | boolean;
  createdAt: string;
  updatedAt: string;
  kernelStale: boolean;
};

export type SubscriptionShare = {
  id: string;
  name: string;
  enabled?: boolean;
  url?: string;
  [key: string]: unknown;
};

export type RoutingPolicy = {
  name?: string;
  enabled?: boolean;
  icon?: string;
  outbound?: string;
  target?: string;
  [key: string]: unknown;
};

export type OpenBoxProfile = {
  ipv6: boolean;
  ipv6Proxy: string;
  directForNodes: boolean;
  rejectQuic: boolean;
  directBypass: boolean;
  bypassPorts: string;
  bypassPortsMode: string;
  tun: { autoRedirect: boolean; stack: string; mtu: number; tcpMss: number };
  chainProxies: Array<Record<string, unknown>>;
  servers: Array<Record<string, unknown>>;
  clientRoutes: Array<Record<string, unknown>>;
  dns: {
    split: boolean;
    mode: string;
    fakeIpForProxy: boolean;
    direct: string;
    directProtocol: string;
    directPort: number;
    directExtras: string[];
    proxy: string;
    proxyProtocol: string;
    proxyPort: number;
    proxyExtras: string[];
    rewrite?: Record<string, unknown>;
  };
  traffic: { keepMonths: number };
  testUrl: string;
  directTestUrl: string;
  updates: {
    openbox: { auto: boolean; hour: number; channel: string; checkChannel: string; days: number };
  };
  routing: {
    policies: RoutingPolicy[];
    fallbackDefault: string;
    fallbackName: string;
    fallbackIcon: string;
  };
  [key: string]: unknown;
};

export type StorageResponse = { entries: Record<string, string> };

export type UpdateStatus = {
  version: string;
  singboxVersion: string;
  builtAt: string;
  geoVersion: string;
  geoDate: string;
  geoCounts: { geosite: number; geoip: number };
  channel: { mode: string; prefix: string };
  status: { stage: string; running: boolean };
  logTail: string;
};

export type ClientDevice = { id?: string; name: string; [key: string]: unknown };
export type ClientsResponse = { clients?: Array<Record<string, unknown>>; devices?: ClientDevice[] };

export type DnsFilterConfig = {
  enabled?: boolean;
  [key: string]: unknown;
};
