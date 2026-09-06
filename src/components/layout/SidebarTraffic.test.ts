import { describe, expect, it } from "vitest";
import { countSidebarTrafficSegments, formatSidebarMemory, selectSidebarTrafficPoints, splitSidebarTrafficRate } from "./SidebarTraffic";

describe("SidebarTraffic", () => {
  it("keeps only the latest 30 real samples inside the 60 second window", () => {
    const history = Array.from({ length: 40 }, (_, index) => ({
      sampledAtMs: 61000 + index * 1000,
      uploadRateBps: index,
      downloadRateBps: index * 2,
    }));
    const points = selectSidebarTrafficPoints(history, 100000);
    expect(points).toHaveLength(30);
    expect(points[0].sampledAtMs).toBe(71000);
    expect(points.at(-1)?.sampledAtMs).toBe(100000);
  });

  it("separates the numeric rate from its unit", () => {
    expect(splitSidebarTrafficRate(0)).toEqual({ value: "0.00", unit: "B/s" });
    expect(splitSidebarTrafficRate(1536)).toEqual({ value: "1.50", unit: "KB/s" });
  });

  it("keeps a real sampling gap as separate graph segments", () => {
    expect(countSidebarTrafficSegments([
      { sampledAtMs: 1000, uploadRateBps: 1, downloadRateBps: 2 },
      { sampledAtMs: 2000, uploadRateBps: 2, downloadRateBps: 3 },
      { sampledAtMs: 8000, uploadRateBps: 3, downloadRateBps: 4 },
    ])).toBe(2);
  });

  it("formats owned core working-set bytes as fixed megabytes", () => {
    expect(formatSidebarMemory(0)).toEqual({ value: "0.00", unit: "MB" });
    expect(formatSidebarMemory(52428800)).toEqual({ value: "50.00", unit: "MB" });
  });
});
