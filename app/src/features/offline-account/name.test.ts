import { describe, expect, it } from "vitest";
import type { LauncherConfig } from "../../shell/types";
import { isValidPlayerName, offlineLaunchBlock } from "./name";

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

describe("offlineLaunchBlock", () => {
  const config = (offlineEnabled: boolean, offlineName: string) =>
    ({ offlineEnabled, offlineName }) as LauncherConfig;

  it("離線模式開著但名稱不能用時擋下啟動", () => {
    expect(offlineLaunchBlock(config(true, ""))).not.toBeNull();
    expect(offlineLaunchBlock(config(true, "ab"))).not.toBeNull();
  });

  it("名稱可用或離線模式關閉時不阻擋", () => {
    expect(offlineLaunchBlock(config(true, "Steve"))).toBeNull();
    expect(offlineLaunchBlock(config(false, "ab"))).toBeNull();
  });
});
