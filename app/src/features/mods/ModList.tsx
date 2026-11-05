import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ModFile } from "../../shell/types";

const BYTES_PER_MB = 1024 * 1024;

export function ModList({ instanceId }: { instanceId: string }) {
  const [mods, setMods] = useState<ModFile[]>([]);
  const [error, setError] = useState("");

  const load = useCallback(() => {
    invoke<ModFile[]>("list_mods", { id: instanceId })
      .then(setMods)
      .catch((reason) => setError(String(reason)));
  }, [instanceId]);

  useEffect(load, [load]);

  function act(command: string, file: ModFile, enabled = file.enabled) {
    setError("");
    invoke(command, { action: { instanceId, name: file.name, enabled } })
      .then(load)
      .catch((reason) => setError(String(reason)));
  }

  return (
    <>
      <div className="mods-head">
        <h3>模組</h3>
        <button
          className="text-button"
          onClick={() =>
            invoke("open_mods_folder", { id: instanceId }).catch((reason) =>
              setError(String(reason)),
            )
          }
        >
          打開模組資料夾
        </button>
      </div>
      <ul className="mods">
        {mods.length === 0 && (
          <li className="mods-empty">
            把 .jar 放進模組資料夾就會出現在這裡。原版不會載入模組，要等之後支援載入器。
          </li>
        )}
        {mods.map((file) => (
          <li key={file.name} className={file.enabled ? "mod" : "mod off"}>
            <button
              className="switch"
              role="switch"
              aria-checked={file.enabled}
              aria-label={`${file.enabled ? "停用" : "啟用"} ${file.name}`}
              onClick={() => act("set_mod_enabled", file, !file.enabled)}
            />
            <span className="mod-name">{file.name}</span>
            <small>{(file.size / BYTES_PER_MB).toFixed(1)} MB</small>
            <button className="text-button" onClick={() => act("delete_mod", file)}>
              刪除
            </button>
          </li>
        ))}
      </ul>
      {error && <p className="error">{error}</p>}
    </>
  );
}
