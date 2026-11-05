import { useState } from "react";

/** 實例越多，莖上多長出的小葉子：位置、大小和方向。 */
const EXTRA_LEAVES = [
  { y: 196, size: 0.42, flip: false },
  { y: 184, size: 0.36, flip: true },
  { y: 172, size: 0.3, flip: false },
  { y: 138, size: 0.26, flip: false },
];
const EXTRA_LEAF_PATH = "M0 0 C0 -46 32 -76 76 -80 C76 -32 46 0 0 0 Z";
/** 種樹後開出的梅花位置和大小。 */
const BLOSSOMS = [
  { x: 176, y: 34, size: 1 },
  { x: 150, y: 58, size: 0.8 },
  { x: 126, y: 84, size: 0.7 },
  { x: 100, y: 112, size: 0.9 },
  { x: 58, y: 118, size: 0.75 },
  { x: 30, y: 104, size: 0.95 },
  { x: 104, y: 176, size: 0.6 },
];
const PETAL_ANGLES = [0, 72, 144, 216, 288];

/**
 * 首頁的樹苗。`growth` 由 0 到 1，下載時就是它長高的過程。
 * 實例越多，葉子越多。
 */
export function Sapling({
  growth,
  leaves,
  celebrate,
  blossoms = false,
  onPlant,
}: {
  growth: number;
  leaves: number;
  celebrate: boolean;
  blossoms?: boolean;
  /** 植樹節當天點擊樹苗時呼叫。 */
  onPlant?: () => void;
}) {
  // 開場動畫只在打開啟動器時播放一次，之後由 growth 控制。
  const [intro, setIntro] = useState(true);
  // 葉子要等莖長到它的位置才開始長，否則會浮在半空。
  const leftLeaf = stageOf(growth, 0.36, 0.72);
  const rightLeaf = stageOf(growth, 0.6, 1);
  const extraLeaves = growth === 1 ? EXTRA_LEAVES.slice(0, Math.max(leaves - 1, 0)) : [];

  return (
    <svg
      className={`sapling${intro ? " intro" : ""}${celebrate ? " celebrate" : ""}${
        onPlant ? " plantable" : ""
      }`}
      viewBox="0 0 200 240"
      aria-hidden="true"
      onClick={onPlant}
      onAnimationEnd={(event) => {
        // 右葉最後長完，它結束才算開場動畫完成。
        if ((event.target as Element).classList.contains("sapling-leaf-right")) {
          setIntro(false);
        }
      }}
    >
      <defs>
        <linearGradient id="leaf-main" x1="0" y1="1" x2="1" y2="0">
          <stop className="leaf-stop-deep" offset="0" />
          <stop className="leaf-stop-light" offset="1" />
        </linearGradient>
        <linearGradient id="leaf-soft" x1="1" y1="1" x2="0" y2="0">
          <stop className="leaf-stop-main" offset="0" />
          <stop className="leaf-stop-pale" offset="1" />
        </linearGradient>
      </defs>
      <g className="sapling-sway">
        <path
          className="sapling-stem"
          d="M100 240 C100 200 100 150 100 114"
          pathLength={1}
          style={{ strokeDashoffset: 1 - stageOf(growth, 0, 0.6) }}
        />
        {extraLeaves.map(({ y, size, flip }) => (
          <g key={y} transform={`translate(100 ${y}) scale(${flip ? -size : size} ${size})`}>
            <path className="sapling-extra" d={EXTRA_LEAF_PATH} />
          </g>
        ))}
        <g
          className="sapling-leaf sapling-leaf-right"
          style={{ transform: `scale(${rightLeaf})`, opacity: rightLeaf > 0 ? 1 : 0 }}
        >
          <path fill="url(#leaf-main)" d="M100 112 C100 66 132 36 176 32 C176 80 146 112 100 112 Z" />
          <path className="sapling-vein" d="M100 112 C124 92 150 62 172 36" />
        </g>
        <g
          className="sapling-leaf sapling-leaf-left"
          style={{ transform: `scale(${leftLeaf})`, opacity: leftLeaf > 0 ? 1 : 0 }}
        >
          <path fill="url(#leaf-soft)" d="M100 158 C100 122 70 102 28 102 C28 140 58 164 100 158 Z" />
          <path className="sapling-vein" d="M100 158 C78 138 54 118 32 104" />
        </g>
        {blossoms &&
          BLOSSOMS.map(({ x, y, size }, index) => (
            <g key={index} transform={`translate(${x} ${y}) scale(${size})`}>
              <g className="blossom" style={{ animationDelay: `${index * 90}ms` }}>
                {PETAL_ANGLES.map((angle) => (
                  <circle key={angle} className="petal" r="4" cy="-5" transform={`rotate(${angle})`} />
                ))}
                <circle className="blossom-heart" r="2" />
              </g>
            </g>
          ))}
      </g>
    </svg>
  );
}

function stageOf(growth: number, start: number, end: number) {
  return Math.min(1, Math.max(0, (growth - start) / (end - start)));
}
