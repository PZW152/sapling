import { useApp } from "../../shell/AppContext";

/** 設定頁：影響所有實例的啟動行為。 */
export function LaunchOptions() {
  const { config, updateConfig } = useApp();

  return (
    <section className="panel setting setting-wide">
      <div className="setting-head">
        <span>啟動遊戲後</span>
      </div>
      <div className="toggle-row">
        <button
          className="switch"
          role="switch"
          aria-checked={config.minimizeOnLaunch}
          aria-label="啟動遊戲後收起窗口"
          onClick={() => updateConfig({ minimizeOnLaunch: !config.minimizeOnLaunch })}
        />
        <span>收起啟動器窗口，遊戲關閉後自動回來</span>
      </div>
    </section>
  );
}
