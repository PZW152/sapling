import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { Slider } from "../../shell/Slider";

const MEMORY_STEP_MB = 512;
const MIN_MEMORY_MB = 1024;
const MAX_MEMORY_MB = 16384;
const AUTO_MEMORY = 0;
const DEFAULT_MANUAL_MEMORY_MB = 4096;
const MB_PER_GB = 1024;
const MAX_WINDOW_SIZE = 7680;

/** 只留下數字。空的代表用遊戲的預設值。 */
function windowSize(text: string) {
  const number = Number.parseInt(text, 10);
  return Number.isFinite(number) ? Math.min(Math.max(number, 0), MAX_WINDOW_SIZE) : 0;
}

/** 實例頁的設定分頁：這個實例怎麼啟動。 */
export function LaunchSettings() {
  const { selected, updateInstance } = useApp();
  const [totalMemory, setTotalMemory] = useState(0);

  useEffect(() => {
    api.systemMemoryMb().then(setTotalMemory);
  }, []);

  const memoryMb = selected?.memoryMb ?? AUTO_MEMORY;
  const autoMemory = memoryMb === AUTO_MEMORY;
  const fullscreen = selected?.fullscreen ?? false;

  async function chooseJava() {
    const path = await open({
      title: "選擇 java.exe",
      filters: [{ name: "Java", extensions: ["exe"] }],
    });
    if (path) {
      updateInstance({ javaPath: path });
    }
  }

  return (
    <div className="instance-settings">
      <section className="setting">
        <div className="setting-head">
          <label htmlFor="memory">遊戲記憶體</label>
          <output htmlFor="memory">
            {autoMemory ? "自動" : `${(memoryMb / MB_PER_GB).toFixed(1)} GB`}
          </output>
        </div>
        <div className="toggle-row">
          <button
            className="switch"
            role="switch"
            aria-checked={autoMemory}
            aria-label="自動分配記憶體"
            disabled={!selected}
            onClick={() =>
              updateInstance({ memoryMb: autoMemory ? DEFAULT_MANUAL_MEMORY_MB : AUTO_MEMORY })
            }
          />
          <span>自動分配</span>
        </div>
        <Slider
          id="memory"
          min={MIN_MEMORY_MB}
          max={MAX_MEMORY_MB}
          step={MEMORY_STEP_MB}
          disabled={autoMemory || !selected}
          value={autoMemory ? MIN_MEMORY_MB : memoryMb}
          onChange={(value) => updateInstance({ memoryMb: value })}
        />
        <p>{totalMemory > 0 && `電腦共有 ${(totalMemory / MB_PER_GB).toFixed(0)} GB。`}</p>
      </section>

      <section className="setting">
        <div className="setting-head">
          <span>遊戲窗口</span>
        </div>
        <div className="toggle-row">
          <button
            className="switch"
            role="switch"
            aria-checked={fullscreen}
            aria-label="全螢幕"
            disabled={!selected}
            onClick={() => updateInstance({ fullscreen: !fullscreen })}
          />
          <span>全螢幕</span>
        </div>
        {!fullscreen && (
          <div className="window-size">
            <input
              type="text"
              inputMode="numeric"
              aria-label="窗口寬度"
              placeholder="寬度"
              value={selected?.windowWidth || ""}
              onChange={(event) => updateInstance({ windowWidth: windowSize(event.target.value) })}
            />
            <span aria-hidden="true">×</span>
            <input
              type="text"
              inputMode="numeric"
              aria-label="窗口高度"
              placeholder="高度"
              value={selected?.windowHeight || ""}
              onChange={(event) => updateInstance({ windowHeight: windowSize(event.target.value) })}
            />
          </div>
        )}
        <p>留空就用遊戲的預設大小。</p>
      </section>

      <section className="setting">
        <div className="setting-head">
          <label htmlFor="join-server">自動加入伺服器</label>
        </div>
        <input
          id="join-server"
          type="text"
          spellCheck={false}
          placeholder="例如 play.example.com:25565"
          value={selected?.joinServer ?? ""}
          onChange={(event) => updateInstance({ joinServer: event.target.value })}
        />
        <p>遊戲開始後直接進入這個伺服器。從「繼續玩」進世界時不會加入。</p>
      </section>

      <section className="setting">
        <div className="setting-head">
          <label htmlFor="java-path">Java</label>
          {selected?.javaPath && (
            <button className="text-button" onClick={() => updateInstance({ javaPath: "" })}>
              改回自動
            </button>
          )}
        </div>
        <div className="java-path">
          <input
            id="java-path"
            type="text"
            spellCheck={false}
            placeholder="自動使用適合這個版本的 Java"
            value={selected?.javaPath ?? ""}
            onChange={(event) => updateInstance({ javaPath: event.target.value })}
          />
          <button className="secondary" onClick={chooseJava}>
            選擇
          </button>
        </div>
      </section>

      <section className="setting">
        <div className="setting-head">
          <label htmlFor="java-args">額外 Java 參數</label>
        </div>
        <input
          id="java-args"
          type="text"
          spellCheck={false}
          placeholder="不確定的話留空"
          disabled={!selected}
          value={selected?.javaArgs ?? ""}
          onChange={(event) => updateInstance({ javaArgs: event.target.value })}
        />
      </section>
    </div>
  );
}
