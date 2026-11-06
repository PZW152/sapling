export type VersionList = { latest: string; versions: string[]; installed: string[] };

export type Profile = { id: string; name: string; skinUrl: string | null };

export type LoginCode = { userCode: string; verificationUri: string };

export type LoaderKind = "vanilla" | "fabric" | "forge" | "neoForge";

export type Instance = {
  id: string;
  name: string;
  version: string;
  loader: LoaderKind;
  /** 載入器版本，原版是空的。 */
  loaderVersion: string;
  /** 0 代表自動分配。 */
  memoryMb: number;
  javaArgs: string;
};

export type LauncherConfig = {
  downloadConcurrency: number;
  selectedInstance: string;
  theme: string;
  disabledFeatures: string[];
  featureSwitchesUnlocked: boolean;
  plantedYears: number[];
  offlineEnabled: boolean;
  offlineName: string;
};

export type World = {
  /** 世界的資料夾名稱。 */
  folder: string;
  lastPlayedMs: number;
  /** `icon.png` 的內容，沒有縮圖時為 null。 */
  icon: number[] | null;
};

export type CleanupSummary = { bytes: number; items: number };

export type ImportSource = {
  label: string;
  path: string;
  worlds: World[];
};

export type ProjectKind = "mod" | "modpack" | "resourcePack" | "shader";

export type ModHit = {
  projectId: string;
  title: string;
  description: string;
  downloads: number;
  iconUrl: string | null;
  author: string;
  categories: string[];
  dateModified: string;
};

export type SearchRequest = {
  query: string;
  kind: ProjectKind;
  gameVersion: string | null;
  loader: LoaderKind | null;
  category: string | null;
  sort: string | null;
  offset: number;
};

export type SearchPage = { hits: ModHit[]; total: number };

export type ProjectVersion = {
  id: string;
  name: string;
  gameVersions: string[];
  loaders: string[];
  versionType: string;
  datePublished: string;
  fileName: string;
  size: number;
};

export type ModFile = { name: string; enabled: boolean; size: number };

export type UpdateInfo = { version: string; notes: string; url: string; sha256: string };

export type Progress = {
  stage: "game" | "java";
  finishedFiles: number;
  downloadedFiles: number;
  totalFiles: number;
  finishedBytes: number;
  totalBytes: number;
};

export type LaunchEvent =
  | ({ kind: "progress" } & Progress)
  | { kind: "started" }
  | { kind: "log"; line: string };
