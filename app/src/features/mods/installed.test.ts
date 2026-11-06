import { describe, expect, it } from "vitest";
import type { InstalledMod, ModFile } from "../../shell/types";
import { matchesMod, modTitle, updatable } from "./installed";

const file: ModFile = { name: "sodium-0.8.jar", enabled: true, size: 1 };
const sodium: InstalledMod = {
  fileName: "sodium-0.8.jar",
  projectId: "p",
  title: "Sodium",
  slug: "sodium",
  iconUrl: null,
  update: { versionId: "v", versionName: "0.9" },
};

describe("modTitle", () => {
  it("查不到時顯示檔名", () => {
    expect(modTitle(file, undefined)).toBe("sodium-0.8.jar");
  });

  it("熱門模組帶中文名", () => {
    expect(modTitle(file, sodium)).toBe("鈉 Sodium");
  });
});

describe("matchesMod", () => {
  it("中文名、英文名和檔名都搜得到", () => {
    expect(matchesMod(file, sodium, "鈉")).toBe(true);
    expect(matchesMod(file, sodium, "SODIUM")).toBe(true);
    expect(matchesMod(file, undefined, "0.8")).toBe(true);
    expect(matchesMod(file, sodium, "iris")).toBe(false);
  });
});

describe("updatable", () => {
  it("略過已經不在資料夾裡的模組", () => {
    expect(updatable([file], [sodium])).toEqual([sodium]);
    expect(updatable([], [sodium])).toEqual([]);
  });

  it("略過沒有新版的模組", () => {
    expect(updatable([file], [{ ...sodium, update: null }])).toEqual([]);
  });
});
