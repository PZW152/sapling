import { useEffect, useState } from "react";
import { api } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { Slider } from "../../shell/Slider";

const MEMORY_STEP_MB = 512;
const MIN_MEMORY_MB = 1024;
const MAX_MEMORY_MB = 16384;
const AUTO_MEMORY = 0;
const DEFAULT_MANUAL_MEMORY_MB = 4096;
const MB_PER_GB = 1024;

export function LaunchSettings() {
  const { selected, updateInstance } = useApp();
  const [totalMemory, setTotalMemory] = useState(0);

  useEffect(() => {
    api.systemMemoryMb().then(setTotalMemory);
  }, []);

  const memoryMb = selected?.memoryMb ?? AUTO_MEMORY;
  const autoMemory = memoryMb === AUTO_MEMORY;

  return (
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
        <p>
          {totalMemory > 0 && `電腦共有 ${(totalMemory / MB_PER_GB).toFixed(0)} GB。`}
        </p>
    </section>
  );
}
