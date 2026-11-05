import { describe, expect, it } from "vitest";
import { groupBySeries } from "./series";

describe("groupBySeries", () => {
  it("舊編號按前兩段分組，新編號按第一段分組", () => {
    expect(groupBySeries(["26.1.2", "26.1", "1.21.4", "1.21.3", "1.20.6"])).toEqual([
      { series: "26", versions: ["26.1.2", "26.1"] },
      { series: "1.21", versions: ["1.21.4", "1.21.3"] },
      { series: "1.20", versions: ["1.20.6"] },
    ]);
  });

  it("沒有版本時回傳空清單", () => {
    expect(groupBySeries([])).toEqual([]);
  });
});
