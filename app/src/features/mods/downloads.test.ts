import { describe, expect, it } from "vitest";
import { formatDownloads } from "./downloads";

describe("formatDownloads", () => {
  it("用萬和億表示大數字", () => {
    expect(formatDownloads(999)).toBe("999");
    expect(formatDownloads(25_000)).toBe("2 萬");
    expect(formatDownloads(248_980_957)).toBe("2.5 億");
  });
});
