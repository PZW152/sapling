import { describe, expect, it } from "vitest";
import { isArborDay, petalOpenings, wearsZhongshan } from "./day";

const arborDay = new Date(2027, 2, 12);
const nextDay = new Date(2027, 2, 13);

describe("isArborDay", () => {
  it("只有 3 月 12 日是植樹節", () => {
    expect(isArborDay(arborDay)).toBe(true);
    expect(isArborDay(nextDay)).toBe(false);
    expect(isArborDay(new Date(2027, 3, 12))).toBe(false);
  });
});

describe("petalOpenings", () => {
  it("花瓣按順序開", () => {
    expect(petalOpenings(0, 5)).toEqual([0, 0, 0, 0, 0]);
    expect(petalOpenings(0.5, 5)).toEqual([1, 1, 0.5, 0, 0]);
    expect(petalOpenings(1, 5)).toEqual([1, 1, 1, 1, 1]);
  });

  it("超出範圍的進度不會壞掉", () => {
    expect(petalOpenings(-1, 5)).toEqual([0, 0, 0, 0, 0]);
    expect(petalOpenings(2, 5)).toEqual([1, 1, 1, 1, 1]);
  });
});

describe("wearsZhongshan", () => {
  it("當天種下今年的樹才換上", () => {
    expect(wearsZhongshan([2027], arborDay)).toBe(true);
    expect(wearsZhongshan([], arborDay)).toBe(false);
  });

  it("去年種過不算", () => {
    expect(wearsZhongshan([2026], arborDay)).toBe(false);
  });

  it("隔天自動恢復", () => {
    expect(wearsZhongshan([2027], nextDay)).toBe(false);
  });
});
