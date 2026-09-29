import { describe, expect, it } from "vitest";
import { compileLogSearch, getLogCategory, normalizeLogQuery } from "./LogsPage";

describe("log filtering helpers", () => {
  it("extracts the same filter category shown by bracketed controller logs", () => {
    expect(getLogCategory("[TCP] dial example.com:443 failed")).toBe("dial example.com:");
    expect(getLogCategory("dns query example.com")).toBe("dns");
  });

  it("compiles the search text as a regular expression", () => {
    const regularExpression = compileLogSearch("example\\.(com|net)");
    expect(regularExpression.invalid).toBe(false);
    expect(regularExpression.regex?.test("dial example.net:443")).toBe(true);
  });

  it("reports invalid regular expressions", () => {
    expect(compileLogSearch("[").invalid).toBe(true);
  });

  it("formats a URL or host like the reference interface", () => {
    expect(normalizeLogQuery(" HTTPS://Example.COM:8443/path?q=1 ")).toBe("example.com");
    expect(normalizeLogQuery("example.com:443")).toBe("example.com");
  });
});
