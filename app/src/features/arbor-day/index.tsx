import { useState } from "react";
import { useApp } from "../../shell/AppContext";
import type { Feature } from "../../shell/feature";

const MARCH = 2;
const ARBOR_DAY = 12;

/** 種過樹後解鎖的隱藏主題。 */
export const HIDDEN_THEME = "zhongshan";

/** 3 月 12 日：植樹節，也是國父孫中山先生的逝世紀念日。 */
export function isArborDay(date: Date) {
  return date.getMonth() === MARCH && date.getDate() === ARBOR_DAY;
}

function ArborDayNote() {
  const { config } = useApp();
  const [open, setOpen] = useState(true);
  const today = new Date();
  const planted = config.plantedYears.includes(today.getFullYear());

  if (!open || !isArborDay(today)) {
    return null;
  }

  return (
    <aside className="arbor-note">
      <span className="arbor-leaf" aria-hidden="true" />
      {planted ? (
        <div>
          <strong>{today.getFullYear()} 年的樹種下了</strong>
          <p>梅花開了。隱藏主題「中山」已解鎖，紀念國父孫中山先生。</p>
        </div>
      ) : (
        <div>
          <strong>今天是植樹節</strong>
          <p>點一下樹苗，種下今年的樹，紀念國父孫中山先生。</p>
        </div>
      )}
      <button className="text-button" onClick={() => setOpen(false)}>
        知道了
      </button>
    </aside>
  );
}

export const arborDay: Feature = {
  id: "arbor-day",
  name: "植樹節",
  description: "3 月 12 日的限定彩蛋",
  optional: false,
  Overlay: ArborDayNote,
};
