import { useEffect, useState } from "react";
import { api } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import type { Feature } from "../../shell/feature";
import type { LauncherConfig, Profile } from "../../shell/types";
import { isValidPlayerName } from "./name";
import "./offline-account.css";

function OfflinePanel({ profile, disabled }: { profile: Profile | null; disabled: boolean }) {
  const { config, updateConfig } = useApp();
  const [available, setAvailable] = useState(false);

  // 登入或登出後帳號狀態會變，要重新確認。
  useEffect(() => {
    api
      .hasSavedAccount()
      .catch(() => false)
      .then((saved) => {
        setAvailable(saved);
        if (!saved && config.offlineEnabled) {
          updateConfig({ offlineEnabled: false });
        }
      });
  }, [profile, config.offlineEnabled, updateConfig]);

  if (!available) {
    return null;
  }

  const name = config.offlineName;
  const invalid = config.offlineEnabled && !isValidPlayerName(name);

  return (
    <section className="panel offline">
      <div className="toggle-row">
        <button
          className="switch"
          role="switch"
          aria-checked={config.offlineEnabled}
          aria-label="使用離線模式"
          disabled={disabled}
          onClick={() => updateConfig({ offlineEnabled: !config.offlineEnabled })}
        />
        <span>離線模式</span>
      </div>
      {config.offlineEnabled && (
        <>
          <input
            type="text"
            aria-label="離線玩家名稱"
            placeholder="玩家名稱"
            spellCheck={false}
            maxLength={16}
            disabled={disabled}
            value={name}
            onChange={(event) => updateConfig({ offlineName: event.target.value })}
          />
          <p className={invalid ? "error" : "hint"}>
            {invalid ? "名稱要 3 到 16 個英文字母、數字或底線。" : "只能玩單人遊戲和區域網路。"}
          </p>
        </>
      )}
    </section>
  );
}

function offlineName({ offlineEnabled, offlineName: name }: LauncherConfig) {
  return offlineEnabled && isValidPlayerName(name) ? name : null;
}

export const offlineAccount: Feature = {
  id: "offline-account",
  name: "離線模式",
  description: "登入過正版帳號後，可以用自訂名稱離線遊玩",
  optional: true,
  AccountPanel: OfflinePanel,
  offlineName,
};
