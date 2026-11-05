export type VersionList = { latest: string; versions: string[] };

export type Profile = { id: string; name: string; skinUrl: string | null };

export type LoginCode = { userCode: string; verificationUri: string };

export type Instance = {
  id: string;
  name: string;
  version: string;
  loader: string;
  memoryMb: number;
  javaArgs: string;
};

export type Settings = {
  instance: Instance;
  config: { downloadConcurrency: number };
};

export type Progress = {
  stage: "game" | "java";
  finishedFiles: number;
  totalFiles: number;
  finishedBytes: number;
  totalBytes: number;
};

export type LaunchEvent =
  | ({ kind: "progress" } & Progress)
  | { kind: "started" }
  | { kind: "log"; line: string };
