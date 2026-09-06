import { memo, useEffect, useMemo, useRef } from "react";
import type { RuntimeObservation, TrafficSample } from "../../lib/observability";
import "./SidebarTraffic.css";

const MAX_POINTS = 30;
const GAP_MS = 5000;
const FRAME_INTERVAL_MS = 1000 / 15;
const ANIMATION_DURATION_MS = 1000;

type SidebarTrafficProps = {
  observation: RuntimeObservation | null;
  unavailable: boolean;
};

type RateParts = { value: string; unit: string };

export function selectSidebarTrafficPoints(
  history: ReadonlyArray<TrafficSample>,
  nowMs: number,
): ReadonlyArray<TrafficSample> {
  return history
    .filter((point) => point.sampledAtMs <= nowMs && nowMs - point.sampledAtMs <= 60000)
    .slice(-MAX_POINTS);
}

export function splitSidebarTrafficRate(value: number): RateParts {
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  const exponent = value < 1 ? 0 : Math.min(Math.floor(Math.log2(value) / 10), units.length - 1);
  const scaled = value / 1024 ** exponent;
  return { value: scaled >= 1000 ? scaled.toFixed(0) : scaled.toPrecision(3), unit: `${units[exponent]}/s` };
}

export function formatSidebarMemory(value: number): RateParts {
  return { value: (value / 1024 ** 2).toFixed(2), unit: "MB" };
}

export function countSidebarTrafficSegments(points: ReadonlyArray<TrafficSample>): number {
  return points.reduce((count, point, index) =>
    count + (index === 0 || point.sampledAtMs - points[index - 1].sampledAtMs > GAP_MS ? 1 : 0), 0);
}

