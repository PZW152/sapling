import type { CSSProperties } from "react";
import { petalOpenings } from "./day";

const PETALS = 5;
const FULL_TURN = 360;
/** 比例來自中華民國國花的標準畫法：五片正圓花瓣，每個方向三條花蕊。 */
const PETAL_DISTANCE = 15.93;
const PETAL_RADIUS = 11.24;
const CENTER_RADIUS = 7.5;
const LONG_STAMEN = 16;
const SHORT_STAMEN_X = 1.8;
const SHORT_STAMEN_Y = 11.05;

function petalStyle(index: number, open: number): CSSProperties {
  const turn = (index * FULL_TURN) / PETALS;
  return {
    transform: `rotate(${turn}deg) scale(${0.3 + open * 0.7})`,
    opacity: open === 0 ? 0.14 : 0.14 + open * 0.86,
  };
}

/** 一朵梅花。`share` 是 0 到 1，花瓣按順序一片片開。 */
export function PlumBlossom({ share = 1 }: { share?: number }) {
  const openings = petalOpenings(share, PETALS);

  return (
    <svg className="plum" viewBox="-29 -29 58 58" aria-hidden="true">
      {/* 先畫全部外框，再蓋上花瓣，相鄰花瓣之間才不會有重疊的線。 */}
      {openings.map((open, index) => (
        <circle
          key={index}
          className="plum-outline"
          style={petalStyle(index, open)}
          cy={-PETAL_DISTANCE}
          r={PETAL_RADIUS}
        />
      ))}
      {openings.map((open, index) => (
        <circle
          key={index}
          className="plum-petal"
          style={petalStyle(index, open)}
          cy={-PETAL_DISTANCE}
          r={PETAL_RADIUS}
        />
      ))}
      {/* 五個圓圍不滿中心，要補上，底色才不會透出來。 */}
      <circle className="plum-petal" r={CENTER_RADIUS} />
      {/* 花蕊在五片花瓣都開完才出現，半開時會露在底色上顯得雜亂。 */}
      {openings.map((_, index) => (
        <g
          key={index}
          className="plum-stamens"
          style={{
            transform: `rotate(${(index * FULL_TURN) / PETALS}deg)`,
            opacity: share >= 1 ? 1 : 0,
          }}
        >
          <line y2={-LONG_STAMEN} />
          <circle cy={-LONG_STAMEN} r="1" />
          <line x2={-SHORT_STAMEN_X} y2={SHORT_STAMEN_Y} />
          <circle cx={-SHORT_STAMEN_X} cy={SHORT_STAMEN_Y} r="0.66" />
          <line x2={SHORT_STAMEN_X} y2={SHORT_STAMEN_Y} />
          <circle cx={SHORT_STAMEN_X} cy={SHORT_STAMEN_Y} r="0.66" />
        </g>
      ))}
      <circle className="plum-heart" r="2.25" />
    </svg>
  );
}

/** 植樹節當天的啟動進度。 */
export function BloomProgress({ share }: { share: number }) {
  return (
    <div className="plum-progress">
      <PlumBlossom share={share} />
    </div>
  );
}
