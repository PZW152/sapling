import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "./types";

const MEMORY_STEP_MB = 512;
const MIN_MEMORY_MB = 1024;
const MAX_MEMORY_MB = 16384;
const MB_PER_GB = 1024;
const MIN_CONCURRENCY = 1;
const MAX_CONCURRENCY = 64;
/** 停止調整多久後自動保存。 */
const SAVE_DELAY_MS = 400;

type SaveState = "idle" | "saving" | "saved" | "failed";

export function SettingsPage() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [error, setError] = useState("");
  const changed = useRef(false);

  useEffect(() => {
    invoke<Settings>("load_settings")
      .then(setSettings)
      .catch((reason) => setError(String(reason)));
  }, []);

  useEffect(() => {
    if (!changed.current) {
      return;
    }
    const timer = setTimeout(() => {
      invoke("save_settings", { settings })
        .then(() => setSaveState("saved"))
        .catch((reason) => {
          setError(String(reason));
          setSaveState("failed");
        });
    }, SAVE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [settings]);

  function update(change: (current: Settings) => Settings) {
    changed.current = true;
    setSaveState("saving");
    setSettings((current) => current && change(current));
  }

  function updateInstance(change: Partial<Settings["instance"]>) {
    update((current) => ({ ...current, instance: { ...current.instance, ...change } }));
  }

  if (!settings) {
    return (
      <div className="notice">
        <p className={error ? "error" : undefined}>{error || "正在讀取設定"}</p>
      </div>
    );
  }

  const memoryMb = settings.instance.memoryMb;
  const concurrency = settings.config.downloadConcurrency;

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
          style={sliderFill(memoryMb, MIN_MEMORY_MB, MAX_MEMORY_MB)}
          onChange={(event) => updateInstance({ memoryMb: Number(event.target.value) })}
        />
        <p>一般遊玩 4 GB 就夠。不要超過電腦實體記憶體的一半。</p>
      </section>

      <section className="panel setting">
        <div className="setting-head">
          <label htmlFor="concurrency">同時下載的檔案數</label>
          <output htmlFor="concurrency">{concurrency}</output>
        </div>
        <input
          id="concurrency"
          type="range"
          min={MIN_CONCURRENCY}
          max={MAX_CONCURRENCY}
          value={concurrency}
          style={sliderFill(concurrency, MIN_CONCURRENCY, MAX_CONCURRENCY)}
          onChange={(event) =>
            update((current) => ({
              ...current,
              config: { downloadConcurrency: Number(event.target.value) },
            }))
          }
        />
        <p>網絡不穩定時可以調低。</p>
      </section>

      <section className="panel setting setting-wide">
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

      <p className={saveState === "failed" ? "error setting-wide" : "status setting-wide"}>
        {saveMessage(saveState, error)}
      </p>
    </div>
  );
}

function sliderFill(value: number, min: number, max: number) {
  return { "--fill": `${((value - min) / (max - min)) * 100}%` } as CSSProperties;
}

function saveMessage(state: SaveState, error: string) {
  if (state === "failed") {
    return error;
  }
  if (state === "saving") {
    return "正在保存";
  }
  return state === "saved" ? "已保存，下次啟動遊戲時生效。" : "設定會自動保存。";
}
