import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { SettingsVisualHarness } from "./SettingsVisualHarness";

describe("SettingsVisualHarness", () => {
  it("labels its content as development-only visual fixtures", () => {
    const markup = renderToStaticMarkup(createElement(SettingsVisualHarness));
    expect(markup).toContain("仅开发环境，不代表产品设置");
    expect(markup).toContain("SettingRow 样本");
    expect(markup).toContain("带补充说明的行");
    expect(markup.match(/role="switch"/g)).toHaveLength(3);
    expect(markup).toContain("Switch OFF");
    expect(markup).toContain("Switch ON");
    expect(markup).toContain("Switch Disabled");
    expect(markup).toContain("Select visual fixture");
    expect(markup).toContain("Input visual fixture");
    expect(markup).toContain("Chevron Row");
    expect(markup).toContain("IconButton visual fixture");
    expect(markup).toContain("正在处理…");
    expect(markup).toContain("Visual fixture notice");
    expect(markup).not.toContain("运行状态");
    expect(markup).not.toContain("配置修订");
  });

  it("can render the dialog fixture without product settings", () => {
    const markup = renderToStaticMarkup(createElement(SettingsVisualHarness, { initialDialogOpen: true }));
    expect(markup).toContain("role=\"dialog\"");
    expect(markup).toContain("Dialog visual fixture");
    expect(markup).toContain("不映射任何产品设置");
  });
});
