import { describe, expect, it } from "vitest";
import { canEnterWorld, iconUrl, playedAgo } from "./worlds";

describe("canEnterWorld", () => {
  it("1.20 以後可以直接進入世界", () => {
    expect(canEnterWorld("1.20")).toBe(true);
    expect(canEnterWorld("1.21.8")).toBe(true);
    expect(canEnterWorld("26.3")).toBe(true);
  });

  it("1.19 系列不行", () => {
    expect(canEnterWorld("1.19")).toBe(false);
    expect(canEnterWorld("1.19.4")).toBe(false);
  });
});

describe("playedAgo", () => {
  const now = 100 * 24 * 3_600_000;

  it("用最接近的單位表達", () => {
    expect(playedAgo(now - 5_000, now)).toBe("剛剛");
    expect(playedAgo(now - 5 * 60_000, now)).toBe("5 分鐘前");
    expect(playedAgo(now - 3 * 3_600_000, now)).toBe("3 小時前");
    expect(playedAgo(now - 2 * 24 * 3_600_000, now)).toBe("2 天前");
    expect(playedAgo(now - 65 * 24 * 3_600_000, now)).toBe("2 個月前");
  });

  it("電腦時間比存檔早時不會出現負數", () => {
    expect(playedAgo(now + 60_000, now)).toBe("剛剛");
  });
});

describe("iconUrl", () => {
  it("轉成 PNG 圖片網址", () => {
    expect(iconUrl([72, 105])).toBe("data:image/png;base64,SGk=");
  });
});
