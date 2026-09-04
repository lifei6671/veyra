import type { TrafficSample } from "./observability";

export function formatTraffic(value: number, rate = false): string {
  const units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
  const index = value === 0 ? 0 : Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  const unitIndex = Math.max(0, index);
  return `${(value / 1024 ** unitIndex).toLocaleString("en-US", { maximumFractionDigits: unitIndex === 0 ? 0 : 1 })} ${units[unitIndex]}${rate ? "/s" : ""}`;
}

export function trafficTrend(history: ReadonlyArray<TrafficSample>, nowMs: number, windowMs: 60000 | 600000) {
  const points = history.filter((point) => point.sampledAtMs <= nowMs && nowMs - point.sampledAtMs <= windowMs);
  const maximum = Math.max(1, ...points.flatMap((point) => [point.uploadRateBps, point.downloadRateBps]));
  const coordinates = points.map((point) => ({
    x: 600 * (1 - (nowMs - point.sampledAtMs) / windowMs),
    uploadY: 156 - point.uploadRateBps / maximum * 152,
    downloadY: 156 - point.downloadRateBps / maximum * 152,
  }));
  const path = (direction: "uploadY" | "downloadY") => coordinates.map((point, index) => {
    const disconnected = index === 0 || points[index].sampledAtMs - points[index - 1].sampledAtMs > 5000;
    return `${disconnected ? "M" : "L"}${point.x.toFixed(2)},${point[direction].toFixed(2)}`;
  }).join(" ");
  const isolated = coordinates.filter((_, index) =>
    (index === 0 || points[index].sampledAtMs - points[index - 1].sampledAtMs > 5000) &&
    (index === points.length - 1 || points[index + 1].sampledAtMs - points[index].sampledAtMs > 5000));
  return { points, coordinates, isolated, maximum, uploadPath: path("uploadY"), downloadPath: path("downloadY") };
}