export const SidebarTraffic = memo(function SidebarTraffic({ observation, unavailable }: SidebarTrafficProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const latestSampleRef = useRef<number | null>(null);
  const ready = !unavailable && observation?.sidecarLifecycle === "ready";
  const points = useMemo(
    () => ready ? selectSidebarTrafficPoints(observation.trafficHistory, observation.observedAtMs) : [],
    [ready, observation?.trafficHistory, observation?.observedAtMs],
  );
  const pointsSignature = points.map((point) =>
    `${point.sampledAtMs}:${point.uploadRateBps}:${point.downloadRateBps}`).join("|");
  const upload = splitSidebarTrafficRate(ready ? observation.uploadRateBps : 0);
  const download = splitSidebarTrafficRate(ready ? observation.downloadRateBps : 0);
  const memory = observation?.sidecarLifecycle === "stopped" ? formatSidebarMemory(0) :
    ready && observation.coreMemoryBytes !== null ? formatSidebarMemory(observation.coreMemoryBytes) :
      { value: "—", unit: "MB" };

  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas === null) return;

    let animationFrame = 0;
    let frameTimer: number | null = null;
    let resizeObserver: ResizeObserver | null = null;
    let lastFrameAt = 0;
    const latestSample = points.at(-1)?.sampledAtMs ?? null;
    const animate = latestSample !== null && latestSampleRef.current !== null && latestSample !== latestSampleRef.current;
    latestSampleRef.current = latestSample;

    const cancelDraw = () => {
      if (frameTimer !== null) window.clearTimeout(frameTimer);
      if (animationFrame !== 0) window.cancelAnimationFrame(animationFrame);
      frameTimer = null;
      animationFrame = 0;
    };

    const draw = (offsetProgress: number) => {
      const bounds = canvas.getBoundingClientRect();
      const width = Math.max(1, bounds.width);
      const height = Math.max(1, bounds.height);
      const pixelRatio = window.devicePixelRatio || 1;
      const targetWidth = Math.round(width * pixelRatio);
      const targetHeight = Math.round(height * pixelRatio);
      if (canvas.width !== targetWidth || canvas.height !== targetHeight) {
        canvas.width = targetWidth;
        canvas.height = targetHeight;
      }
      const context = canvas.getContext("2d");
      if (context === null) return;
      context.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
      context.clearRect(0, 0, width, height);

      const styles = getComputedStyle(canvas);
      context.strokeStyle = styles.getPropertyValue("--sidebar-traffic-grid").trim();
      context.globalAlpha = 1;
      context.lineWidth = 1;
      context.beginPath();
      context.moveTo(0, height / 7);
      context.lineTo(width, height / 7);
      context.moveTo(0, height * 4 / 7);
      context.lineTo(width, height * 4 / 7);
      context.stroke();

      if (points.length === 0) {
        context.strokeStyle = styles.getPropertyValue("--traffic-download").trim();
        context.globalAlpha = 1;
        context.lineWidth = 4;
        context.beginPath();
        context.moveTo(0, height - 1);
        context.lineTo(width, height - 1);
        context.stroke();
        return;
      }
      const step = width / MAX_POINTS;
      const scrollOffset = animate ? step * (1 - offsetProgress) : 0;
      const xFor = (index: number) => (MAX_POINTS - points.length + index + 0.5) * step + scrollOffset;
      const yFor = (value: number) => {
        const band = height / 7;
        if (value === 0) return height - 1;
        if (value <= 10) return height - value / 10 * band;
        if (value <= 100) return height - (value / 100 + 1) * band;
        if (value <= 1024) return height - (value / 1024 + 2) * band;
        if (value <= 10240) return height - (value / 10240 + 3) * band;
        if (value <= 102400) return height - (value / 102400 + 4) * band;
        if (value <= 1048576) return height - (value / 1048576 + 5) * band;
        if (value <= 10485760) return height - (value / 10485760 + 6) * band;
        return 1;
      };

      const drawSeries = (key: "uploadRateBps" | "downloadRateBps", color: string, alpha: number) => {
        context.strokeStyle = color;
        context.globalAlpha = alpha;
        context.lineWidth = 4;
        context.lineCap = "round";
        context.lineJoin = "round";
        context.beginPath();
        let segmentStart = 0;
        while (segmentStart < points.length) {
          let segmentEnd = segmentStart;
          while (segmentEnd + 1 < points.length && points[segmentEnd + 1].sampledAtMs - points[segmentEnd].sampledAtMs <= GAP_MS) {
            segmentEnd += 1;
          }
          context.moveTo(xFor(segmentStart), yFor(points[segmentStart][key]));
          for (let index = segmentStart + 1; index <= segmentEnd; index += 1) {
            const previousX = xFor(index - 1);
            const previousY = yFor(points[index - 1][key]);
            const currentX = xFor(index);
            const currentY = yFor(points[index][key]);
            context.quadraticCurveTo(previousX, previousY, (previousX + currentX) / 2, (previousY + currentY) / 2);
          }
          if (segmentEnd > segmentStart) {
            context.lineTo(xFor(segmentEnd), yFor(points[segmentEnd][key]));
          }
          if (segmentEnd === segmentStart) {
            context.lineTo(xFor(segmentStart) + 0.01, yFor(points[segmentStart][key]));
          }
          segmentStart = segmentEnd + 1;
        }
        context.stroke();
      };

      drawSeries("uploadRateBps", styles.getPropertyValue("--traffic-upload").trim(), 0.6);
      drawSeries("downloadRateBps", styles.getPropertyValue("--traffic-download").trim(), 1);
      context.globalAlpha = 1;
    };

    const startedAt = performance.now();
    const drawAnimatedFrame = (timestamp: number) => {
      animationFrame = 0;
      const elapsedSinceFrame = timestamp - lastFrameAt;
      if (elapsedSinceFrame < FRAME_INTERVAL_MS) {
        frameTimer = window.setTimeout(() => {
          frameTimer = null;
          animationFrame = window.requestAnimationFrame(drawAnimatedFrame);
        }, FRAME_INTERVAL_MS - elapsedSinceFrame);
        return;
      }
      lastFrameAt = timestamp;
      const progress = Math.min((timestamp - startedAt) / ANIMATION_DURATION_MS, 1);
      draw(progress);
      if (progress < 1) animationFrame = window.requestAnimationFrame(drawAnimatedFrame);
    };

    draw(animate ? 0 : 1);
    if (animate) animationFrame = window.requestAnimationFrame(drawAnimatedFrame);
    if (typeof ResizeObserver !== "undefined") {
      resizeObserver = new ResizeObserver(() => draw(1));
      resizeObserver.observe(canvas);
    }
    const theme = window.matchMedia("(prefers-color-scheme: dark)");
    const redrawTheme = () => draw(1);
    theme.addEventListener("change", redrawTheme);
    return () => {
      cancelDraw();
      resizeObserver?.disconnect();
      theme.removeEventListener("change", redrawTheme);
    };
  }, [pointsSignature]);

  return <div className="sidebar-traffic-content">
    <div className="sidebar-traffic-graph" title="最近 30 个采样点；采样缺口保持断开">
      <canvas
        ref={canvasRef}
        aria-label="最近 30 个上下行网速采样，浅橙色为上传，蓝色为下载"
        data-point-count={points.length}
        data-segment-count={countSidebarTrafficSegments(points)}
      />
    </div>
    <div className="sidebar-traffic-rates" aria-label="即时网速">
      <RateRow direction="upload" label="上传速度" rate={upload} />
      <RateRow direction="download" label="下载速度" rate={download} />
      <RateRow direction="memory" label="内核内存" rate={memory} />
    </div>
  </div>;
});

function RateRow({ direction, label, rate }: { direction: "upload" | "download" | "memory"; label: string; rate: RateParts }) {
  const iconPath = direction === "upload" ? "M12 19V5m0 0-5 5m5-5 5 5" :
    direction === "download" ? "M12 5v14m0 0-5-5m5 5 5-5" :
      "M9 3v3m6-3v3m-6 12v3m6-3v3M3 9h3m-3 6h3m12-6h3m-3 6h3M7 6h10a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1Zm3 4h4v4h-4v-4Z";
  return <div className={`sidebar-rate sidebar-rate-${direction}`} title={label}>
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d={iconPath} /></svg>
    <span className="sidebar-rate-value">{rate.value}</span>
    <span className="sidebar-rate-unit">{rate.unit}</span>
  </div>;
}
