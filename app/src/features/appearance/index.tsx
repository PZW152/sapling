import type { MouseEvent } from "react";
import { useApp } from "../../shell/AppContext";
import type { Feature } from "../../shell/feature";
import { THEMES, applyTheme } from "./theme";

function AppearanceSettings() {
  const { config, updateConfig } = useApp();

  function choose(id: string, event: MouseEvent) {
    applyTheme(id, { x: event.clientX, y: event.clientY });
    updateConfig({ theme: id });
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
