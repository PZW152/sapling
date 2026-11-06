import { useCallback, useEffect, useState } from "react";
import { api } from "../../shell/api";
import type { ModAction } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import type { ModFile } from "../../shell/types";
import { ModBrowser } from "./ModBrowser";

const BYTES_PER_MB = 1024 * 1024;

export function ModList({ instanceId }: { instanceId: string }) {
  const [mods, setMods] = useState<ModFile[]>([]);
  const [error, setError] = useState("");
  const [browsing, setBrowsing] = useState(false);
  const { selected } = useApp();
  const vanilla = selected?.loader === "vanilla";

  const load = useCallback(() => {
    api.listMods(instanceId)
      .then(setMods)
      .catch((reason) => setError(String(reason)));
  }, [instanceId]);

  useEffect(load, [load]);

  function act(
    request: (action: ModAction) => Promise<void>,
    file: ModFile,
    enabled = file.enabled,
  ) {
    setError("");
    request({ instanceId, name: file.name, enabled })
      .then(load)
      .catch((reason) => setError(String(reason)));
  }

  if (browsing && selected) {
    return (
      <ModBrowser
        instance={selected}
        onClose={() => {
          setBrowsing(false);
          load();
        }}
      />
    );
  }

  return (
    <>
      <div className="mods-head">
        <h3>模組</h3>
        <div className="mods-actions">
          {!vanilla && (
            <button className="text-button" onClick={() => setBrowsing(true)}>
              下載模組
            </button>
          )}
          <button
            className="text-button"
            onClick={() =>
              api.openModsFolder(instanceId).catch((reason) => setError(String(reason)))
            }
          >
            打開資料夾
          </button>
        </div>
      </div>
      <ul className="mods">
        {mods.length === 0 && (
          <li className="mods-empty">
            {vanilla
              ? "原版不會載入模組。想用模組，請新增一個使用載入器的實例。"
              : "按「下載模組」搜尋，或把 .jar 放進模組資料夾。"}
          </li>
        )}
        {mods.map((file) => (
          <li key={file.name} className={file.enabled ? "mod" : "mod off"}>
            <button
              className="switch"
              role="switch"
              aria-checked={file.enabled}
              aria-label={`${file.enabled ? "停用" : "啟用"} ${file.name}`}
              onClick={() => act(api.setModEnabled, file, !file.enabled)}
            />
            <span className="mod-name">{file.name}</span>
            <small>{(file.size / BYTES_PER_MB).toFixed(1)} MB</small>
            <button className="text-button" onClick={() => act(api.deleteMod, file)}>
              刪除
            </button>
          </li>
        ))}
      </ul>
      {error && <p className="error">{error}</p>}
    </>
  );
}
