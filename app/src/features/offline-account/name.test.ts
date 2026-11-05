import { describe, expect, it } from "vitest";
import { isValidPlayerName } from "./name";

describe("isValidPlayerName", () => {
  it("接受遊戲允許的名稱", () => {
    expect(isValidPlayerName("Steve_01")).toBe(true);
  });

  it("拒絕太短、太長和含其他字元的名稱", () => {
    expect(isValidPlayerName("ab")).toBe(false);
    expect(isValidPlayerName("a".repeat(17))).toBe(false);
    expect(isValidPlayerName("樹苗玩家")).toBe(false);
    expect(isValidPlayerName("has space")).toBe(false);
  });
});
