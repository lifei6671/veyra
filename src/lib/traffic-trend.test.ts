import { describe, expect, it } from "vitest";
import { formatTraffic, trafficTrend } from "./traffic-trend";

describe("内存流量趋势显示", () => {
  it("同一历史裁剪十分钟和六十秒，较早点只影响主图纵轴", () => {
    const history = [
      { sampledAtMs: 0, uploadRateBps: 1024, downloadRateBps: 2048 },
      { sampledAtMs: 540000, uploadRateBps: 10, downloadRateBps: 20 },
      { sampledAtMs: 600000, uploadRateBps: 20, downloadRateBps: 40 },
    ];
    const main = trafficTrend(history, 600000, 600000);
    const compact = trafficTrend(history, 600000, 60000);
    expect(main.points).toHaveLength(3);
    expect(compact.points.map((point) => point.sampledAtMs)).toEqual([540000, 600000]);
    expect(main.maximum).toBe(2048);
    expect(compact.maximum).toBe(40);
    expect(main.coordinates[1].x).toBe(540);
    expect(compact.coordinates[0].x).toBe(0);
    expect(trafficTrend(history, 1200001, 600000).points).toHaveLength(0);
  });

  it("连续的 600 个采样点不渲染装饰圆，断档两侧的孤立点仍可见", () => {
    const continuous = Array.from({ length: 600 }, (_, index) => ({ sampledAtMs: index * 1000, uploadRateBps: 0, downloadRateBps: 10 }));
    expect(trafficTrend(continuous, 600000, 600000).isolated).toHaveLength(0);
    expect(trafficTrend([continuous[0], continuous[8]], 600000, 600000).isolated).toHaveLength(2);
    expect(trafficTrend([continuous[0]], 600000, 600000).isolated).toHaveLength(1);
  });
  it("按真实采样间隔定位，共用上下行最大值，超过 5 秒保持断线", () => {
    const trend = trafficTrend([
      { sampledAtMs: 0, uploadRateBps: 50, downloadRateBps: 100 },
      { sampledAtMs: 5000, uploadRateBps: 0, downloadRateBps: 50 },
      { sampledAtMs: 10001, uploadRateBps: 25, downloadRateBps: 0 },
    ], 60000, 60000);
    expect(trend.maximum).toBe(100);
    expect(trend.uploadPath).toBe("M0.00,80.00 L50.00,156.00 M100.01,118.00");
    expect(trend.downloadPath).toBe("M0.00,4.00 L50.00,80.00 M100.01,156.00");
  });

  it("显示时钟前进只淘汰过期点，不制造零值点或修改后端窗口", () => {
    const history = [{ sampledAtMs: 1000, uploadRateBps: 0, downloadRateBps: 0 }];
    const first = trafficTrend(history, 1000, 60000);
    expect(first.coordinates).toEqual([{ x: 600, uploadY: 156, downloadY: 156 }]);
    expect(first.maximum).toBe(1);
    expect(trafficTrend(history, 61000, 60000).points).toHaveLength(1);
    expect(trafficTrend(history, 61001, 60000).points).toEqual([]);
    expect(history).toEqual([{ sampledAtMs: 1000, uploadRateBps: 0, downloadRateBps: 0 }]);
  });

  it("空窗口无路径，单个点保留可绘制坐标", () => {
    expect(trafficTrend([], 0, 60000).uploadPath).toBe("");
    expect(trafficTrend([{ sampledAtMs: 0, uploadRateBps: 1, downloadRateBps: 2 }], 0, 60000).coordinates).toHaveLength(1);
  });

  it.each([
    [0, true, "0 B/s"], [1023, true, "1,023 B/s"], [1024, true, "1 KiB/s"],
    [1536, true, "1.5 KiB/s"], [1048576, true, "1 MiB/s"], [1073741824, false, "1 GiB"],
  ])("格式化 %s 的速率或累计单位", (value, rate, expected) => {
    expect(formatTraffic(value, rate)).toBe(expected);
  });
});
