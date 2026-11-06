import { useCallback, useEffect, useState } from "react";
import type { CSSProperties } from "react";
import { api } from "../../shell/api";
import { ConfirmButton } from "../../shell/ConfirmButton";
import type { Feature, HeroPanelProps } from "../../shell/feature";
import type { ImportSource, Server, World } from "../../shell/types";
import { canEnterWorld, iconUrl, playedAgo } from "./worlds";
import "./recent-worlds.css";

/** 首頁最多放幾張卡片。 */
const TILE_LIMIT = 3;

function RecentWorlds({ instance, busy, canStart, onPlay, onJoin }: HeroPanelProps) {
  const [worlds, setWorlds] = useState<World[]>([]);
  const [servers, setServers] = useState<Server[]>([]);
  const supported = canEnterWorld(instance.version);

  // 切換實例或玩完一局後，清單都可能變。
  useEffect(() => {
    if (busy) {
      return;
    }
    let current = true;
    const found = supported ? api.recentWorlds(instance.id).catch(() => []) : Promise.resolve([]);
    Promise.all([found, api.listServers(instance.id).catch(() => [])]).then(([list, saved]) => {
      if (current) {
        setWorlds(list);
        setServers(saved);
      }
    });
    return () => {
      current = false;
    };
  }, [instance.id, supported, busy]);

  const shownServers = servers.slice(0, Math.max(TILE_LIMIT - worlds.length, 0));
  if (worlds.length === 0 && shownServers.length === 0) {
    return null;
  }

  const now = Date.now();

  return (
    <section className="worlds" aria-label="繼續玩">
      <h3>繼續玩</h3>
      <ul key={instance.id}>
        {worlds.map((world, index) => (
          <li key={world.folder} style={{ "--i": index } as CSSProperties}>
            <button
              className="world"
              disabled={busy || !canStart}
              aria-label={`進入世界 ${world.folder}`}
              onClick={() => onPlay(world.folder)}
            >
              {world.icon ? (
                <img className="world-icon" src={iconUrl(world.icon)} alt="" />
              ) : (
                <span className="world-icon" aria-hidden="true">
                  {[...world.folder][0]}
                </span>
              )}
              <span className="world-text">
                <strong>{world.folder}</strong>
                <small>{playedAgo(world.lastPlayedMs, now)}</small>
              </span>
            </button>
          </li>
        ))}
        {shownServers.map((server) => (
          <li key={server.address}>
            <button
              className="world"
              disabled={busy || !canStart}
              aria-label={`加入伺服器 ${server.name}`}
              onClick={() => onJoin(server.address)}
            >
              <span className="world-icon" aria-hidden="true">
                {[...server.name][0]}
              </span>
              <span className="world-text">
                <strong>{server.name}</strong>
                <small>伺服器</small>
              </span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** 實例頁的世界清單：備份後變成另一個世界，要還原時在遊戲裡打開它。 */
function WorldList({ instanceId }: { instanceId: string }) {
  const [worlds, setWorlds] = useState<World[]>([]);
  const [working, setWorking] = useState("");
  const [message, setMessage] = useState("");
  const [importing, setImporting] = useState(false);
  const [sources, setSources] = useState<ImportSource[]>([]);

  const load = useCallback(() => {
    api.listWorlds(instanceId).then(setWorlds).catch((reason) => setMessage(String(reason)));
  }, [instanceId]);

  useEffect(load, [load]);

  async function backup(folder: string) {
    setWorking(folder);
    setMessage("");
    try {
      const name = await api.backupWorld(instanceId, folder);
      setMessage(`已備份為「${name}」，要還原時在遊戲裡打開它。`);
      load();
    } catch (reason) {
      setMessage(String(reason));
    }
    setWorking("");
  }

  async function openImport() {
    setImporting(!importing);
    setMessage("");
    setSources(await api.listImportSources().catch(() => []));
  }

  async function remove(folder: string) {
    setMessage("");
    try {
      await api.deleteWorld(instanceId, folder);
      load();
    } catch (reason) {
      setMessage(String(reason));
    }
  }

  async function importFrom(source: ImportSource, folder: string) {
    setWorking(folder);
    setMessage("");
    try {
      const name = await api.importWorld(instanceId, source.path, folder);
      setMessage(`已匯入「${name}」。用比原本舊的遊戲版本打開可能會出問題。`);
      load();
    } catch (reason) {
      setMessage(String(reason));
    }
    setWorking("");
  }

  const now = Date.now();

  return (
    <>
      <div className="mods-head">
        <h2 className="sr-only">世界</h2>
        <button className="text-button" aria-expanded={importing} onClick={openImport}>
          匯入世界
        </button>
      </div>
      {importing && sources.length === 0 && (
        <p className="hint">找不到其他啟動器的世界。</p>
      )}
      {importing &&
        sources.map((source) => (
          <div key={source.path}>
            <p className="hint">{source.label}</p>
            <ul className="world-list">
              {source.worlds.map((world) => (
                <li key={world.folder}>
                  <span className="mod-name">{world.folder}</span>
                  <small>{playedAgo(world.lastPlayedMs, now)}</small>
                  <button
                    className="text-button"
                    disabled={working !== ""}
                    aria-label={`匯入 ${world.folder}`}
                    onClick={() => importFrom(source, world.folder)}
                  >
                    {working === world.folder ? "正在匯入" : "匯入"}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        ))}
      <ul className="world-list">
        {worlds.map((world) => (
          <li key={world.folder}>
            <span className="mod-name">{world.folder}</span>
            <small>{playedAgo(world.lastPlayedMs, now)}</small>
            <button
              className="text-button"
              disabled={working !== ""}
              aria-label={`備份 ${world.folder}`}
              onClick={() => backup(world.folder)}
            >
              {working === world.folder ? "正在備份" : "備份"}
            </button>
            <ConfirmButton
              className="text-button"
              label="刪除"
              warning="再按一次，不能復原"
              subject={world.folder}
              disabled={working !== ""}
              onConfirm={() => remove(world.folder)}
            />
          </li>
        ))}
      </ul>
      {message && (
        <p className="hint" role="status">
          {message}
        </p>
      )}
    </>
  );
}

export const recentWorlds: Feature = {
  id: "recent-worlds",
  name: "最近遊玩",
  description: "在首頁顯示最近玩過的世界，按一下直接進入，並可以備份世界",
  optional: true,
  HeroPanel: RecentWorlds,
  instancePanel: { label: "世界", order: 2, Panel: WorldList },
};
