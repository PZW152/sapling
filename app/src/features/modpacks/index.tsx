import { useEffect, useState } from "react";
import { api, onInstallEvent } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import type { Feature } from "../../shell/feature";
import type { ModHit } from "../../shell/types";
import { formatDownloads } from "../mods/downloads";
import "./modpacks.css";

/** 停止輸入多久後才搜尋，避免每打一個字都發請求。 */
const SEARCH_DELAY_MS = 350;

function ModpackPage() {
  const { goTo, loadInstances, selectInstance } = useApp();
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<ModHit[]>([]);
  const [searching, setSearching] = useState(true);
  const [installing, setInstalling] = useState("");
  const [files, setFiles] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    let current = true;
    setSearching(true);
    const timer = setTimeout(() => {
      api
        .searchPacks(query)
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
  }, [query]);

  useEffect(
    () =>
      onInstallEvent((payload) => {
        if (payload.kind === "progress") {
          setFiles(`${payload.finishedFiles} / ${payload.totalFiles}`);
        }
      }),
    [],
  );

  async function install(hit: ModHit) {
    setInstalling(hit.projectId);
    setFiles("");
    setError("");
    try {
      const instance = await api.installPack(hit.projectId, hit.title);
      await loadInstances();
      selectInstance(instance.id);
      goTo("launch");
    } catch (reason) {
      setError(String(reason));
    }
    setInstalling("");
  }

  return (
    <section className="panel modpacks">
      <label className="field">
        <span>整合包是別人配好的一整套模組，安裝後會建立新實例。</span>
        <input
          value={query}
          placeholder="搜尋整合包名稱"
          spellCheck={false}
          onChange={(event) => setQuery(event.target.value)}
        />
      </label>
      {error && <p className="error">{error}</p>}
      {!error && !searching && hits.length === 0 && (
        <p className="mods-empty">找不到整合包，換個關鍵字試試。</p>
      )}
      <ul className="mod-hits">
        {hits.map((hit) => (
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
              disabled={installing !== ""}
              onClick={() => install(hit)}
            >
              {installing === hit.projectId ? `正在安裝 ${files}` : "安裝"}
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}

export const modpacks: Feature = {
  id: "modpacks",
  name: "整合包",
  description: "從 Modrinth 安裝別人配好的模組組合",
  optional: true,
  tab: { label: "整合包", Page: ModpackPage, keepAlive: true },
};
