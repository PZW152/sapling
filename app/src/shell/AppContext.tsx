import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { api } from "./api";
import { applyColorMode, applyTheme, applyMaterial } from "./applyTheme";
import { applyScale } from "./applyScale";
import type { Feature } from "./feature";
import { recentFirst } from "./instances";
import { createSaveQueue } from "./saveQueue";
import type { Instance, LauncherConfig, VersionList } from "./types";

/** 停止調整多久後自動保存。 */
const SAVE_DELAY_MS = 400;
const DEFAULT_CONFIG: LauncherConfig = {
  downloadConcurrency: 16,
  selectedInstance: "",
  theme: "sapling",
  disabledFeatures: [],
  featureSwitchesUnlocked: false,
  plantedYears: [],
  offlineEnabled: false,
  offlineName: "",
  favorites: [],
  uiScale: 100,
  colorMode: "system",
  glass: true,
  minimizeOnLaunch: true,
  seenVersion: "",
};

type AppState = {
  /** 實例和設定第一次載入完成前為 false。 */
  loaded: boolean;
  list: VersionList | null;
  listError: string | null;
  instances: Instance[];
  selected: Instance | null;
  config: LauncherConfig;
  saveError: string;
  /** 目前開著的功能模塊。 */
  features: Feature[];
  /** 全部功能模塊，包含關閉的。 */
  allFeatures: Feature[];
  isEnabled: (featureId: string) => boolean;
  goTo: (featureId: string) => void;
  activeTab: string;
  /** 實例裡的檔案從外部改變時增加，清單用它重新讀取。 */
  contentVersion: number;
  touchContent: () => void;
  loadVersions: () => Promise<void>;
  loadInstances: () => Promise<void>;
  selectInstance: (id: string) => void;
  updateConfig: (change: Partial<LauncherConfig>) => void;
  updateInstance: (change: Partial<Instance>) => void;
  /** 立刻保存等待中的改動。 */
  flushSaves: () => Promise<void>;
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
  allFeatures,
  activeTab,
  goTo,
}: {
  children: ReactNode;
  allFeatures: Feature[];
  activeTab: string;
  goTo: (featureId: string) => void;
}) {
  const [list, setList] = useState<VersionList | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [instances, setInstances] = useState<Instance[]>([]);
  const [config, setConfig] = useState(DEFAULT_CONFIG);
  const [saveError, setSaveError] = useState("");
  const [loaded, setLoaded] = useState(false);
  const [contentVersion, setContentVersion] = useState(0);
  const [saves] = useState(() => createSaveQueue(SAVE_DELAY_MS, setSaveError));
  // 長時間的操作結束後才改設定時，要以最新的設定為基礎，不能用當初的舊值。
  const latestConfig = useRef(config);

  const selected =
    instances.find((instance) => instance.id === config.selectedInstance) ?? instances[0] ?? null;

  const loadInstances = useCallback(async () => {
    setInstances(recentFirst(await api.listInstances()));
  }, []);

  const loadVersions = useCallback(async () => {
    setListError(null);
    try {
      setList(await api.listVersions());
    } catch (reason) {
      setListError(String(reason));
    }
  }, []);

  useEffect(() => {
    Promise.all([api.loadConfig(), loadInstances()])
      .then(([saved]) => {
        latestConfig.current = saved;
        setConfig(saved);
        applyTheme(saved.theme);
        applyScale(saved.uiScale);
      })
      .finally(() => setLoaded(true));
    loadVersions();
  }, [loadInstances, loadVersions]);

  // 跟隨系統時，系統切換深淺色要立刻反應。
  useEffect(() => {
    applyColorMode(config.colorMode);
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const follow = () => applyColorMode(config.colorMode);
    query.addEventListener("change", follow);
    return () => query.removeEventListener("change", follow);
  }, [config.colorMode]);

  useEffect(() => {
    if (loaded) {
      applyMaterial(config.glass);
    }
  }, [loaded, config.glass]);

  const touchContent = useCallback(() => setContentVersion((version) => version + 1), []);

  const updateConfig = useCallback(
    (change: Partial<LauncherConfig>) => {
      const next = { ...latestConfig.current, ...change };
      latestConfig.current = next;
      setConfig(next);
      saves.schedule("config", () => api.saveConfig(next));
    },
    [saves],
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
      saves.schedule(`instance:${next.id}`, () => api.saveInstance(next));
    },
    [selected, saves],
  );

  const selectInstance = useCallback(
    (id: string) => updateConfig({ selectedInstance: id }),
    [updateConfig],
  );

  const features = useMemo(
    () =>
      allFeatures.filter(
        (feature) => !feature.optional || !config.disabledFeatures.includes(feature.id),
      ),
    [allFeatures, config.disabledFeatures],
  );

  const value = useMemo(
    () => ({
      loaded,
      list,
      listError,
      instances,
      selected,
      config,
      saveError,
      features,
      allFeatures,
      activeTab,
      contentVersion,
      touchContent,
      isEnabled: (featureId: string) => !config.disabledFeatures.includes(featureId),
      goTo,
      loadVersions,
      loadInstances,
      selectInstance,
      updateConfig,
      updateInstance,
      flushSaves: saves.flush,
    }),
    [
      loaded,
      list,
      listError,
      instances,
      selected,
      config,
      saveError,
      features,
      allFeatures,
      activeTab,
      contentVersion,
      touchContent,
      goTo,
      loadVersions,
      loadInstances,
      selectInstance,
      updateConfig,
      updateInstance,
      saves,
    ],
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}
