import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { api } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import type { Feature } from "../../shell/feature";
import "./about.css";

const REPOSITORY_URL = "https://github.com/PZW152/sapling";
const USAGE_URL = `${REPOSITORY_URL}/blob/main/USAGE.md`;
const LICENSES_URL = `${REPOSITORY_URL}/blob/main/THIRD-PARTY-NOTICES.md`;
const ISSUES_URL = `${REPOSITORY_URL}/issues/new`;
/** 連按版本號這麼多次，切換功能模塊開關的顯示。 */
const UNLOCK_CLICKS = 5;

function AboutPage() {
  const { config, updateConfig } = useApp();
  const [version, setVersion] = useState("");
  const [clicks, setClicks] = useState(0);
  const [message, setMessage] = useState("");

  function clickVersion() {
    if (clicks + 1 < UNLOCK_CLICKS) {
      setClicks(clicks + 1);
      return;
    }
    const unlocked = !config.featureSwitchesUnlocked;
    updateConfig({ featureSwitchesUnlocked: unlocked });
    setMessage(unlocked ? "功能模塊開關已顯示在設定頁。" : "功能模塊開關已隱藏。");
    setClicks(0);
  }

  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  async function copyDiagnostics() {
    await navigator.clipboard.writeText(await api.diagnostics());
    setMessage("診斷資訊已複製，回報問題時貼上。裡面沒有帳號資料。");
  }

  return (
    <div className="notice">
      <div className="notice-card about">
        <img src="/logo.svg" alt="" width={60} height={60} />
        <h2>Sapling Launcher</h2>
        <p className="version-text" onClick={clickVersion}>
          版本 {version}
        </p>
        {message && <p className="status">{message}</p>}
        {config.plantedYears.length > 0 && (
          <p className="status">你已經種下 {config.plantedYears.length} 棵樹。</p>
        )}
        <div className="about-links">
          <button className="secondary" onClick={() => api.openLink(REPOSITORY_URL)}>
            GitHub 主頁
          </button>
          <button className="secondary" onClick={() => api.openLink(USAGE_URL)}>
            使用指南
          </button>
        </div>
        <div className="about-more">
          <button className="text-button" onClick={() => api.openLink(ISSUES_URL)}>
            回報問題
          </button>
          <button className="text-button" onClick={copyDiagnostics}>
            複製診斷資訊
          </button>
          <button className="text-button" onClick={() => api.openDataFolder()}>
            打開資料資料夾
          </button>
          <button className="text-button" onClick={() => api.openLink(LICENSES_URL)}>
            第三方授權
          </button>
        </div>
        <p className="about-note">非 Minecraft 官方產品，與 Mojang 和 Microsoft 無關。</p>
      </div>
    </div>
  );
}

export const about: Feature = {
  id: "about",
  name: "關於",
  description: "版本資訊和連結",
  optional: false,
  tab: { label: "更多", Page: AboutPage, pinned: true },
};
