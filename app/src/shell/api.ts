import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CleanupSummary,
  ImportSource,
  Instance,
  LaunchEvent,
  LauncherConfig,
  LoaderKind,
  LoginCode,
  AddedKind,
  ExportSummary,
  InstalledMod,
  ModFile,
  Screenshot,
  Server,
  PackFile,
  PackKind,
  ProjectKind,
  ProjectVersion,
  SearchPage,
  SearchRequest,
  Profile,
  UpdateInfo,
  VersionList,
  World,
} from "./types";

export type ModAction = { instanceId: string; name: string; enabled: boolean };

/** 前端呼叫後端的唯一入口。命令名稱只出現在這裡，要和 src-tauri/src/lib.rs 一致。 */
export const api = {
  listVersions: () => invoke<VersionList>("list_versions"),
  systemMemoryMb: () => invoke<number>("system_memory_mb"),

  loadConfig: () => invoke<LauncherConfig>("load_config"),
  saveConfig: (config: LauncherConfig) => invoke<void>("save_config", { config }),

  listInstances: () => invoke<Instance[]>("list_instances"),
  createInstance: (name: string, version: string, loader: LoaderKind) =>
    invoke<Instance>("create_instance", { name, version, loader }),
  saveInstance: (instance: Instance) => invoke<void>("save_instance", { instance }),
  duplicateInstance: (id: string) => invoke<Instance>("duplicate_instance", { id }),
  deleteInstance: (id: string) => invoke<void>("delete_instance", { id }),
  openInstanceFolder: (id: string) => invoke<void>("open_instance_folder", { id }),

  recentWorlds: (id: string) => invoke<World[]>("recent_worlds", { id }),
  cleanupPreview: () => invoke<CleanupSummary>("cleanup_preview"),
  cleanupRun: () => invoke<number>("cleanup_run"),
  listImportSources: () => invoke<ImportSource[]>("list_import_sources"),
  importWorld: (id: string, source: string, folder: string) =>
    invoke<string>("import_world", { id, source, folder }),
  listWorlds: (id: string) => invoke<World[]>("list_worlds", { id }),
  backupWorld: (id: string, folder: string) => invoke<string>("backup_world", { id, folder }),

  searchProjects: (request: SearchRequest) => invoke<SearchPage>("search_projects", { request }),
  projectVersions: (projectId: string) =>
    invoke<ProjectVersion[]>("project_versions", { projectId }),
  installProject: (
    id: string,
    kind: ProjectKind,
    projectId: string,
    versionId: string | null,
    world: string | null = null,
  ) => invoke<string[]>("install_project", { id, kind, projectId, versionId, world }),
  installPack: (projectId: string, versionId: string | null, name: string) =>
    invoke<Instance>("install_pack", { projectId, versionId, name }),

  listMods: (id: string) => invoke<ModFile[]>("list_mods", { id }),
  setModEnabled: (action: ModAction) => invoke<void>("set_mod_enabled", { action }),
  deleteMod: (action: ModAction) => invoke<void>("delete_mod", { action }),
  openModsFolder: (id: string) => invoke<void>("open_mods_folder", { id }),
  listServers: (id: string) => invoke<Server[]>("list_servers", { id }),
  listScreenshots: (id: string) => invoke<Screenshot[]>("list_screenshots", { id }),
  deleteScreenshot: (id: string, name: string) =>
    invoke<void>("delete_screenshot", { id, name }),
  createShortcut: (id: string) => invoke<void>("create_shortcut", { id }),
  takePendingLaunch: () => invoke<string | null>("take_pending_launch"),
  diagnostics: () => invoke<string>("diagnostics"),
  openDataFolder: () => invoke<void>("open_data_folder"),
  cancelInstall: () => invoke<void>("cancel_install"),
  changeSkin: (path: string, slim: boolean) => invoke<Profile>("change_skin", { path, slim }),
  deleteWorld: (id: string, folder: string) => invoke<void>("delete_world", { id, folder }),
  openInstanceSubfolder: (id: string, name: string) =>
    invoke<void>("open_instance_subfolder", { id, name }),
  installLocalPack: (path: string) => invoke<Instance>("install_local_pack", { path }),
  exportPack: (id: string, path: string) => invoke<ExportSummary>("export_pack", { id, path }),
  addInstanceFile: (id: string, path: string) =>
    invoke<AddedKind>("add_instance_file", { id, path }),
  changeInstanceVersion: (id: string, version: string, loader: LoaderKind) =>
    invoke<Instance>("change_instance_version", { id, version, loader }),
  repairInstance: (id: string) => invoke<void>("repair_instance", { id }),
  inspectMods: (id: string) => invoke<InstalledMod[]>("inspect_mods", { id }),
  listPacks: (id: string, kind: PackKind) => invoke<PackFile[]>("list_packs", { id, kind }),
  deletePack: (id: string, kind: PackKind, name: string) =>
    invoke<void>("delete_pack", { id, kind, name }),
  openPackFolder: (id: string, kind: PackKind) =>
    invoke<void>("open_pack_folder", { id, kind }),

  restoreAccount: () => invoke<Profile | null>("restore_account"),
  loginStart: () => invoke<LoginCode>("login_start"),
  /** 用戶取消時回傳 `null`。 */
  loginFinish: () => invoke<Profile | null>("login_finish"),
  loginCancel: () => invoke<void>("cancel_login"),
  signOut: () => invoke<void>("sign_out"),
  listAccounts: () => invoke<Profile[]>("list_accounts"),
  switchAccount: (id: string) => invoke<Profile | null>("switch_account", { id }),
  hasSavedAccount: () => invoke<boolean>("has_saved_account"),

  installGame: (version: string) => invoke<void>("install_game", { version }),
  playGame: (
    instanceId: string,
    offlineName: string | null,
    world: string | null,
    server: string | null,
  ) => invoke<void>("play_game", { instanceId, offlineName, world, server }),
  cancelLaunch: () => invoke<void>("cancel_launch"),

  openLink: (url: string) => invoke<void>("open_link", { url }),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  applyUpdate: (update: UpdateInfo) => invoke<void>("apply_update", { update }),
};

function subscribe(event: string, handler: (payload: LaunchEvent) => void) {
  const stopListening = listen<LaunchEvent>(event, ({ payload }) => handler(payload));
  return () => {
    stopListening.then((stop) => stop());
  };
}

/** 訂閱啟動過程的事件，回傳取消訂閱的函數。 */
export const onLaunchEvent = (handler: (payload: LaunchEvent) => void) =>
  subscribe("launch-event", handler);

/** 從桌面捷徑要求啟動實例，回傳取消訂閱的函數。 */
export function onLaunchRequest(handler: (instanceId: string) => void) {
  const stopListening = listen<string>("launch-request", ({ payload }) => handler(payload));
  return () => {
    stopListening.then((stop) => stop());
  };
}

/** 訂閱下載過程的事件，回傳取消訂閱的函數。 */
export const onInstallEvent = (handler: (payload: LaunchEvent) => void) =>
  subscribe("install-event", handler);

/** 訂閱整合包的安裝進度。遊戲下載也在跑時，兩邊的進度不能混在一起。 */
export const onPackEvent = (handler: (payload: LaunchEvent) => void) =>
  subscribe("pack-event", handler);
