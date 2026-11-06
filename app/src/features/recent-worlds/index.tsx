import { useEffect, useState } from "react";
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

export const recentWorlds: Feature = {
  id: "recent-worlds",
  name: "最近遊玩",
  description: "在首頁顯示最近玩過的世界，按一下直接進入",
  optional: true,
  HeroPanel: RecentWorlds,
};
