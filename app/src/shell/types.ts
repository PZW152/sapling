export type VersionList = {
  latest: string;
  versions: string[];
  snapshots: string[];
  installed: string[];
};

export type Profile = { id: string; name: string; skinUrl: string | null };

export type LoginCode = { userCode: string; verificationUri: string };

export type LoaderKind = "vanilla" | "fabric" | "quilt" | "forge" | "neoForge";

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
  /** 0 代表用遊戲的預設大小。 */
  windowWidth: number;
  windowHeight: number;
  fullscreen: boolean;
  /** 空的代表自動選擇 Java。 */
  javaPath: string;
  /** 遊戲開始後直接加入的伺服器。 */
  joinServer: string;
  /** 0 代表還沒玩過。 */
  lastPlayedMs: number;
  playedSeconds: number;
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
  favorites: Favorite[];
  /** 介面大小的百分比。 */
  uiScale: number;
  colorMode: ColorMode;
  minimizeOnLaunch: boolean;
};

export type Server = { name: string; address: string };
export type Screenshot = { name: string; path: string; takenMs: number };

export type ColorMode = "system" | "light" | "dark";
export type AddedKind = "mod" | "resourcePack" | "shader";
export type ExportSummary = { linkedMods: number; bundledMods: number };

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

export type ProjectKind = "mod" | "modpack" | "resourcePack" | "shader" | "dataPack";

export type ModHit = {
  projectId: string;
  title: string;
  description: string;
  downloads: number;
  iconUrl: string | null;
  slug: string;
  author: string;
  categories: string[];
  dateModified: string;
};

/** 收藏的資源。 */
export type Favorite = ModHit & { kind: ProjectKind };

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

/** Modrinth 認得的模組。手動放進來的模組沒有這份資料。 */
export type InstalledMod = {
  fileName: string;
  projectId: string;
  title: string;
  slug: string;
  iconUrl: string | null;
  update: { versionId: string; versionName: string } | null;
};

export type PackKind = "resourcePack" | "shader";
export type PackFile = { name: string; size: number };

export type UpdateInfo = { version: string; notes: string; url: string; sha256: string };

export type Progress = {
  stage: "game" | "java";
  finishedFiles: number;
  downloadedFiles: number;
  totalFiles: number;
  finishedBytes: number;
  totalBytes: number;
  /** 真正下載的位元組數，已存在而跳過的不算。 */
  downloadedBytes: number;
};

export type LaunchEvent =
  | ({ kind: "progress" } & Progress)
  | { kind: "started" }
  | { kind: "log"; line: string };
