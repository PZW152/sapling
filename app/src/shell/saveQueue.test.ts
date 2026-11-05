import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createSaveQueue } from "./saveQueue";

describe("createSaveQueue", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("只保存同一個 key 最後一次的改動", async () => {
    const saved: string[] = [];
    const queue = createSaveQueue(100, () => {});

    queue.schedule("config", async () => saved.push("舊"));
    queue.schedule("config", async () => saved.push("新"));
    await vi.advanceTimersByTimeAsync(100);

    expect(saved).toEqual(["新"]);
  });

  it("不同 key 互不取消", async () => {
    const saved: string[] = [];
    const queue = createSaveQueue(100, () => {});

    queue.schedule("instance:1", async () => saved.push("實例"));
    queue.schedule("config", async () => saved.push("設定"));
    await vi.advanceTimersByTimeAsync(100);

    expect(saved).toEqual(["實例", "設定"]);
  });

  it("flush 立刻保存，之後不會重複保存", async () => {
    const save = vi.fn(async () => {});
    const queue = createSaveQueue(100, () => {});

    queue.schedule("config", save);
    await queue.flush();
    await vi.advanceTimersByTimeAsync(100);

    expect(save).toHaveBeenCalledTimes(1);
  });

  it("回報失敗，下次成功時清除", async () => {
    const results: string[] = [];
    const queue = createSaveQueue(100, (error) => results.push(error));

    queue.schedule("config", () => Promise.reject("硬碟已滿"));
    await queue.flush();
    queue.schedule("config", async () => {});
    await queue.flush();

    expect(results).toEqual(["硬碟已滿", ""]);
  });
});
