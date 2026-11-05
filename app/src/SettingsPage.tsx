import { useEffect, useRef, useState } from "react";
import type { CSSProperties, MouseEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { THEMES, applyTheme } from "./theme";
import type { Instance, LauncherConfig, UpdateInfo } from "./types";

const MEMORY_STEP_MB = 512;
const MIN_MEMORY_MB = 1024;
const MAX_MEMORY_MB = 16384;
const AUTO_MEMORY = 0;
const DEFAULT_MANUAL_MEMORY_MB = 4096;
const MB_PER_GB = 1024;
const MIN_CONCURRENCY = 1;
const MAX_CONCURRENCY = 64;
/** 停止調整多久後自動保存。 */
const SAVE_DELAY_MS = 400;

type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "latest" }
  | { kind: "available"; update: UpdateInfo }
  | { kind: "installing" }
  | { kind: "failed"; message: string };

export function SettingsPage({
  config,
  instance,
  onConfigChange,
  onInstanceChange,
}: {
  config: LauncherConfig;
  instance: Instance | null;
  onConfigChange: (config: LauncherConfig) => void;
  onInstanceChange: (instance: Instance) => void;
}) {
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  const [totalMemory, setTotalMemory] = useState(0);
  const [update, setUpdate] = useState<UpdateState>({ kind: "idle" });
  const saveTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    invoke<number>("system_memory_mb").then(setTotalMemory);
  }, []);

  function queueSave(save: () => Promise<unknown>) {
    setSaved(false);
    clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => {
      save()
        .then(() => {
          setError("");
          setSaved(true);
        })
        .catch((reason) => setError(String(reason)));
    }, SAVE_DELAY_MS);
  }

  function changeConfig(change: Partial<LauncherConfig>) {
    const next = { ...config, ...change };
    onConfigChange(next);
    queueSave(() => invoke("save_config", { config: next }));
  }

  function changeInstance(change: Partial<Instance>) {
    if (!instance) {
      return;
    }
    const next = { ...instance, ...change };
    onInstanceChange(next);
    queueSave(() => invoke("save_instance", { instance: next }));
  }

  function chooseTheme(id: string, event: MouseEvent) {
    applyTheme(id, { x: event.clientX, y: event.clientY });
    changeConfig({ theme: id });
  }

  async function checkUpdate() {
    setUpdate({ kind: "checking" });
    try {
      const found = await invoke<UpdateInfo | null>("check_update");
      setUpdate(found ? { kind: "available", update: found } : { kind: "latest" });
    } catch (reason) {
      setUpdate({ kind: "failed", message: String(reason) });
    }
  }

  async function installUpdate(info: UpdateInfo) {
    setUpdate({ kind: "installing" });
    try {
      await invoke("apply_update", { update: info });
    } catch (reason) {
      setUpdate({ kind: "failed", message: String(reason) });
    }
  }

  const memoryMb = instance?.memoryMb ?? AUTO_MEMORY;
  const autoMemory = memoryMb === AUTO_MEMORY;

  return (
    <div className="settings">
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
              onClick={(event) => chooseTheme(id, event)}
            >
              <span className="theme-leaf" style={{ background: color }} />
              {name}
            </button>
          ))}
        </div>
      </section>

      <section className="panel setting">
        <div className="setting-head">
          <label htmlFor="memory">遊戲記憶體</label>
          <output htmlFor="memory">
            {autoMemory ? "自動" : `${(memoryMb / MB_PER_GB).toFixed(1)} GB`}
          </output>
        </div>
        <label className="toggle-row">
          <button
            className="switch"
            role="switch"
            aria-checked={autoMemory}
            aria-label="自動分配記憶體"
            disabled={!instance}
            onClick={() =>
              changeInstance({ memoryMb: autoMemory ? DEFAULT_MANUAL_MEMORY_MB : AUTO_MEMORY })
            }
          />
          <span>自動分配</span>
        </label>
        <input
          id="memory"
          type="range"
          min={MIN_MEMORY_MB}
          max={MAX_MEMORY_MB}
          step={MEMORY_STEP_MB}
          disabled={autoMemory || !instance}
          value={autoMemory ? MIN_MEMORY_MB : memoryMb}
          style={sliderFill(autoMemory ? MIN_MEMORY_MB : memoryMb, MIN_MEMORY_MB, MAX_MEMORY_MB)}
          onChange={(event) => changeInstance({ memoryMb: Number(event.target.value) })}
        />
        <p>
          {instance ? `套用到實例「${instance.name}」。` : "建立實例後可以調整。"}
          {totalMemory > 0 && `電腦共有 ${(totalMemory / MB_PER_GB).toFixed(0)} GB。`}
        </p>
      </section>

      <section className="panel setting">
        <div className="setting-head">
          <label htmlFor="concurrency">同時下載的檔案數</label>
          <output htmlFor="concurrency">{config.downloadConcurrency}</output>
        </div>
        <input
          id="concurrency"
          type="range"
          min={MIN_CONCURRENCY}
          max={MAX_CONCURRENCY}
          value={config.downloadConcurrency}
          style={sliderFill(config.downloadConcurrency, MIN_CONCURRENCY, MAX_CONCURRENCY)}
          onChange={(event) => changeConfig({ downloadConcurrency: Number(event.target.value) })}
        />
        <p>網絡不穩定時可以調低。</p>
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
          disabled={!instance}
          value={instance?.javaArgs ?? ""}
          onChange={(event) => changeInstance({ javaArgs: event.target.value })}
        />
      </section>

      <section className="panel setting">
        <div className="setting-head">
          <span>啟動器更新</span>
        </div>
        <UpdateControls state={update} onCheck={checkUpdate} onInstall={installUpdate} />
      </section>

      <p className={error ? "error setting-wide" : "status setting-wide"}>
        {error || (saved ? "已保存。" : "設定會自動保存。")}
      </p>
    </div>
  );
}

function UpdateControls({
  state,
  onCheck,
  onInstall,
}: {
  state: UpdateState;
  onCheck: () => void;
  onInstall: (update: UpdateInfo) => void;
}) {
  if (state.kind === "available") {
    return (
      <button className="secondary strong" onClick={() => onInstall(state.update)}>
        更新到 {state.update.version}
      </button>
    );
  }
  const busy = state.kind === "checking" || state.kind === "installing";
  return (
    <>
      <button className="secondary" disabled={busy} onClick={onCheck}>
        {state.kind === "installing"
          ? "正在更新"
          : state.kind === "checking"
            ? "正在檢查"
            : "檢查更新"}
      </button>
      {state.kind === "latest" && <p>已經是最新版本。</p>}
      {state.kind === "failed" && <p className="error">{state.message}</p>}
    </>
  );
}

function sliderFill(value: number, min: number, max: number) {
  return { "--fill": `${((value - min) / (max - min)) * 100}%` } as CSSProperties;
}
