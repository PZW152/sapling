import { describe, expect, it } from "vitest";
import { formatSpeed, nextSpeed } from "./progress";

describe("nextSpeed", () => {
  it("第一次沒有速度", () => {
    expect(nextSpeed(null, 0, 500).speed).toBe(0);
  });

  it("滿一秒才更新", () => {
    const first = nextSpeed(null, 0, 0);
    expect(nextSpeed(first, 500, 100)).toBe(first);
    expect(nextSpeed(first, 2000, 1000).speed).toBe(500);
  });

  it("換一批下載時重新計算", () => {
    expect(nextSpeed({ time: 0, bytes: 900, speed: 5 }, 3000, 10).speed).toBe(0);
  });
});

describe("formatSpeed", () => {
  it("用 MB/s 表示", () => {
    expect(formatSpeed(1572864)).toBe("1.5 MB/s");
  });
});
