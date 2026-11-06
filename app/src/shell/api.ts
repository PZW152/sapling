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
  ModFile,
  ModHit,
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

  searchPacks: (query: string) => invoke<ModHit[]>("search_packs", { query }),
  installPack: (projectId: string, name: string) =>
    invoke<Instance>("install_pack", { projectId, name }),
  searchMods: (query: string, version: string, loader: LoaderKind) =>
    invoke<ModHit[]>("search_mods", { query, version, loader }),
  installMod: (id: string, projectId: string) =>
    invoke<string[]>("install_mod", { id, projectId }),

  listMods: (id: string) => invoke<ModFile[]>("list_mods", { id }),
  setModEnabled: (action: ModAction) => invoke<void>("set_mod_enabled", { action }),
  deleteMod: (action: ModAction) => invoke<void>("delete_mod", { action }),
  openModsFolder: (id: string) => invoke<void>("open_mods_folder", { id }),

  restoreAccount: () => invoke<Profile | null>("restore_account"),
  loginStart: () => invoke<LoginCode>("login_start"),
  loginFinish: () => invoke<Profile>("login_finish"),
  signOut: () => invoke<void>("sign_out"),
  listAccounts: () => invoke<Profile[]>("list_accounts"),
  switchAccount: (id: string) => invoke<Profile | null>("switch_account", { id }),
  hasSavedAccount: () => invoke<boolean>("has_saved_account"),

  installGame: (version: string) => invoke<void>("install_game", { version }),
  playGame: (instanceId: string, offlineName: string | null, world: string | null) =>
    invoke<void>("play_game", { instanceId, offlineName, world }),
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

/** 訂閱下載過程的事件，回傳取消訂閱的函數。 */
export const onInstallEvent = (handler: (payload: LaunchEvent) => void) =>
  subscribe("install-event", handler);
