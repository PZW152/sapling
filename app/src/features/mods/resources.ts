import type { Instance, LoaderKind, ModHit, ProjectKind, ProjectVersion } from "../../shell/types";

const BYTES_PER_MB = 1024 * 1024;
const MAX_TAGS = 3;
const MAX_LISTED_VERSIONS = 3;
const DATAPACK_LOADER = "datapack";
/** 原版實例裝不了模組，用一個不存在的名稱讓清單變空。 */
const NO_LOADER = "none";

export const SORT_OPTIONS = [
  { id: "", name: "預設排序" },
  { id: "downloads", name: "下載最多" },
  { id: "updated", name: "最近更新" },
  { id: "newest", name: "最新發佈" },
];

/** Modrinth 的模組分類和中文名稱。 */
export const CATEGORY_NAMES: Record<string, string> = {
  optimization: "優化",
  utility: "工具",
  library: "前置",
  adventure: "冒險",
  decoration: "裝飾",
  equipment: "裝備",
  food: "食物",
  "game-mechanics": "遊戲機制",
  magic: "魔法",
  management: "管理",
  minigame: "小遊戲",
  mobs: "生物",
  social: "社交",
  storage: "儲存",
  technology: "科技",
  transportation: "交通",
  worldgen: "世界生成",
};

const LOADER_IDS: Partial<Record<LoaderKind, string>> = {
  fabric: "fabric",
  forge: "forge",
  neoForge: "neoforge",
};

const LOADER_LABELS: Record<string, string> = {
  fabric: "Fabric",
  forge: "Forge",
  neoforge: "NeoForge",
  quilt: "Quilt",
  iris: "Iris",
  optifine: "OptiFine",
};

/** 接上下一頁。排名在翻頁途中變動時，同一個項目可能出現兩次。 */
export function appendHits(shown: ModHit[], next: ModHit[]) {
  const seen = new Set(shown.map((hit) => hit.projectId));
  return [...shown, ...next.filter((hit) => !seen.has(hit.projectId))];
}

/** 列表上顯示的分類，只顯示有中文名稱的幾個。 */
export function hitTags(hit: ModHit) {
  return hit.categories
    .map((category) => CATEGORY_NAMES[category])
    .filter(Boolean)
    .slice(0, MAX_TAGS);
}

/** 版本必須帶有這個標記才裝得上。同一個專案可能同時有模組版和資料包版。 */
function requiredLoader(kind: ProjectKind, instance: Instance) {
  if (kind === "dataPack") {
    return DATAPACK_LOADER;
  }
  return kind === "mod" ? (LOADER_IDS[instance.loader] ?? NO_LOADER) : null;
}

/** 只留下這個實例裝得上的版本。 */
export function compatibleVersions(
  versions: ProjectVersion[],
  instance: Instance | null | undefined,
  kind: ProjectKind,
) {
  if (!instance) {
    return versions;
  }
  const loader = requiredLoader(kind, instance);
  return versions.filter(
    (version) =>
      version.gameVersions.includes(instance.version) &&
      (loader === null || version.loaders.includes(loader)),
  );
}

/** 版本適用範圍的白話說明，遊戲版本太多時只列最新的幾個。 */
export function versionSummary(version: ProjectVersion) {
  const games = [...version.gameVersions].reverse();
  const shown = games.slice(0, MAX_LISTED_VERSIONS).join("、");
  const more = games.length > MAX_LISTED_VERSIONS ? ` 等 ${games.length} 個版本` : "";
  const loaders = version.loaders
    .map((loader) => LOADER_LABELS[loader])
    .filter(Boolean)
    .join("、");
  return loaders ? `${shown}${more}，${loaders}` : `${shown}${more}`;
}

export function formatSize(bytes: number) {
  return `${(bytes / BYTES_PER_MB).toFixed(1)} MB`;
}
