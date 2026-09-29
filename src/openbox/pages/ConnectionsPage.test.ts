import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { ControllerConnection, StorageResponse } from "../api/types";
import { appendSourceLabel, connectionColumnsForView, connectionPolicyName, ConnectionsPage, defaultConnectionColumns, formatConnectionQuery, groupConnections, groupableConnectionColumns, isDnsHijackConnection, moveConnectionColumn, moveSourceLabel, parseConnectionColumns, parseConnectionGrouping, sortConnections } from "./ConnectionsPage";

const connection = (id: string, host: string, download: number): ControllerConnection => ({
  id,
  chains: ["直连", "其他"],
  download,
  upload: 512,
  metadata: {
    destinationIP: "198.41.192.167",
    destinationPort: "7844",
    dnsMode: "normal",
    host,
    network: "udp",
    processPath: "",
    sourceIP: "192.168.1.10",
    sourcePort: "37387",
    type: "tun/tun-in",
  },
  rule: "final",
  rulePayload: "",
  start: "2026-09-29T10:10:10.943309829+08:00",
});

describe("connections page", () => {
  it("uses the original default column order and rejects invalid storage values", () => {
    expect(parseConnectionColumns(undefined)).toEqual(defaultConnectionColumns);
    expect(parseConnectionColumns('["sourceIP","host","unknown","host"]')).toEqual(["sourceIP", "host"]);
    expect(parseConnectionColumns("not-json")).toEqual(defaultConnectionColumns);
  });

  it("sorts real connection rows using the configured column and direction", () => {
    const rows = [connection("b", "z.example", 2), connection("a", "a.example", 10)];
    expect(sortConnections(rows, "host", "asc").map(item => item.id)).toEqual(["a", "b"]);
    expect(sortConnections(rows, "dl", "desc").map(item => item.id)).toEqual(["a", "b"]);
  });

  it("only accepts enumerable connection columns for grouping", () => {
    expect(parseConnectionGrouping('["sourceIP","dlSpeed","host","sourceIP","unknown"]')).toEqual(["sourceIP", "host"]);
    expect(parseConnectionGrouping("not-json")).toEqual([]);
    expect(groupableConnectionColumns).not.toContain("connectTime");
  });

  it("formats connection queries like the original toolbar action", () => {
    expect(formatConnectionQuery(" HTTPS://Example.COM:443/path?q=1 ")).toBe("example.com");
    expect(formatConnectionQuery("百度.com:7844/path")).toBe("xn--wxtr44c.com");
    expect(formatConnectionQuery("192.168.1.10:7890")).toBe("192.168.1.10");
    expect(formatConnectionQuery(" ")).toBe("");
  });

  it("moves grouped columns first and hides close while active grouping is enabled", () => {
    const columns = ["close", "outbound", "sourceIP", "dlSpeed", "host", "connectTime"] as const;
    expect(connectionColumnsForView([...columns], "active", ["host", "sourceIP"]))
      .toEqual(["sourceIP", "host", "outbound", "dlSpeed", "connectTime"]);
    expect(connectionColumnsForView([...columns], "closed", ["host", "sourceIP"]))
      .toEqual(["sourceIP", "host", "outbound", "dlSpeed", "connectTime"]);
    expect(connectionColumnsForView([...columns], "active", []))
      .toEqual(columns);
  });

  it("aggregates rows by one or more enumerable columns", () => {
    const rows = [connection("a", "a.example", 1), connection("b", "a.example", 2), connection("c", "b.example", 3)];
    rows[2].metadata.sourceIP = "192.168.1.11";
    const groupedByHost = groupConnections(rows, ["host"]);
    expect(groupedByHost).toMatchObject([
      { values: { host: "a.example:7844" }, count: 2 },
      { values: { host: "b.example:7844" }, count: 1 },
    ]);
    expect(groupedByHost[0].connections.map(item => item.id)).toEqual(["a", "b"]);
    expect(groupedByHost[1].connections.map(item => item.id)).toEqual(["c"]);
    expect(groupConnections(rows, ["host", "sourceIP"])).toMatchObject([
      { values: { host: "a.example:7844", sourceIP: "192.168.1.10" }, count: 2 },
      { values: { host: "b.example:7844", sourceIP: "192.168.1.11" }, count: 1 },
    ]);

    const fallbackHost = connection("fallback", "", 4);
    fallbackHost.metadata.destinationIP = "a.example";
    const hostGroups = groupConnections([...rows.slice(0, 2), fallbackHost], ["host"]);
    expect(hostGroups).toHaveLength(2);
    expect(hostGroups.map(group => group.values.host)).toEqual(["a.example:7844", "a.example:7844"]);
  });

  it("moves columns in both directions and preserves the dropped order", () => {
    const movedRight = moveConnectionColumn(["close", "sourceIP", "host"], ["type", "process"], "sourceIP", "inactive", 1);
    expect(movedRight).toEqual({ active: ["close", "host"], inactive: ["type", "sourceIP", "process"] });
    const movedLeft = moveConnectionColumn(movedRight.active, movedRight.inactive, "process", "active", 1);
    expect(movedLeft).toEqual({ active: ["close", "process", "host"], inactive: ["type", "sourceIP"] });
    expect(moveConnectionColumn(["close", "sourceIP", "chains", "host"], [], "host", "active", 2).active).toEqual(["close", "sourceIP", "host", "chains"]);
  });

  it("adds a trimmed terminal label only when both fields are present", () => {
    expect(appendSourceLabel([], " 192.168.1.10 ", " 测试 ")).toEqual([{ cidr: "192.168.1.10", name: "测试" }]);
    const labels = [{ cidr: "192.168.1.10", name: "测试" }];
    expect(appendSourceLabel(labels, "0", "新增")).toEqual([...labels, { cidr: "0", name: "新增" }]);
    expect(appendSourceLabel(labels, "", "缺少地址")).toBe(labels);
  });

  it("moves terminal labels to the exact placeholder index", () => {
    const first = { cidr: "192.168.1.10", name: "测试" };
    const second = { cidr: "0", name: "0" };
    expect(moveSourceLabel([first, second], 0, 1)).toEqual([second, first]);
    expect(moveSourceLabel([first, second], 1, 0)).toEqual([second, first]);
  });

  it("only identifies the hostless local DNS interception rows", () => {
    const dns = connection("dns", "", 0);
    dns.metadata.destinationIP = "127.0.0.1";
    dns.metadata.destinationPort = "53";
    expect(isDnsHijackConnection(dns)).toBe(true);
    dns.metadata.host = "dns.example";
    expect(isDnsHijackConnection(dns)).toBe(false);
  });

  it("uses the outermost connection chain as the related policy", () => {
    expect(connectionPolicyName(connection("active", "quic.cftunnel.com", 0))).toBe("其他");
    const empty = connection("empty", "example.com", 0);
    empty.chains = [];
    expect(connectionPolicyName(empty)).toBe("");
  });

  it("renders the flat original toolbar and reverses the controller chain into route order", () => {
    const storage: StorageResponse = { entries: {
      "config/connection-table-columns": JSON.stringify(defaultConnectionColumns),
      "config/connection-sort-type": "host",
      "config/connection-sort-direction": "asc",
      "config/hide-dns-connections": "true",
      "config/source-ip-label-list": "[]",
    } };
    const markup = renderToStaticMarkup(createElement(ConnectionsPage, {
      frame: { connections: [connection("active", "quic.cftunnel.com", 12719)], closedConnections: [] },
      storage,
      onPatchStorage: async () => undefined,
      onToast: () => undefined,
    }));
    expect(markup).toContain("活跃 (1)");
    expect(markup).toContain("连接设置");
    expect(markup).toContain("隐藏连接");
    expect(markup).toContain("M13.19 8.688");
    expect(markup).toContain("关闭全部连接");
    expect(markup).toContain("格式化查询");
    expect(markup).toMatch(/其他[\s\S]*直连/);
    expect(markup).not.toContain("复制当前页面链接");
    expect(markup).not.toContain("数据来自实时 WebSocket");

    storage.entries["config/table-grouping"] = JSON.stringify(["sourceIP"]);
    const groupedMarkup = renderToStaticMarkup(createElement(ConnectionsPage, {
      frame: { connections: [connection("active", "quic.cftunnel.com", 12719)], closedConnections: [] },
      storage,
      onPatchStorage: async () => undefined,
      onToast: () => undefined,
    }));
    expect(groupedMarkup).toContain("取消按源IP聚合");
    expect(groupedMarkup).toContain("192.168.1.10");
    expect(groupedMarkup).toContain("(1)");
    expect(groupedMarkup).not.toContain('<span role="columnheader">关闭</span>');

    storage.entries["config/quick-filter-enabled"] = "true";
    const hiddenMarkup = renderToStaticMarkup(createElement(ConnectionsPage, {
      frame: { connections: [connection("active", "quic.cftunnel.com", 12719)], closedConnections: [] },
      storage,
      onPatchStorage: async () => undefined,
      onToast: () => undefined,
    }));
    expect(hiddenMarkup).toContain("显示连接");
    expect(hiddenMarkup).toContain("M13.181 8.68");
  });
});
