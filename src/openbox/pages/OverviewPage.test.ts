import { describe, expect, it } from "vitest";
import { addMonths, createLatestRequestGuard, currentSiteLatency, formatMonthLabel, historyPointPosition, isMonthInHistoryRange, lastDayOfMonth, latencyBarCount } from "./OverviewPage";

describe("overview calendar navigation", () => {
  it("moves across year boundaries", () => {
    expect(addMonths("2026-01", -1)).toBe("2025-12");
    expect(addMonths("2026-12", 1)).toBe("2027-01");
  });

  it("selects the real last day for historical months", () => {
    expect(lastDayOfMonth("2024-02")).toBe("2024-02-29");
    expect(lastDayOfMonth("2026-09")).toBe("2026-09-30");
  });

  it("formats the same compact Chinese month label as the source UI", () => {
    expect(formatMonthLabel("2026-09")).toBe("2026年9月");
  });

  it("keeps month navigation between the current month and twelve months ago", () => {
    expect(isMonthInHistoryRange("2026-09", "2026-09")).toBe(true);
    expect(isMonthInHistoryRange("2025-09", "2026-09")).toBe(true);
    expect(isMonthInHistoryRange("2025-08", "2026-09")).toBe(false);
    expect(isMonthInHistoryRange("2026-10", "2026-09")).toBe(false);
  });

  it("keeps partial realtime history aligned to the right side of a fixed window", () => {
    expect(historyPointPosition(0, 20)).toBeCloseTo((40 / 59) * 100);
    expect(historyPointPosition(19, 20)).toBe(100);
    expect(historyPointPosition(0, 60)).toBe(0);
    expect(historyPointPosition(59, 60)).toBe(100);
  });

  it("rejects a stale drill response after a newer row or dimension request", () => {
    const guard = createLatestRequestGuard();
    const first = guard.issue();
    const second = guard.issue();

    expect(guard.isCurrent(first)).toBe(false);
    expect(guard.isCurrent(second)).toBe(true);

    guard.invalidate();
    expect(guard.isCurrent(second)).toBe(false);
  });

  it("keeps enough responsive latency samples to preserve unreachable markers", () => {
    expect(latencyBarCount(0)).toBe(10);
    expect(latencyBarCount(280)).toBe(35);
    expect(latencyBarCount(800)).toBe(60);
  });

  it("uses the latest site sample so an unreachable result is not hidden by an older success", () => {
    expect(currentSiteLatency([])).toBeUndefined();
    expect(currentSiteLatency([{ time: "first", delay: 188 }])).toBe(188);
    expect(currentSiteLatency([{ time: "first", delay: 188 }, { time: "latest", delay: 0 }])).toBeNull();
  });
});
