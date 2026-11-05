import { useState } from "react";

/** 下載更多版本時多長出的小葉子：莖上的位置、大小和方向。 */
const EXTRA_LEAVES = [
  { x: 100, y: 196, size: 0.42, flip: false },
  { x: 100, y: 184, size: 0.36, flip: true },
  { x: 100, y: 172, size: 0.3, flip: false },
  { x: 100, y: 138, size: 0.26, flip: false },
];
const RIGHT_LEAF_PATH = "M0 0 C0 -46 32 -76 76 -80 C76 -32 46 0 0 0 Z";

/**
 * 首頁的樹苗。`growth` 由 0 到 1，下載時就是它長高的過程。
 * 已下載的版本越多，`leaves` 越多，樹苗長出越多小葉子。
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
      <path
        className="sapling-stem"
        d="M100 226 C100 190 100 150 100 114"
        pathLength={1}
        style={{ strokeDashoffset: 1 - stageOf(growth, 0, 0.6) }}
      />
      {extraLeaves.map(({ x, y, size, flip }) => (
        <g key={y} transform={`translate(${x} ${y}) scale(${flip ? -size : size} ${size})`}>
          <path className="sapling-extra" d={RIGHT_LEAF_PATH} />
        </g>
      ))}
      <path
        className="sapling-leaf sapling-leaf-right"
        d="M100 112 C100 66 132 36 176 32 C176 80 146 112 100 112 Z"
        style={{ transform: `scale(${rightLeaf})`, opacity: rightLeaf > 0 ? 1 : 0 }}
      />
      <path
        className="sapling-leaf sapling-leaf-left"
        d="M100 158 C100 122 70 102 28 102 C28 140 58 164 100 158 Z"
        style={{ transform: `scale(${leftLeaf})`, opacity: leftLeaf > 0 ? 1 : 0 }}
      />
      <ellipse className="sapling-mound" cx="100" cy="232" rx="52" ry="10" />
    </svg>
  );
}

function stageOf(growth: number, start: number, end: number) {
  return Math.min(1, Math.max(0, (growth - start) / (end - start)));
}
