import { describe, expect, it } from "vitest";
import type { ModHit } from "../types";
import { displayTitle, searchTerm } from "./names";

describe("displayTitle", () => {
  it("有中文名時放在前面", () => {
    expect(displayTitle({ slug: "sodium", title: "Sodium" } as ModHit)).toBe("鈉 Sodium");
  });

  it("沒有中文名時保留原名", () => {
    expect(displayTitle({ slug: "unknown", title: "Unknown" } as ModHit)).toBe("Unknown");
  });
});

describe("searchTerm", () => {
  it("中文名換成英文再搜尋", () => {
    expect(searchTerm("機械動力")).toBe("create");
    expect(searchTerm("農夫")).toBe("farmers delight");
  });

  it("英文和空白不動", () => {
    expect(searchTerm("sodium")).toBe("sodium");
    expect(searchTerm("  ")).toBe("  ");
  });
});
