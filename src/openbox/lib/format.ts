export function formatBytes(value = 0, decimals = 1) {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "kB", "MB", "GB", "TB"];
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1000)), units.length - 1);
  const amount = value / 1000 ** index;
  return `${amount.toFixed(index === 0 ? 0 : decimals).replace(/\.0$/, "")} ${units[index]}`;
}

export function formatRate(value = 0) {
  return `${formatBytes(value)}/s`;
}

export function formatDuration(seconds = 0) {
  if (seconds < 60) return `${Math.max(0, Math.floor(seconds))}秒`;
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return hours ? `${hours}小时${minutes ? `${minutes}分钟` : ""}` : `${minutes}分钟`;
}

export function formatDateTime(value?: string) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" }).format(date);
}

export function matchesQuery(values: Array<string | number | undefined>, query: string) {
  const terms = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  if (!terms.length) return true;
  const haystack = values.filter(value => value !== undefined).join(" ").toLocaleLowerCase();
  return terms.every(term => haystack.includes(term));
}

export function latestLatency(history: Array<{ delay: number }> | undefined) {
  if (!history) return null;
  for (let index = history.length - 1; index >= 0; index -= 1) {
    const point = history[index];
    if (point && point.delay > 0) return point.delay;
  }
  return null;
}

export function getCurrentMonth() {
  const date = new Date();
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}`;
}

export function getCurrentDay() {
  const date = new Date();
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}
