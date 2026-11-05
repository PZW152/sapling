import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { applyTheme } from "../features/appearance/theme";
import { overallProgress } from "./progress";
import type { Instance, LaunchEvent, LauncherConfig, Progress, VersionList } from "./types";

/** 停止調整多久後自動保存。 */
const SAVE_DELAY_MS = 400;
const DEFAULT_CONFIG: LauncherConfig = {
  downloadConcurrency: 16,
  selectedInstance: "",
  theme: "sapling",
  disabledFeatures: [],
  featureSwitchesUnlocked: false,
  plantedYears: [],
};

/** 目前的下載。失敗時保留 `error`，讓對應版本顯示原因。 */
export type InstallState = {
  id: string;
  progress: Progress | null;
  percent: number;
  error: string;
};

type AppState = {
  list: VersionList | null;
  listError: string | null;
  instances: Instance[];
  selected: Instance | null;
  config: LauncherConfig;
  install: InstallState | null;
  saveError: string;
  isEnabled: (featureId: string) => boolean;
  goTo: (featureId: string) => void;
  activeTab: string;
  loadVersions: () => Promise<void>;
  loadInstances: () => Promise<void>;
  selectInstance: (id: string) => void;
  updateConfig: (change: Partial<LauncherConfig>) => void;
  updateInstance: (change: Partial<Instance>) => void;
  installGame: (version: string) => Promise<void>;
  playVersion: (version: string) => Promise<void>;
};

const AppContext = createContext<AppState | null>(null);

/** 所有功能模塊共用的狀態。模塊之間只透過這裡溝通。 */
export function useApp() {
  const state = useContext(AppContext);
  if (!state) {
    throw new Error("useApp 必須在 AppProvider 裡使用");
  }
  return state;
}

export function AppProvider({
  children,
  activeTab,
  goTo,
}: {
  children: ReactNode;
  activeTab: string;
  goTo: (featureId: string) => void;
}) {
  const [list, setList] = useState<VersionList | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [instances, setInstances] = useState<Instance[]>([]);
  const [config, setConfig] = useState(DEFAULT_CONFIG);
  const [install, setInstall] = useState<InstallState | null>(null);
  const [saveError, setSaveError] = useState("");
  const saveTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  const selected =
    instances.find((instance) => instance.id === config.selectedInstance) ?? instances[0] ?? null;

  const loadInstances = useCallback(async () => {
    setInstances(await invoke<Instance[]>("list_instances"));
  }, []);

  const loadVersions = useCallback(async () => {
    setListError(null);
    try {
      setList(await invoke<VersionList>("list_versions"));
    } catch (reason) {
      setListError(String(reason));
    }
  }, []);

  useEffect(() => {
    invoke<LauncherConfig>("load_config").then((saved) => {
      setConfig(saved);
      applyTheme(saved.theme);
    });
    loadInstances();
    loadVersions();
  }, [loadInstances, loadVersions]);

  useEffect(() => {
    const stopListening = listen<LaunchEvent>("install-event", ({ payload }) => {
      if (payload.kind === "progress") {
        setInstall(
          (current) =>
            current && { ...current, progress: payload, percent: overallProgress(payload) },
        );
      }
    });
    return () => {
      stopListening.then((stop) => stop());
    };
  }, []);

  const queueSave = useCallback((save: () => Promise<unknown>) => {
    clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => {
      save()
        .then(() => setSaveError(""))
        .catch((reason) => setSaveError(String(reason)));
    }, SAVE_DELAY_MS);
  }, []);

  const updateConfig = useCallback(
    (change: Partial<LauncherConfig>) => {
      const next = { ...config, ...change };
      setConfig(next);
      queueSave(() => invoke("save_config", { config: next }));
    },
    [config, queueSave],
  );

  const updateInstance = useCallback(
    (change: Partial<Instance>) => {
      if (!selected) {
        return;
      }
      const next = { ...selected, ...change };
      setInstances((current) =>
        current.map((instance) => (instance.id === next.id ? next : instance)),
      );
      queueSave(() => invoke("save_instance", { instance: next }));
    },
    [selected, queueSave],
  );

  const selectInstance = useCallback(
    (id: string) => updateConfig({ selectedInstance: id }),
    [updateConfig],
  );

  /** 找到這個版本的實例；沒有就建立一個，以版本號為名稱。 */
  const instanceForVersion = useCallback(
    async (version: string) => {
      const existing = instances.find((instance) => instance.version === version);
      if (existing) {
        return existing;
      }
      const created = await invoke<Instance>("create_instance", { name: version, version });
      await loadInstances();
      return created;
    },
    [instances, loadInstances],
  );

  const installGame = useCallback(
    async (version: string) => {
      setInstall({ id: version, progress: null, percent: 0, error: "" });
      try {
        await invoke("install_game", { version });
        selectInstance((await instanceForVersion(version)).id);
        setInstall(null);
        await loadVersions();
      } catch (reason) {
        setInstall({ id: version, progress: null, percent: 0, error: String(reason) });
      }
    },
    [instanceForVersion, loadVersions, selectInstance],
  );

  const playVersion = useCallback(
    async (version: string) => {
      selectInstance((await instanceForVersion(version)).id);
      goTo("launch");
    },
    [goTo, instanceForVersion, selectInstance],
  );

  const value = useMemo(
    () => ({
      list,
      listError,
      instances,
      selected,
      config,
      install,
      saveError,
      activeTab,
      isEnabled: (featureId: string) => !config.disabledFeatures.includes(featureId),
      goTo,
      loadVersions,
      loadInstances,
      selectInstance,
      updateConfig,
      updateInstance,
      installGame,
      playVersion,
    }),
    [
      list,
      listError,
      instances,
      selected,
      config,
      install,
      saveError,
      activeTab,
      goTo,
      loadVersions,
      loadInstances,
      selectInstance,
      updateConfig,
      updateInstance,
      installGame,
      playVersion,
    ],
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}
