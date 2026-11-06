import { describe, expect, it } from "vitest";
import { clearFinished, isDone, isRunning, trackDownload } from "./downloads";
import type { DownloadTask } from "./downloads";

const task = (result: string): DownloadTask => ({ id: 1, name: "a", result });

describe("download tasks", () => {
  it("分得出進行中、完成和失敗", () => {
    expect(isRunning(task(""))).toBe(true);
    expect(isDone(task("done"))).toBe(true);
    expect(isDone(task("網路錯誤"))).toBe(false);
    expect(isRunning(task("網路錯誤"))).toBe(false);
  });

  it("回傳工作的結果", async () => {
    await expect(trackDownload("a", async () => 5)).resolves.toBe(5);
    clearFinished();
  });

  it("失敗時把錯誤往外丟", async () => {
    await expect(
      trackDownload("a", async () => {
        throw new Error("壞了");
      }),
    ).rejects.toThrow("壞了");
    clearFinished();
  });
});
