import { describe, expect, it } from "vitest";
import { lineLevel, matchesLine, tailOf } from "./logs";

describe("lineLevel", () => {
  it("分得出錯誤、警告和一般訊息", () => {
    expect(lineLevel("[main/ERROR]: boom")).toBe("error");
    expect(lineLevel("java.lang.NullPointerException")).toBe("error");
    expect(lineLevel("\tat net.minecraft.Main")).toBe("error");
    expect(lineLevel("[main/WARN]: careful")).toBe("warn");
    expect(lineLevel("[main/INFO]: hello")).toBe("info");
  });
});

describe("matchesLine", () => {
  it("可以只看問題", () => {
    expect(matchesLine("[main/INFO]: hello", true, "")).toBe(false);
    expect(matchesLine("[main/WARN]: careful", true, "")).toBe(true);
  });

  it("搜尋不分大小寫", () => {
    expect(matchesLine("Loading SODIUM", false, "sodium")).toBe(true);
    expect(matchesLine("Loading iris", false, "sodium")).toBe(false);
  });
});

describe("tailOf", () => {
  it("只留最後幾行", () => {
    expect(tailOf(["a", "b", "c"], 2)).toEqual(["b", "c"]);
    expect(tailOf(["a"], 2)).toEqual(["a"]);
  });
});
