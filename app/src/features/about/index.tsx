import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../../shell/AppContext";
import type { Feature } from "../../shell/feature";

const REPOSITORY_URL = "https://github.com/PZW152/sapling";
const USAGE_URL = `${REPOSITORY_URL}/blob/main/USAGE.md`;
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

  return (
    <div className="notice">
      <div className="notice-card about">
        <img src="/logo.svg" alt="" width={60} height={60} />
        <h2>Sapling Launcher</h2>
        <p className="version-text" onClick={clickVersion}>
          版本 {version}
        </p>
        {message && <p className="status">{message}</p>}
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

export const about: Feature = {
  id: "about",
  name: "關於",
  description: "版本資訊和連結",
  optional: false,
  tab: { label: "更多", Page: AboutPage },
};
