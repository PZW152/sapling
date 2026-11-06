import { useState } from "react";
import { useApp } from "../../shell/AppContext";
import { ErrorText } from "../../shell/ErrorText";
import { FeatureBoundary } from "../../shell/FeatureBoundary";
import type { Feature } from "../../shell/feature";
import "./settings.css";

function SettingsPage() {
  const { config, features: active, saveError } = useApp();
  const [advanced, setAdvanced] = useState(false);

  return (
    <div className="settings">
      {active.map(
        ({ id, name, Settings }) =>
          Settings && (
            <FeatureBoundary key={id} name={name}>
              <Settings />
            </FeatureBoundary>
          ),
      )}
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
            ({ id, name, AdvancedSettings }) =>
              AdvancedSettings && (
                <FeatureBoundary key={id} name={name}>
                  <AdvancedSettings />
                </FeatureBoundary>
              ),
          )}
          {config.featureSwitchesUnlocked && <FeatureSwitches />}
        </div>
      </div>
      {saveError ? (
        <ErrorText className="setting-wide" message={saveError} />
      ) : (
        <p className="status setting-wide">設定會自動保存。</p>
      )}
    </div>
  );
}

function FeatureSwitches() {
  const { allFeatures, config, isEnabled, updateConfig } = useApp();

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
        {allFeatures.filter((feature) => feature.optional).map(({ id, name, description }) => (
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
