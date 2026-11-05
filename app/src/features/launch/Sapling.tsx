import { useState } from "react";

/** 實例越多，莖上多長出的小葉子：位置、大小和方向。 */
const EXTRA_LEAVES = [
  { y: 196, size: 0.42, flip: false },
  { y: 184, size: 0.36, flip: true },
  { y: 172, size: 0.3, flip: false },
  { y: 138, size: 0.26, flip: false },
];
const EXTRA_LEAF_PATH = "M0 0 C0 -46 32 -76 76 -80 C76 -32 46 0 0 0 Z";

/**
 * 首頁的樹苗。`growth` 由 0 到 1，下載時就是它長高的過程。
 * 實例越多，葉子越多。
 */
export function Sapling({
  growth,
  leaves,
  celebrate,
}: {
  growth: number;
  leaves: number;
  celebrate: boolean;
}) {
  // 開場動畫只在打開啟動器時播放一次，之後由 growth 控制。
  const [intro, setIntro] = useState(true);
  // 葉子要等莖長到它的位置才開始長，否則會浮在半空。
  const leftLeaf = stageOf(growth, 0.36, 0.72);
  const rightLeaf = stageOf(growth, 0.6, 1);
  const extraLeaves = growth === 1 ? EXTRA_LEAVES.slice(0, Math.max(leaves - 1, 0)) : [];

  return (
    <svg
      className={`sapling${intro ? " intro" : ""}${celebrate ? " celebrate" : ""}`}
      viewBox="0 0 200 240"
      aria-hidden="true"
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
      </g>
    </svg>
  );
}

function stageOf(growth: number, start: number, end: number) {
  return Math.min(1, Math.max(0, (growth - start) / (end - start)));
}
