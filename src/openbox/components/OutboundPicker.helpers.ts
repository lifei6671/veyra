import type { ProxiesResponse } from "../api/types";

export function resolveOutboundProxy(name: string, proxies: ProxiesResponse | null) {
  const visited = new Set<string>();
  let current = proxies?.proxies[name];
  while (current?.now) {
    if (visited.has(current.name)) return undefined;
    visited.add(current.name);
    current = proxies?.proxies[current.now];
  }
  return current;
}
