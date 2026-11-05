import { describe, expect, it } from "vitest";
import { overallProgress, toMegabytes } from "./progress";
import type { Progress } from "./types";

function progress(stage: Progress["stage"], finishedBytes: number, totalBytes: number): Progress {
  return { stage, finishedFiles: 0, downloadedFiles: 0, totalFiles: 0, finishedBytes, totalBytes };
}

describe("overallProgress", () => {
  it("還沒開始時是 0", () => {
    expect(overallProgress(null)).toBe(0);
  });

  it("遊戲檔案完成後進入 Java 階段，進度不會倒退", () => {
    const gameDone = overallProgress(progress("game", 10, 10));
    const javaStart = overallProgress(progress("java", 0, 10));

    expect(javaStart).toBe(gameDone);
    expect(overallProgress(progress("java", 10, 10))).toBeCloseTo(1);
  });

  it("總大小是 0 時不會出錯", () => {
    expect(overallProgress(progress("game", 0, 0))).toBe(0);
  });
});

describe("toMegabytes", () => {
  it("四捨五入到整數 MB", () => {
    expect(toMegabytes(1.6 * 1024 * 1024)).toBe(2);
  });
});
