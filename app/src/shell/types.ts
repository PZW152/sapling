export type VersionList = { latest: string; versions: string[]; installed: string[] };

export type Profile = { id: string; name: string; skinUrl: string | null };

export type LoginCode = { userCode: string; verificationUri: string };

export type Instance = {
  id: string;
  name: string;
  version: string;
  loader: string;
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
