import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { PanelSettings } from "./PanelSettings";

function render(entries: Record<string, string> = {}) {
  return renderToStaticMarkup(createElement(PanelSettings, {
    storage: { entries }, theme: "light", setTheme: () => {}, onSaved: () => {},
  }));
}

describe("OpenBox panel settings", () => {
  it("renders stored values and custom site icons instead of substituting defaults", () => {
    const html = render({
      "config/global-radius": "9",
      "config/speedtest-timeout": "7200",
      "config/low-latency": "320",
      "config/medium-latency": "950",
      "config/proxy-group-columns": "3",
      "config/test-sites": JSON.stringify([{ id: "google", icon: "brand:github", name: "代码站点", url: "https://github.com/" }]),
    });
    expect(html).toContain("9px");
    expect(html).toContain('value="7200"');
    expect(html).toContain('value="320"');
    expect(html).toContain('value="950"');
    expect(html).toContain('>三列</span>');
    expect(html).toContain('aria-label="代码站点 图标"');
    expect(html).toContain('value="https://github.com/"');
    expect(html).not.toContain('value="https://www.google.com/generate_204"');
  });

  it("uses the original defaults when the backend has no panel settings", () => {
    const html = render();
    expect(html).toContain("16px");
    expect(html).toContain("此API会用于IP检查中全球节点IP信息查询、连接详情中的IP地理信息查询、面板DNS查询中的IP地理信息查询。");
    expect(html.match(/role="combobox"/g)).toHaveLength(4);
    expect(html).toContain("概览里的延时小卡片和规则页右上角的快捷查询共用这四个站点。图标从图标库选;名称空着就用图标的品牌名(没有品牌名就用网址的主机名);网址填 http(s) 地址,延时按内核经当前分流访问这个地址计");
    expect(html).toContain('value="400"');
    expect(html).toContain('value="800"');
    expect(html).toContain('>双列</span>');
    expect(html).toContain('>跟随系统</span>');
  });

  it("keeps source reading order when the two-column grid becomes one column", () => {
    const html = render();
    const labels = ["面板语言", "面板背景", "全局圆角", "主题", "修改密码", "IP信息API", "测速超时", "黄色的阈值", "红色的阈值", "IPv6 测试", "隐藏不可用节点"];
    const positions = labels.map(label => html.indexOf(label));
    expect(positions.every(position => position >= 0)).toBe(true);
    expect(positions).toEqual([...positions].sort((a, b) => a - b));
  });
});
