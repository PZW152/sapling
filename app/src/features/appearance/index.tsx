import type { MouseEvent } from "react";
import { applyScale } from "../../shell/applyScale";
import { applyColorMode, applyMaterial, applyTheme, revealChange } from "../../shell/applyTheme";
import { useApp } from "../../shell/AppContext";
import type { Feature } from "../../shell/feature";
import type { ColorMode, WindowMaterial } from "../../shell/types";
import { THEMES } from "./theme";
import "./appearance.css";

const MODES: { id: ColorMode; name: string }[] = [
  { id: "system", name: "跟隨系統" },
  { id: "light", name: "淺色" },
  { id: "dark", name: "深色" },
];

const MATERIALS: { id: WindowMaterial; name: string; hint: string }[] = [
  { id: "acrylic", name: "毛玻璃", hint: "透出窗口後面的畫面，最通透。" },
  { id: "mica", name: "雲母", hint: "只帶一點桌布的顏色，比較省資源。" },
  { id: "solid", name: "純色", hint: "不透明，最省資源。" },
];

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
        <span>深淺色</span>
      </div>
      <div className="chips" role="group" aria-label="深淺色">
        {MODES.map(({ id, name }) => (
          <button
            key={id}
            className="chip"
            aria-pressed={config.colorMode === id}
            onClick={(event) => {
              const origin = { x: event.clientX, y: event.clientY };
              revealChange(() => applyColorMode(id), origin);
              updateConfig({ colorMode: id });
            }}
          >
            {name}
          </button>
        ))}
      </div>
      <div className="setting-head">
        <span>窗口材質</span>
      </div>
      <div className="chips" role="group" aria-label="窗口材質">
        {MATERIALS.map(({ id, name }) => (
          <button
            key={id}
            className="chip"
            aria-pressed={config.windowMaterial === id}
            onClick={(event) => {
              const origin = { x: event.clientX, y: event.clientY };
              revealChange(() => applyMaterial(id), origin);
              updateConfig({ windowMaterial: id });
            }}
          >
            {name}
          </button>
        ))}
      </div>
      <p className="hint">{MATERIALS.find(({ id }) => id === config.windowMaterial)?.hint}</p>
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
