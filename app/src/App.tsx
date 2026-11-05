import { useCallback, useEffect, useState } from "react";
import type { CSSProperties } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { DownloadPage } from "./DownloadPage";
import { LaunchPage } from "./LaunchPage";
import { SettingsPage } from "./SettingsPage";
import { overallProgress } from "./progress";
import type { LaunchEvent, Progress, Settings, VersionList } from "./types";
import "./App.css";

type Tab = "launch" | "download" | "settings" | "more";

const TABS: { id: Tab; label: string }[] = [
  { id: "launch", label: "啟動" },
  { id: "download", label: "下載" },
  { id: "settings", label: "設定" },
  { id: "more", label: "更多" },
];

const REPOSITORY_URL = "https://github.com/PZW152/sapling";
const USAGE_URL = `${REPOSITORY_URL}/blob/main/USAGE.md`;

/** 目前的下載。失敗時保留 `error`，讓對應版本顯示原因。 */
export type InstallState = {
  id: string;
  progress: Progress | null;
  percent: number;
  error: string;
};

function App() {
  const [tab, setTab] = useState<Tab>("launch");
  const [list, setList] = useState<VersionList | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [selected, setSelected] = useState("");
  const [install, setInstall] = useState<InstallState | null>(null);

  const loadVersions = useCallback(async () => {
    setListError(null);
    try {
      const [versions, settings] = await Promise.all([
        invoke<VersionList>("list_versions"),
        invoke<Settings>("load_settings"),
      ]);
      setList(versions);
      setSelected((current) => {
        const wanted = current || settings.instance.version;
        return versions.installed.includes(wanted) ? wanted : (versions.installed[0] ?? "");
      });
    } catch (reason) {
      setListError(String(reason));
    }
  }, []);

  useEffect(() => {
    loadVersions();
  }, [loadVersions]);

  useEffect(() => {
    const stopListening = listen<LaunchEvent>("install-event", ({ payload }) => {
      if (payload.kind === "progress") {
        setInstall(
          (current) =>
            current && {
              ...current,
              progress: payload,
              percent: overallProgress(payload),
            },
        );
      }
    });
    return () => {
      stopListening.then((stop) => stop());
    };
  }, []);

  async function installGame(id: string) {
    setInstall({ id, progress: null, percent: 0, error: "" });
    try {
      await invoke("install_game", { version: id });
      setInstall(null);
      await loadVersions();
    } catch (reason) {
      setInstall({ id, progress: null, percent: 0, error: String(reason) });
    }
  }

  function playVersion(id: string) {
    setSelected(id);
    setTab("launch");
  }

  return (
    <div
      className="app"
      style={{ "--scene": TABS.findIndex(({ id }) => id === tab) } as CSSProperties}
    >
      <svg className="hills" viewBox="0 0 800 220" preserveAspectRatio="none" aria-hidden="true">
        <path className="hill-back" d="M0 170C190 130 430 190 800 40V220H0Z" />
        <path className="hill-front" d="M0 196C260 180 520 200 800 110V220H0Z" />
      </svg>
      <TitleBar tab={tab} onTabChange={setTab} />
      {/* 啟動頁一直保留，切換分頁時遊戲狀態不會消失。 */}
      <main className="page" hidden={tab !== "launch"}>
        <LaunchPage
          list={list}
          listError={listError}
          selected={selected}
          onSelect={setSelected}
          onGoToDownload={() => setTab("download")}
          onRetry={loadVersions}
        />
      </main>
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
          <SettingsPage />
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
