import { FEATURES } from "..";
import { useApp } from "../../shell/AppContext";
import type { Feature } from "../../shell/feature";

function SettingsPage() {
  const { config, isEnabled, saveError } = useApp();
  const sections = FEATURES.filter(
    (feature) => feature.Settings && (!feature.optional || isEnabled(feature.id)),
  );

  return (
    <div className="settings">
      {sections.map(({ id, Settings }) => Settings && <Settings key={id} />)}
      {config.featureSwitchesUnlocked && <FeatureSwitches />}
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
