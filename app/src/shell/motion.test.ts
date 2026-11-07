import { describe, expect, it } from "vitest";
import { toMilliseconds } from "./motion";

describe("CSS 時長", () => {
  it("開發時的寫法", () => {
    expect(toMilliseconds("420ms")).toBe(420);
  });

  it("打包壓縮後的寫法", () => {
    expect(toMilliseconds(" .42s")).toBeCloseTo(420);
  });
});
