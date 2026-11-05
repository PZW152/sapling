import { useState } from "react";
import { FEATURES } from "..";
import { useApp } from "../../shell/AppContext";
import type { Feature } from "../../shell/feature";
import "./settings.css";

function SettingsPage() {
  const { config, isEnabled, saveError } = useApp();
  const [advanced, setAdvanced] = useState(false);
  const active = FEATURES.filter((feature) => !feature.optional || isEnabled(feature.id));

  return (
    <div className="settings">
      {active.map(({ id, Settings }) => Settings && <Settings key={id} />)}
      <button
        className="disclosure setting-wide"
        aria-expanded={advanced}
        onClick={() => setAdvanced(!advanced)}
      >
        進階設定
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path d="M2.5 4.5 6 8l3.5-3.5" />
        </svg>
      </button>
      <div className="collapsible setting-wide" data-open={advanced}>
        <div className="collapsible-inner">
          {active.map(
            ({ id, AdvancedSettings }) => AdvancedSettings && <AdvancedSettings key={id} />,
          )}
          {config.featureSwitchesUnlocked && <FeatureSwitches />}
        </div>
      </div>
      <p className={saveError ? "error setting-wide" : "status setting-wide"}>
        {saveError || "設定會自動保存。"}
      </p>
    </div>
  );
}

function FeatureSwitches() {
  const { config, isEnabled, updateConfig } = useApp();

  function toggle(id: string) {
    const disabled = config.disabledFeatures;
    updateConfig({
      disabledFeatures: disabled.includes(id)
        ? disabled.filter((item) => item !== id)
        : [...disabled, id],
    });
  }

  return (
    <section className="panel setting setting-wide">
      <div className="setting-head">
        <span>功能模塊</span>
      </div>
      <ul className="feature-switches">
        {FEATURES.filter((feature) => feature.optional).map(({ id, name, description }) => (
          <li key={id}>
            <button
              className="switch"
              role="switch"
              aria-checked={isEnabled(id)}
              aria-label={name}
              onClick={() => toggle(id)}
            />
            <div>
              <strong>{name}</strong>
              <p>{description}</p>
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}

export const settings: Feature = {
  id: "settings",
  name: "設定",
  description: "調整啟動器和功能模塊",
  optional: false,
  tab: { label: "設定", Page: SettingsPage },
};
