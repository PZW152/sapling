import { useEffect, useRef } from "react";

export function LogPanel({ lines, onClose }: { lines: string[]; onClose: () => void }) {
  const end = useRef<HTMLDivElement>(null);

  useEffect(() => {
    end.current?.scrollIntoView();
  }, [lines]);

  return (
    <div className="log-panel" role="dialog" aria-label="遊戲日誌">
      <header>
        <h2>遊戲日誌</h2>
        <button className="text-button" onClick={onClose}>
          關閉
        </button>
      </header>
      <div className="log-lines">
        {lines.map((line, index) => (
          <p key={index}>{line}</p>
        ))}
        <div ref={end} />
      </div>
    </div>
  );
}
