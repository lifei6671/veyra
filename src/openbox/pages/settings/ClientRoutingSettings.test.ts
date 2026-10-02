import { describe, expect, it } from "vitest";
import { addKnownClient, admittedClientCount, clientMatch, clientRouteDraft, clientRouteSummary, knownClientSelected, saveClientRoute } from "./ClientRoutingSettings.helpers";
import type { ClientRoutingRule } from "../../api/types";

const rule: ClientRoutingRule = { id: "tv", name: "电视", sources: ["192.168.1.10/32"], outbound: "所有-自动", enabled: false };

describe("terminal routing editor contracts", () => {
  it("round-trips a disabled route and infers the original legacy MAC bypass/admit mode", () => {
    expect(saveClientRoute(clientRouteDraft(rule))).toEqual({ ...rule, match: "ip", macs: [] });
    expect(clientMatch({ ...rule, macs: ["aa:bb:cc:dd:ee:ff"] })).toBe("ip");
    expect(clientMatch({ ...rule, bypass: true, macs: ["aa:bb:cc:dd:ee:ff"] })).toBe("mac");
    const legacy = { ...rule, admit: true, bypass: true, macs: ["aa:bb:cc:dd:ee:ff"] };
    expect(clientRouteDraft(legacy)).toMatchObject({ match: "mac", mode: "admit", enabled: false });
  });
  it("normalizes and deduplicates IP sources without altering CIDR or IPv6", () => {
    const draft = { ...clientRouteDraft(rule), name: " 电视 ", sourcesText: "192.168.1.10;192.168.1.0/24,192.168.1.10\n2001:db8::1/128\n2001:db8::/64" };
    expect(saveClientRoute(draft)).toMatchObject({ name: "电视", sources: ["192.168.1.10", "192.168.1.0/24", "2001:db8::1/128", "2001:db8::/64"], macs: [] });
  });
  it("validates names, sources, IP prefixes and the route exit", () => {
    const draft = clientRouteDraft(rule);
    expect(() => saveClientRoute({ ...draft, name: " " })).toThrow("名称不能为空");
    expect(() => saveClientRoute({ ...draft, sourcesText: "" })).toThrow("至少填一个终端");
    for (const sourcesText of ["256.1.1.1", "192.168.1.1/33", "2001:db8::1/129", "192.168.1.1/-1", "192.168.1.1/24/8", "a.example"]) expect(() => saveClientRoute({ ...draft, sourcesText })).toThrow("无效的 IP / 网段");
    expect(() => saveClientRoute({ ...draft, outbound: "" })).toThrow("请选择出站");
  });
  it("saves only the active match and mode, clearing stale exits and the other mode flags", () => {
    const draft = { ...clientRouteDraft(rule), match: "mac" as const, macsText: "AA-BB-CC-DD-EE-FF aa:bb:cc:dd:ee:ff\n11:22:33:44:55:66" };
    const macs = ["aa:bb:cc:dd:ee:ff", "11:22:33:44:55:66"];
    expect(saveClientRoute({ ...draft, mode: "bypass" })).toEqual({ id: "tv", name: "电视", enabled: false, match: "mac", sources: [], macs, outbound: "", bypass: true });
    expect(saveClientRoute({ ...draft, mode: "admit" })).toMatchObject({ sources: [], macs, outbound: "", admit: true });
    expect(saveClientRoute({ ...draft, mode: "route" })).not.toHaveProperty("bypass");
    expect(() => saveClientRoute({ ...draft, macsText: "" })).toThrow("至少填一个 MAC");
    expect(() => saveClientRoute({ ...draft, macsText: "not-a-mac" })).toThrow("无效的 MAC");
  });
  it("appends known devices once and leaves the other match input untouched", () => {
    const draft = clientRouteDraft(rule), client = { ip: "192.168.1.10", name: "电视", mac: "AA:BB:CC:DD:EE:FF" };
    expect(knownClientSelected(draft, client)).toBe(true);
    expect(addKnownClient(draft, client)).toBe(draft);
    expect(addKnownClient(draft, { ip: "192.168.1.11" }).sourcesText).toBe("192.168.1.10/32\n192.168.1.11");
    const byMac = { ...draft, match: "mac" as const };
    const added = addKnownClient(byMac, client);
    expect(added).toMatchObject({ sourcesText: draft.sourcesText, macsText: "aa:bb:cc:dd:ee:ff" });
    expect(addKnownClient(added, client)).toBe(added);
    expect(addKnownClient(byMac, { ip: "192.168.1.11" })).toBe(byMac);
  });
  it("annotates host IPs and MACs with returned device names without rewriting networks", () => {
    const clients = [{ ip: "192.168.1.10", name: "电视", mac: "AA:BB:CC:DD:EE:FF" }, { ip: "2001:db8::1", name: "手机" }];
    expect(clientRouteSummary({ ...rule, sources: [...rule.sources, "192.168.1.0/24", "2001:db8::1/128"] }, clients)).toBe("192.168.1.10 (电视) · 192.168.1.0/24 · 2001:db8::1 (手机)");
    expect(clientRouteSummary({ ...rule, match: "mac", macs: ["aa:bb:cc:dd:ee:ff"] }, clients)).toBe("aa:bb:cc:dd:ee:ff (电视)");
  });
  it("counts distinct active allowlist entries while ignoring disabled and unrelated rules", () => {
    expect(admittedClientCount([{ ...rule, enabled: true, admit: true }, { ...rule, id: "duplicate", enabled: true, admit: true }, { ...rule, id: "disabled", admit: true, sources: ["10.0.0.1"] }, { ...rule, id: "mac", enabled: true, admit: true, match: "mac", macs: ["AA-BB-CC-DD-EE-FF", "aa:bb:cc:dd:ee:ff"] }, { ...rule, id: "route", enabled: true, sources: ["10.0.0.2"] }])).toBe(2);
  });
});
