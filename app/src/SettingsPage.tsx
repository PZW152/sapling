import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "./types";

const MEMORY_STEP_MB = 512;
const MIN_MEMORY_MB = 1024;
const MAX_MEMORY_MB = 16384;
const MB_PER_GB = 1024;

type SaveState = "idle" | "saved" | "failed";

export function SettingsPage() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [error, setError] = useState("");

  useEffect(() => {
    invoke<Settings>("load_settings")
      .then(setSettings)
      .catch((reason) => setError(String(reason)));
  }, []);

  function updateInstance(change: Partial<Settings["instance"]>) {
    setSaveState("idle");
    setSettings((current) =>
      current && { ...current, instance: { ...current.instance, ...change } },
    );
  }

  function updateConcurrency(downloadConcurrency: number) {
    setSaveState("idle");
    setSettings((current) => current && { ...current, config: { downloadConcurrency } });
  }

  async function save() {
    setError("");
    try {
      await invoke("save_settings", { settings });
      setSaveState("saved");
    } catch (reason) {
      setError(String(reason));
      setSaveState("failed");
    }
  }

  if (!settings) {
    return (
      <div className="notice">
        <p className={error ? "error" : undefined}>{error || "正在讀取設定"}</p>
      </div>
    );
  }

  const memoryMb = settings.instance.memoryMb;

  return (
    <div className="settings">
      <section className="panel setting">
        <div className="setting-head">
          <label htmlFor="memory">遊戲記憶體</label>
          <output htmlFor="memory">{(memoryMb / MB_PER_GB).toFixed(1)} GB</output>
        </div>
        <input
          id="memory"
          type="range"
          min={MIN_MEMORY_MB}
          max={MAX_MEMORY_MB}
          step={MEMORY_STEP_MB}
          value={memoryMb}
          onChange={(event) => updateInstance({ memoryMb: Number(event.target.value) })}
        />
        <p>一般遊玩 4 GB 就夠。不要超過電腦實體記憶體的一半。</p>
      </section>

      <section className="panel setting">
        <div className="setting-head">
          <label htmlFor="java-args">額外 Java 參數</label>
        </div>
        <input
          id="java-args"
          type="text"
          spellCheck={false}
          placeholder="不確定的話留空"
          value={settings.instance.javaArgs}
          onChange={(event) => updateInstance({ javaArgs: event.target.value })}
        />
      </section>

      <section className="panel setting">
        <div className="setting-head">
          <label htmlFor="concurrency">同時下載的檔案數</label>
          <output htmlFor="concurrency">{settings.config.downloadConcurrency}</output>
        </div>
        <input
          id="concurrency"
          type="range"
          min={1}
          max={64}
          value={settings.config.downloadConcurrency}
          onChange={(event) => updateConcurrency(Number(event.target.value))}
        />
        <p>網絡不穩定時可以調低。</p>
      </section>

      <div className="settings-actions">
        <button className="save-button" onClick={save}>
          保存設定
        </button>
        {saveState === "saved" && <p className="status">已保存，下次啟動遊戲時生效。</p>}
        {saveState === "failed" && <p className="error">{error}</p>}
      </div>
    </div>
  );
}
