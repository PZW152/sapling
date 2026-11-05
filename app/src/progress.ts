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

export function toMegabytes(bytes: number) {
  return Math.round(bytes / 1024 / 1024);
}
