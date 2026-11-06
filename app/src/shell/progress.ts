import { useRef } from "react";
import type { Progress } from "./types";

/** 遊戲檔案佔整體進度的比例，其餘是 Java。 */
const GAME_SHARE = 0.85;

export function overallProgress(progress: Progress | null) {
  if (!progress) {
    return 0;
  }
  const part = progress.finishedBytes / Math.max(progress.totalBytes, 1);
  return progress.stage === "game" ? part * GAME_SHARE : GAME_SHARE + part * (1 - GAME_SHARE);
}

/** 速度的計算區間。太短數字會一直跳。 */
const SPEED_WINDOW_MS = 1000;
const BYTES_PER_MB = 1024 * 1024;

export type SpeedSample = { time: number; bytes: number; speed: number };

/** 用這次和上次的下載量算速度。區間太短就沿用舊值。 */
export function nextSpeed(previous: SpeedSample | null, time: number, bytes: number): SpeedSample {
  if (!previous || bytes < previous.bytes) {
    return { time, bytes, speed: 0 };
  }
  const elapsed = time - previous.time;
  if (elapsed < SPEED_WINDOW_MS) {
    return previous;
  }
  return { time, bytes, speed: ((bytes - previous.bytes) / elapsed) * 1000 };
}

export function formatSpeed(bytesPerSecond: number) {
  return `${(bytesPerSecond / BYTES_PER_MB).toFixed(1)} MB/s`;
}

/** 下載速度，每秒 byte 數。沒有在下載時是 0。 */
export function useSpeed(progress: Progress | null) {
  const sample = useRef<SpeedSample | null>(null);
  if (progress) {
    sample.current = nextSpeed(sample.current, performance.now(), progress.downloadedBytes);
  } else {
    sample.current = null;
  }
  return sample.current?.speed ?? 0;
}

export function toMegabytes(bytes: number) {
  return Math.round(bytes / 1024 / 1024);
}
