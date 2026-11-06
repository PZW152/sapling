import { useEffect, useState } from "react";
import { api } from "../../shell/api";
import type { Instance, ModHit } from "../../shell/types";
import { formatDownloads } from "./downloads";

/** 停止輸入多久後才搜尋，避免每打一個字都發請求。 */
const SEARCH_DELAY_MS = 350;

/** 從 Modrinth 搜尋並安裝適合這個實例的模組。 */
export function ModBrowser({
  instance,
  onClose,
}: {
  instance: Instance;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<ModHit[]>([]);
  const [searching, setSearching] = useState(true);
  const [installing, setInstalling] = useState("");
  const [installed, setInstalled] = useState<string[]>([]);
  const [error, setError] = useState("");

  useEffect(() => {
    let current = true;
    setSearching(true);
    const timer = setTimeout(() => {
      api
        .searchMods(query, instance.version, instance.loader)
        .then((found) => {
          if (current) {
            setHits(found);
            setError("");
          }
        })
        .catch((reason) => current && setError(String(reason)))
        .finally(() => current && setSearching(false));
    }, SEARCH_DELAY_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [query, instance.version, instance.loader]);

  async function install(hit: ModHit) {
    setInstalling(hit.projectId);
    setError("");
    try {
      await api.installMod(instance.id, hit.projectId);
      setInstalled((ids) => [...ids, hit.projectId]);
    } catch (reason) {
      setError(String(reason));
    }
    setInstalling("");
  }

  return (
    <div className="mod-browser">
      <div className="mods-head">
        <h3>下載模組</h3>
        <button className="text-button" onClick={onClose}>
          返回模組清單
        </button>
      </div>
      <label className="field">
        <span>
          適合 Minecraft {instance.version} 的模組，需要的前置模組會一起安裝
        </span>
        <input
          autoFocus
          value={query}
          placeholder="搜尋模組名稱"
          spellCheck={false}
          onChange={(event) => setQuery(event.target.value)}
        />
      </label>
      {error && <p className="error">{error}</p>}
      {!error && !searching && hits.length === 0 && (
        <p className="mods-empty">找不到適合這個版本的模組，換個關鍵字試試。</p>
      )}
      <ul className="mod-hits">
        {hits.map((hit) => {
          const done = installed.includes(hit.projectId);
          return (
            <li key={hit.projectId}>
              {hit.iconUrl ? (
                <img src={hit.iconUrl} alt="" loading="lazy" />
              ) : (
                <span className="mod-hit-icon" aria-hidden="true" />
              )}
              <div>
                <strong>{hit.title}</strong>
                <p>{hit.description}</p>
                <small>{formatDownloads(hit.downloads)} 次下載</small>
              </div>
              <button
                className="secondary"
                disabled={done || installing !== ""}
                onClick={() => install(hit)}
              >
                {done ? "已安裝" : installing === hit.projectId ? "正在安裝" : "安裝"}
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
