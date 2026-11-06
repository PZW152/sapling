import type { MouseEvent } from "react";
import { applyScale } from "../../shell/applyScale";
import { applyTheme } from "../../shell/applyTheme";
import { useApp } from "../../shell/AppContext";
import type { Feature } from "../../shell/feature";
import { THEMES } from "./theme";
import "./appearance.css";

const SCALES = [
  { percent: 100, name: "標準" },
  { percent: 115, name: "大" },
  { percent: 130, name: "特大" },
];

function AppearanceSettings() {
  const { config, updateConfig } = useApp();

  function choose(id: string, event: MouseEvent) {
    applyTheme(id, { x: event.clientX, y: event.clientY });
    updateConfig({ theme: id });
  }

  function chooseScale(percent: number) {
    applyScale(percent);
    updateConfig({ uiScale: percent });
  }

  return (
    <section className="panel setting setting-wide">
      <div className="setting-head">
        <span>主題</span>
      </div>
      <div className="themes">
        {THEMES.map(({ id, name, color }) => (
          <button
            key={id}
            className="theme"
            aria-pressed={config.theme === id}
            onClick={(event) => choose(id, event)}
          >
            <span className="theme-leaf" style={{ background: color }} />
            {name}
          </button>
        ))}
      </div>
      <div className="setting-head">
        <span>介面大小</span>
      </div>
      <div className="chips" role="group" aria-label="介面大小">
        {SCALES.map(({ percent, name }) => (
          <button
            key={percent}
            className="chip"
            aria-pressed={config.uiScale === percent}
            onClick={() => chooseScale(percent)}
          >
            {name}
          </button>
        ))}
      </div>
    </section>
  );
}

export const appearance: Feature = {
  id: "appearance",
  name: "外觀",
  description: "預設綠色主題",
  optional: false,
  Settings: AppearanceSettings,
};
