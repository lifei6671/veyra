import { describe, expect, it } from "vitest";
import { settingsSections } from "./SettingsPage";

describe("settings navigation contract", () => {
  it("matches the nine source menu entries in order", () => {
    expect(settingsSections.map(section => section.label)).toEqual([
      "面板设置",
      "订阅管理",
      "出站节点",
      "目标分流",
      "终端分流",
      "链式代理",
      "共享网络",
      "DNS 设置",
      "后端设置",
    ]);
  });
});
