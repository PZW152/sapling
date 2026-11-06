import { useCallback, useEffect, useState } from "react";
import type { CSSProperties } from "react";
import { api } from "../../shell/api";
import type { Feature, HeroPanelProps } from "../../shell/feature";
import type { World } from "../../shell/types";
import { canEnterWorld, iconUrl, playedAgo } from "./worlds";
import "./recent-worlds.css";

function RecentWorlds({ instance, busy, canStart, onPlay }: HeroPanelProps) {
  const [worlds, setWorlds] = useState<World[]>([]);
  const supported = canEnterWorld(instance.version);

  // 切換實例或玩完一局後，世界清單都可能變。
  useEffect(() => {
    if (!supported || busy) {
      return;
    }
    let current = true;
    api
      .recentWorlds(instance.id)
      .catch(() => [])
      .then((found) => {
        if (current) {
          setWorlds(found);
        }
      });
    return () => {
      current = false;
    };
  }, [instance.id, supported, busy]);

  if (!supported || worlds.length === 0) {
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
      </ul>
    </section>
  );
}

/** 實例頁的世界清單：備份後變成另一個世界，要還原時在遊戲裡打開它。 */
function WorldList({ instanceId }: { instanceId: string }) {
  const [worlds, setWorlds] = useState<World[]>([]);
  const [working, setWorking] = useState("");
  const [message, setMessage] = useState("");

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

  if (worlds.length === 0) {
    return null;
  }

  const now = Date.now();

  return (
    <>
      <div className="mods-head">
        <h3>世界</h3>
      </div>
      <ul className="world-list">
        {worlds.map((world) => (
          <li key={world.folder}>
            <span className="mod-name">{world.folder}</span>
            <small>{playedAgo(world.lastPlayedMs, now)}</small>
            <button
              className="text-button"
              disabled={working !== ""}
              onClick={() => backup(world.folder)}
            >
              {working === world.folder ? "正在備份" : "備份"}
            </button>
          </li>
        ))}
      </ul>
      {message && <p className="hint">{message}</p>}
    </>
  );
}

export const recentWorlds: Feature = {
  id: "recent-worlds",
  name: "最近遊玩",
  description: "在首頁顯示最近玩過的世界，按一下直接進入，並可以備份世界",
  optional: true,
  HeroPanel: RecentWorlds,
  InstancePanel: WorldList,
};
