import { describe, expect, it } from "vitest";
import type { Instance, ModHit, ProjectVersion } from "../types";
import { appendHits, compatibleVersions, formatSize, hitTags, versionSummary } from "./resources";

function version(gameVersions: string[], loaders: string[]): ProjectVersion {
  return {
    id: "v",
    name: "1.0",
    gameVersions,
    loaders,
    versionType: "release",
    datePublished: "2026-01-01",
    fileName: "a.jar",
    size: 1,
  };
}

const fabric261 = { version: "1.21.8", loader: "fabric" } as Instance;

describe("compatibleVersions", () => {
  const list = [
    version(["1.21.8"], ["fabric"]),
    version(["1.21.8"], ["neoforge"]),
    version(["1.20.1"], ["fabric"]),
  ];

  it("模組要同時符合遊戲版本和載入器", () => {
    expect(compatibleVersions(list, fabric261, "mod")).toEqual([list[0]]);
  });

  it("資源包只看遊戲版本", () => {
    expect(compatibleVersions(list, fabric261, "resourcePack")).toEqual([list[0], list[1]]);
  });

  it("沒有目標實例時全部列出", () => {
    expect(compatibleVersions(list, null, "mod")).toHaveLength(3);
  });

  it("資料包不會選到同專案的模組版", () => {
    const both = [version(["1.21.8"], ["fabric"]), version(["1.21.8"], ["datapack"])];
    expect(compatibleVersions(both, fabric261, "dataPack")).toEqual([both[1]]);
  });
});

describe("versionSummary", () => {
  it("遊戲版本太多時只列最新的", () => {
    const text = versionSummary(version(["1.20", "1.21", "1.21.1", "1.21.8"], ["fabric", "forge"]));
    expect(text).toBe("1.21.8、1.21.1、1.21 等 4 個版本，Fabric、Forge");
  });

  it("資源包沒有載入器", () => {
    expect(versionSummary(version(["1.21.8"], ["minecraft"]))).toBe("1.21.8");
  });
});

describe("appendHits", () => {
  it("略過已經顯示的項目", () => {
    const hit = (projectId: string) => ({ projectId }) as ModHit;
    const merged = appendHits([hit("a"), hit("b")], [hit("b"), hit("c")]);
    expect(merged.map(({ projectId }) => projectId)).toEqual(["a", "b", "c"]);
  });
});

describe("hitTags", () => {
  it("只顯示有中文名稱的分類", () => {
    const hit = { categories: ["fabric", "optimization", "utility"] } as ModHit;
    expect(hitTags(hit)).toEqual(["優化", "工具"]);
  });
});

describe("formatSize", () => {
  it("用 MB 表示", () => {
    expect(formatSize(1_572_864)).toBe("1.5 MB");
  });
});
