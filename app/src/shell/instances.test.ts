import { describe, expect, it } from "vitest";
import { matchesInstance, recentFirst } from "./instances";
import { formatPlaytime } from "./time";
import type { Instance } from "./types";

const instance = (id: string, lastPlayedMs: number) =>
  ({ id, name: `生存 ${id}`, version: "26.3", lastPlayedMs }) as Instance;

describe("recentFirst", () => {
  it("最近玩過的排前面，沒玩過的維持順序", () => {
    const sorted = recentFirst([instance("a", 0), instance("b", 5), instance("c", 0), instance("d", 9)]);
    expect(sorted.map(({ id }) => id)).toEqual(["d", "b", "a", "c"]);
  });
});

describe("matchesInstance", () => {
  it("名稱和版本都搜得到", () => {
    expect(matchesInstance(instance("a", 0), "生存")).toBe(true);
    expect(matchesInstance(instance("a", 0), "26.3")).toBe(true);
    expect(matchesInstance(instance("a", 0), "創造")).toBe(false);
  });
});

describe("formatPlaytime", () => {
  it("短時間不顯示，長時間換成小時", () => {
    expect(formatPlaytime(30)).toBeNull();
    expect(formatPlaytime(600)).toBe("玩了 10 分鐘");
    expect(formatPlaytime(5400)).toBe("玩了 1.5 小時");
  });
});
