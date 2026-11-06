import { describe, expect, it } from "vitest";
import { bendAt, edgeDistance } from "./liquid";

const lens = { width: 100, height: 40, radius: 12, bezel: 12 };

describe("液態玻璃的光學計算", () => {
  it("邊緣折得最厲害，越往中間越平", () => {
    expect(bendAt(0.05)).toBeGreaterThan(bendAt(0.5));
    expect(bendAt(0.5)).toBeGreaterThan(bendAt(0.95));
    expect(bendAt(0.999)).toBeLessThan(0.02);
  });

  it("偏移量不會超出位移圖能表示的範圍", () => {
    for (let t = 0; t <= 1; t += 0.01) {
      expect(bendAt(t)).toBeGreaterThanOrEqual(0);
      expect(bendAt(t)).toBeLessThanOrEqual(1);
    }
  });

  it("算得出圓角矩形內外的距離", () => {
    expect(edgeDistance(50, 20, lens)).toBeCloseTo(20);
    expect(edgeDistance(50, 1, lens)).toBeCloseTo(1);
    expect(edgeDistance(0, 0, lens)).toBeLessThan(0);
  });
});
