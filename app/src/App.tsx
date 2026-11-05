import { useCallback, useEffect, useState } from "react";
import type { CSSProperties } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { DownloadPage } from "./DownloadPage";
import { InstancePage } from "./InstancePage";
import { LaunchPage } from "./LaunchPage";
import { Scene } from "./Scene";
import { SettingsPage } from "./SettingsPage";
import { overallProgress } from "./progress";
import { applyTheme } from "./theme";
import type { Instance, LaunchEvent, LauncherConfig, Progress, VersionList } from "./types";
import "./App.css";

type Tab = "launch" | "instances" | "download" | "settings" | "more";

const TABS: { id: Tab; label: string }[] = [
  { id: "launch", label: "啟動" },
  { id: "instances", label: "實例" },
  { id: "download", label: "下載" },
  { id: "settings", label: "設定" },
  { id: "more", label: "更多" },
];

const REPOSITORY_URL = "https://github.com/PZW152/sapling";
const USAGE_URL = `${REPOSITORY_URL}/blob/main/USAGE.md`;
const DEFAULT_CONFIG: LauncherConfig = {
  downloadConcurrency: 16,
  selectedInstance: "",
  theme: "sapling",
};

/** 目前的下載。失敗時保留 `error`，讓對應版本顯示原因。 */
export type InstallState = {
  id: string;
  progress: Progress | null;
  percent: number;
  error: string;
};

function App() {
  const [tab, setTab] = useState<Tab>("launch");
  const [direction, setDirection] = useState(1);
  const [list, setList] = useState<VersionList | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [instances, setInstances] = useState<Instance[]>([]);
  const [config, setConfig] = useState(DEFAULT_CONFIG);
  const [install, setInstall] = useState<InstallState | null>(null);

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

  function changeTab(next: Tab) {
    const index = (id: Tab) => TABS.findIndex((item) => item.id === id);
    setDirection(index(next) >= index(tab) ? 1 : -1);
    setTab(next);
  }

  function selectInstance(id: string) {
    const next = { ...config, selectedInstance: id };
    setConfig(next);
    invoke("save_config", { config: next });
  }

  /** 找到這個版本的實例；沒有就建立一個，以版本號為名稱。 */
  async function instanceForVersion(version: string) {
    const existing = instances.find((instance) => instance.version === version);
    if (existing) {
      return existing;
    }
    const created = await invoke<Instance>("create_instance", { name: version, version });
    await loadInstances();
    return created;
  }

  async function installGame(id: string) {
    setInstall({ id, progress: null, percent: 0, error: "" });
    try {
      await invoke("install_game", { version: id });
      selectInstance((await instanceForVersion(id)).id);
      setInstall(null);
      await loadVersions();
    } catch (reason) {
      setInstall({ id, progress: null, percent: 0, error: String(reason) });
    }
  }

  async function playVersion(version: string) {
    selectInstance((await instanceForVersion(version)).id);
    changeTab("launch");
  }

  const tabIndex = TABS.findIndex(({ id }) => id === tab);

  return (
    <div className="app" style={{ "--scene": tabIndex, "--dir": direction } as CSSProperties}>
      <Scene />
      <TitleBar tab={tab} onTabChange={changeTab} />
      {/* 啟動頁一直保留，切換分頁時遊戲狀態不會消失。 */}
      <main className="page" hidden={tab !== "launch"}>
        <LaunchPage
          instances={instances}
          selected={selected}
          listError={listError}
          onSelect={selectInstance}
          onGoToDownload={() => changeTab("download")}
          onRetry={loadVersions}
        />
      </main>
      {tab === "instances" && (
        <main className="page">
          <InstancePage
            instances={instances}
            list={list}
            selectedId={selected?.id ?? ""}
            onSelect={selectInstance}
            onChanged={loadInstances}
          />
        </main>
      )}
      {tab === "download" && (
        <main className="page">
          <DownloadPage
            list={list}
            listError={listError}
            install={install}
            onRetry={loadVersions}
            onInstall={installGame}
            onPlay={playVersion}
          />
        </main>
      )}
      {tab === "settings" && (
        <main className="page">
          <SettingsPage
            config={config}
            instance={selected}
            onConfigChange={setConfig}
            onInstanceChange={(next) =>
              setInstances((current) =>
                current.map((instance) => (instance.id === next.id ? next : instance)),
              )
            }
          />
        </main>
      )}
      {tab === "more" && (
        <main className="page">
          <About />
        </main>
      )}
    </div>
  );
}

function TitleBar({ tab, onTabChange }: { tab: Tab; onTabChange: (tab: Tab) => void }) {
  const appWindow = getCurrentWindow();

  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="brand" data-tauri-drag-region>
        <img src="/logo.svg" alt="" width={26} height={26} data-tauri-drag-region />
        <span data-tauri-drag-region>Sapling</span>
      </div>
      <nav className="tabs" aria-label="主要分頁">
        <span
          className="tab-indicator"
          style={{ transform: `translateX(${TABS.findIndex(({ id }) => id === tab) * 100}%)` }}
        />
        {TABS.map(({ id, label }) => (
          <button
            key={id}
            className="tab"
            aria-current={tab === id ? "page" : undefined}
            onClick={() => onTabChange(id)}
          >
            {label}
          </button>
        ))}
      </nav>
      <div className="window-controls">
        <button aria-label="最小化" onClick={() => appWindow.minimize()}>
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path d="M2 6h8" />
          </svg>
        </button>
        <button aria-label="最大化" onClick={() => appWindow.toggleMaximize()}>
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <rect x="2.5" y="2.5" width="7" height="7" rx="1" />
          </svg>
        </button>
        <button className="close" aria-label="關閉" onClick={() => appWindow.close()}>
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" />
          </svg>
        </button>
      </div>
    </header>
  );
}

function About() {
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  return (
    <div className="notice">
      <div className="notice-card about">
        <img src="/logo.svg" alt="" width={64} height={64} />
        <h2>Sapling Launcher</h2>
        <p>版本 {version}</p>
        <div className="about-links">
          <button className="secondary" onClick={() => invoke("open_link", { url: REPOSITORY_URL })}>
            GitHub 主頁
          </button>
          <button className="secondary" onClick={() => invoke("open_link", { url: USAGE_URL })}>
            使用指南
          </button>
        </div>
        <p className="about-note">非 Minecraft 官方產品，與 Mojang 和 Microsoft 無關。</p>
      </div>
    </div>
  );
}

export default App;
