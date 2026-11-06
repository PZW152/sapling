import { useEffect, useRef } from "react";
import { useEscape, useFocusHandoff } from "../../shell/keyboard";

export function LogPanel({ lines, onClose }: { lines: string[]; onClose: () => void }) {
  const end = useRef<HTMLDivElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const close = useFocusHandoff<HTMLButtonElement>();

  useEscape(onClose, panel);

  useEffect(() => {
    end.current?.scrollIntoView();
  }, [lines]);

  return (
    <div className="log-panel" ref={panel} role="dialog" aria-label="問題記錄">
      <header>
        <h2>問題記錄</h2>
        <button className="text-button" ref={close} onClick={onClose}>
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
