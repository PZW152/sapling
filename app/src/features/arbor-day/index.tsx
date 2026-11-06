import { useEffect, useState } from "react";
import type { MouseEvent } from "react";
import { revealChange } from "../../shell/applyTheme";
import { useApp } from "../../shell/AppContext";
import type { Feature } from "../../shell/feature";
import { isArborDay, wearsZhongshan } from "./day";
import { BloomProgress, PlumBlossom } from "./PlumBlossom";
import "./arbor-day.css";

const FESTIVAL = "zhongshan";

function setFestival(active: boolean) {
  const root = document.documentElement;
  if (active) {
    root.dataset.festival = FESTIVAL;
  } else {
    delete root.dataset.festival;
  }
}

function ArborDayNote() {
  const { config, updateConfig } = useApp();
  const [open, setOpen] = useState(true);
  const today = new Date();
  const year = today.getFullYear();
  const planted = config.plantedYears.includes(year);
  const wearing = wearsZhongshan(config.plantedYears, today);

  // 樣式疊在原本的主題上，不改用戶選的主題，所以隔天或關閉模塊時會自動恢復。
  useEffect(() => {
    setFestival(wearing);
    return () => setFestival(false);
  }, [wearing]);

  function plant(event: MouseEvent) {
    revealChange(() => setFestival(true), { x: event.clientX, y: event.clientY });
    updateConfig({ plantedYears: [...config.plantedYears, year] });
  }

  if (!open || !isArborDay(today)) {
    return null;
  }

  return (
    <aside className="arbor-note">
      <span className="arbor-leaf" aria-hidden="true" />
      {planted ? (
        <div>
          <strong>{year} 年的樹種下了</strong>
          <p>今天換上中山主題，紀念國父孫中山先生。明天會恢復原樣。</p>
        </div>
      ) : (
        <div>
          <strong>今天是植樹節</strong>
          <p>種下今年的樹，紀念國父孫中山先生。</p>
        </div>
      )}
      {planted ? (
        <button className="text-button" onClick={() => setOpen(false)}>
          知道了
        </button>
      ) : (
        <button className="text-button" onClick={plant}>
          種樹
        </button>
      )}
    </aside>
  );
}

/** 主區塊右上角的題字和梅花。 */
function Motto() {
  const { config } = useApp();

  if (!wearsZhongshan(config.plantedYears, new Date())) {
    return null;
  }

  return (
    <div className="motto" aria-hidden="true">
      <span>天下為公</span>
      <PlumBlossom />
    </div>
  );
}

export const arborDay: Feature = {
  id: "arbor-day",
  name: "植樹節",
  description: "3 月 12 日的限定彩蛋：種下樹後，當天換上中山主題",
  optional: true,
  Overlay: ArborDayNote,
  HeroPanel: Motto,
  launchProgress: ({ plantedYears }) =>
    wearsZhongshan(plantedYears, new Date()) ? BloomProgress : null,
};
