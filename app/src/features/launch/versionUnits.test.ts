import { describe, expect, it } from "vitest";
import { versionUnits } from "./versionUnits";

describe("versionUnits", () => {
  it("小數點只算半個字", () => {
    expect(versionUnits("26.3")).toBe(3.5);
  });

  it("字母比數字寬", () => {
    expect(versionUnits("26w1")).toBeCloseTo(4.2);
  });

  it("不會回傳零", () => {
    expect(versionUnits("")).toBe(1);
  });
});
