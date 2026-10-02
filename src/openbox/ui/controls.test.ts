import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { SelectControl } from "./controls";

describe("OpenBox shadcn Select", () => {
  it("renders the label for an empty-valued filter instead of an empty trigger", () => {
    const html = renderToStaticMarkup(createElement(SelectControl, {
      label: "节点过滤", value: "", options: [{ value: "", label: "全部" }, { value: "kind:node", label: "全部节点" }],
    }));
    expect(html).toContain('role="combobox"');
    expect(html).toContain('aria-label="节点过滤"');
    expect(html).toContain('>全部</span>');
    expect(html).not.toContain("__openbox_empty__");
  });

  it("displays the stored option label and respects disabled controls", () => {
    const html = renderToStaticMarkup(createElement(SelectControl, {
      label: "更新时间", value: "6", disabled: true, options: [{ value: "6", label: "06:00" }],
    }));
    expect(html).toContain('>06:00</span>');
    expect(html).toContain('disabled=""');
  });

  it("retains the selected log label when its dynamic category disappears", () => {
    const html = renderToStaticMarkup(createElement(SelectControl, {
      label: "日志类型", value: "type:dns", placeholder: "dns", options: [{ value: "all", label: "全部" }],
    }));
    expect(html).toContain('>dns</span>');
    expect(html).not.toContain('>type:dns</span>');
  });
});
