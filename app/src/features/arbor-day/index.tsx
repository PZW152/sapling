import { useState } from "react";
import type { Feature } from "../../shell/feature";

const MARCH = 2;
const ARBOR_DAY = 12;

/** 3 月 12 日：植樹節，也是孫中山先生的逝世紀念日。 */
export function isArborDay(date: Date) {
  return date.getMonth() === MARCH && date.getDate() === ARBOR_DAY;
}

function ArborDayNote() {
  const [open, setOpen] = useState(() => isArborDay(new Date()));

  if (!open) {
    return null;
  }

  return (
    <aside className="arbor-note">
      <span className="arbor-leaf" aria-hidden="true" />
      <div>
        <strong>今天是植樹節</strong>
        <p>紀念國父孫中山先生。樹苗今天長得特別茂盛。</p>
      </div>
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
