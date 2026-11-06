export type LogLevel = "error" | "warn" | "info";

const ERROR_MARKERS = ["/ERROR]", "/FATAL]", "Exception", "Caused by:", "\tat "];
const WARN_MARKERS = ["/WARN]"];

export function lineLevel(line: string): LogLevel {
  if (ERROR_MARKERS.some((marker) => line.includes(marker))) {
    return "error";
  }
  return WARN_MARKERS.some((marker) => line.includes(marker)) ? "warn" : "info";
}

export function matchesLine(line: string, problemsOnly: boolean, search: string) {
  if (problemsOnly && lineLevel(line) === "info") {
    return false;
  }
  return line.toLowerCase().includes(search.trim().toLowerCase());
}

/** 最後幾行。 */
export function tailOf(lines: string[], count: number) {
  return lines.length > count ? lines.slice(-count) : lines;
}
