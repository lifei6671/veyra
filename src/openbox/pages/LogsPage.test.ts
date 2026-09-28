import { describe, expect, it } from "vitest";
import { getLogCategory, normalizeLogQuery } from "./LogsPage";

describe("log filtering helpers", () => {
  it("extracts the same filter category shown by bracketed controller logs", () => {
    expect(getLogCategory("[TCP] dial example.com:443 failed")).toBe("dial example.com:");
    expect(getLogCategory("dns query example.com")).toBe("dns");
  });

  it("normalizes a URL or host into a regex-ready hostname", () => {
    expect(normalizeLogQuery(" HTTPS://Example.COM:8443/path?q=1 ")).toBe("example.com");
    expect(normalizeLogQuery("example.com:443")).toBe("example.com");
  });
});
