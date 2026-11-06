import { useCallback, useEffect, useState } from "react";
import { api } from "../../shell/api";
import type { ModAction } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { ConfirmButton } from "../../shell/ConfirmButton";
import { trackDownload } from "../../shell/downloads";
import { ErrorText } from "../../shell/ErrorText";
import { ResourceBrowser } from "../../shell/resources/ResourceBrowser";
import { displayTitle } from "../../shell/resources/names";
import type { InstalledMod, ModFile } from "../../shell/types";
import { matchesMod, modTitle, updatable } from "./installed";

const BYTES_PER_MB = 1024 * 1024;
/** 模組超過這個數量才顯示搜尋框。 */
const FILTER_FROM = 8;

export function ModList({ instanceId }: { instanceId: string }) {
  const [mods, setMods] = useState<ModFile[]>([]);
  const [known, setKnown] = useState<InstalledMod[]>([]);
  const [checking, setChecking] = useState(false);
  const [updating, setUpdating] = useState<string[]>([]);
  const [filter, setFilter] = useState("");
  const [error, setError] = useState("");
  const [browsing, setBrowsing] = useState(false);
  const { selected } = useApp();

  const load = useCallback(() => {
    api.listMods(instanceId)
      .then(setMods)
      .catch((reason) => setError(String(reason)));
  }, [instanceId]);

  // 要算檔案雜湊又要連線，所以和清單分開：清單先出現，名稱和新版隨後補上。
  // 沒有網路時就只是顯示檔名。
  const inspect = useCallback(() => {
    setChecking(true);
    api.inspectMods(instanceId)
      .then(setKnown)
      .catch(() => setKnown([]))
      .finally(() => setChecking(false));
  }, [instanceId]);

  useEffect(() => {
    load();
    inspect();
  }, [load, inspect]);

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

  async function update(targets: InstalledMod[]) {
    setError("");
    setUpdating(targets.map(({ fileName }) => fileName));
    const failed: string[] = [];
    // 一個一個來：同時裝會搶著處理同一個前置模組。
    for (const mod of targets) {
      try {
        await trackDownload(displayTitle(mod), () =>
          api.installProject(instanceId, "mod", mod.projectId, mod.update?.versionId ?? null),
        );
      } catch (reason) {
        failed.push(String(reason));
      }
      setUpdating((running) => running.filter((name) => name !== mod.fileName));
    }
    setError(failed.join("\n"));
    load();
    inspect();
  }

  if (browsing && selected) {
    return (
      <ResourceBrowser
        kind="mod"
        instance={selected}
        install={async (hit, versionId) => {
          await api.installProject(selected.id, "mod", hit.projectId, versionId);
        }}
        onClose={() => {
          setBrowsing(false);
          load();
          inspect();
        }}
      />
    );
  }

  const info = new Map(known.map((mod) => [mod.fileName, mod]));
  const updates = updatable(mods, known);
  const shown = mods.filter((file) => matchesMod(file, info.get(file.name), filter));

  return (
    <>
      <div className="mods-head">
        <h2 className="sr-only">模組</h2>
        <div className="mods-status" role="status">
          {checking && "正在檢查更新"}
          {!checking && mods.length > 0 && updates.length === 0 && known.length > 0 && "都是最新版"}
        </div>
        <div className="mods-actions">
          {updates.length > 0 && (
            <button
              className="text-button"
              disabled={updating.length > 0}
              onClick={() => update(updates)}
            >
              {updating.length > 0 ? `正在更新，還有 ${updating.length} 個` : `全部更新（${updates.length}）`}
            </button>
          )}
          <button className="text-button" onClick={() => setBrowsing(true)}>
            下載模組
          </button>
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
      {mods.length >= FILTER_FROM && (
        <input
          type="search"
          className="mod-filter"
          aria-label="搜尋已安裝的模組"
          placeholder={`在 ${mods.length} 個模組裡搜尋`}
          spellCheck={false}
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
        />
      )}
      <ErrorText message={error} />
      <ul className="mods">
        {mods.length === 0 && (
          <li className="mods-empty">按「下載模組」搜尋，或把 .jar 放進模組資料夾。</li>
        )}
        {mods.length > 0 && shown.length === 0 && (
          <li className="mods-empty">找不到符合的模組。</li>
        )}
        {shown.map((file) => {
          const mod = info.get(file.name);
          const title = modTitle(file, mod);
          return (
            <li key={file.name} className={file.enabled ? "mod" : "mod off"}>
              <button
                className="switch"
                role="switch"
                aria-checked={file.enabled}
                aria-label={`${file.enabled ? "停用" : "啟用"} ${title}`}
                onClick={() => act(api.setModEnabled, file, !file.enabled)}
              />
              {mod?.iconUrl ? (
                <img className="mod-icon" src={mod.iconUrl} alt="" loading="lazy" />
              ) : (
                <span className="mod-icon" aria-hidden="true" />
              )}
              <span className="mod-text" title={file.name}>
                <span className="mod-name">{title}</span>
                {mod && <small>{file.name}</small>}
              </span>
              <small>{(file.size / BYTES_PER_MB).toFixed(1)} MB</small>
              {mod?.update && (
                <button
                  className="text-button"
                  disabled={updating.includes(file.name)}
                  aria-label={`更新 ${title} 到 ${mod.update.versionName}`}
                  title={`新版：${mod.update.versionName}`}
                  onClick={() => update([mod])}
                >
                  {updating.includes(file.name) ? "更新中" : "更新"}
                </button>
              )}
              <ConfirmButton
                className="text-button"
                label="刪除"
                warning="再按一次刪除"
                subject={title}
                onConfirm={() => act(api.deleteMod, file)}
              />
            </li>
          );
        })}
      </ul>
    </>
  );
}
