import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LaunchPage } from "./LaunchPage";
import { SettingsPage } from "./SettingsPage";
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

function App() {
  const [tab, setTab] = useState<Tab>("launch");

  return (
    <div className="app">
      <TitleBar tab={tab} onTabChange={setTab} />
      {/* 啟動頁一直保留，切換分頁時下載和遊戲狀態不會消失。 */}
      <main className="page" hidden={tab !== "launch"}>
        <LaunchPage />
      </main>
      {tab === "download" && (
        <main className="page">
          <div className="notice">
            <div className="notice-card">
              <h2>自動下載</h2>
              <p>在啟動頁選好版本並按「啟動遊戲」，需要的檔案和 Java 會自動下載。</p>
              <p>模組下載之後加入。</p>
            </div>
          </div>
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
