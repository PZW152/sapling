import { useApp } from "../../shell/AppContext";
import { Slider } from "../settings/Slider";

const MIN_CONCURRENCY = 1;
const MAX_CONCURRENCY = 64;

export function DownloadSettings() {
  const { config, updateConfig } = useApp();

  return (
    <section className="panel setting">
      <div className="setting-head">
        <label htmlFor="concurrency">同時下載的檔案數</label>
        <output htmlFor="concurrency">{config.downloadConcurrency}</output>
      </div>
      <Slider
        id="concurrency"
        min={MIN_CONCURRENCY}
        max={MAX_CONCURRENCY}
        value={config.downloadConcurrency}
        onChange={(value) => updateConfig({ downloadConcurrency: value })}
      />
      <p>網絡不穩定時可以調低。</p>
    </section>
  );
}
