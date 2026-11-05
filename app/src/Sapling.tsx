/** 目前版本卡片裡的樹苗：下載進度就是它長高的過程。`growth` 由 0 到 1。 */
export function Sapling({ growth }: { growth: number }) {
  const rightLeaf = stageOf(growth, 0.35, 0.8);
  const leftLeaf = stageOf(growth, 0.6, 1);

  return (
    <svg className="sapling" viewBox="0 0 200 240" aria-hidden="true">
      <ellipse className="sapling-soil" cx="100" cy="222" rx="58" ry="10" />
      <path
        className="sapling-stem"
        d="M100 222 C100 186 100 150 100 114"
        pathLength={1}
        style={{ strokeDashoffset: 1 - stageOf(growth, 0, 0.6) }}
      />
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
    </svg>
  );
}

function stageOf(growth: number, start: number, end: number) {
  return Math.min(1, Math.max(0, (growth - start) / (end - start)));
}
