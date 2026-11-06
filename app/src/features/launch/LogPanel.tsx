import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../../shell/api";
import { useEscape, useFocusHandoff } from "../../shell/keyboard";
import { lineLevel, matchesLine, tailOf } from "./logs";

/** 畫面上最多畫幾行。全部畫出來的話，遊戲狂印日誌時介面會卡。 */
const SHOWN_LINES = 500;
/** 離底部這麼近就算在看最新的內容。 */
const STICK_DISTANCE = 40;

/** 遊戲日誌：運行中即時更新，崩潰後預設只看問題。 */
export function LogPanel({
  lines,
  instanceId,
  problemsFirst,
  onClose,
}: {
  lines: string[];
  instanceId: string | undefined;
  problemsFirst: boolean;
  onClose: () => void;
}) {
  const panel = useRef<HTMLDivElement>(null);
  const body = useRef<HTMLDivElement>(null);
  const close = useFocusHandoff<HTMLButtonElement>();
  const [problemsOnly, setProblemsOnly] = useState(problemsFirst);
  const [search, setSearch] = useState("");
  const [copied, setCopied] = useState(false);
  // 往上捲時停止跟隨，才看得了舊內容。
  const follow = useRef(true);

  useEscape(onClose, panel);

  const matched = useMemo(
    () => lines.filter((line) => matchesLine(line, problemsOnly, search)),
    [lines, problemsOnly, search],
  );
  const shown = tailOf(matched, SHOWN_LINES);

  useEffect(() => {
    const element = body.current;
    if (element && follow.current) {
      element.scrollTop = element.scrollHeight;
    }
  }, [shown]);

  function trackScroll() {
    const element = body.current;
    if (element) {
      const distance = element.scrollHeight - element.scrollTop - element.clientHeight;
      follow.current = distance < STICK_DISTANCE;
    }
  }

  async function copy() {
    await navigator.clipboard.writeText(matched.join("\n"));
    setCopied(true);
  }

  return (
    <div className="log-panel" ref={panel} role="dialog" aria-label="遊戲日誌">
      <header>
        <h2>遊戲日誌</h2>
        <input
          type="search"
          aria-label="搜尋日誌"
          placeholder="搜尋"
          spellCheck={false}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
        <button className="chip" aria-pressed={problemsOnly} onClick={() => setProblemsOnly(!problemsOnly)}>
          只看問題
        </button>
        <div className="mods-actions">
          <button className="text-button" onClick={copy}>
            {copied ? "已複製" : "複製"}
          </button>
          {instanceId && (
            <button
              className="text-button"
              onClick={() => api.openInstanceSubfolder(instanceId, "logs").catch(() => {})}
            >
              打開資料夾
            </button>
          )}
          <button className="text-button" ref={close} onClick={onClose}>
            關閉
          </button>
        </div>
      </header>
      <div className="log-lines" ref={body} onScroll={trackScroll} tabIndex={0}>
        {matched.length === 0 && <p>{lines.length === 0 ? "還沒有內容。" : "沒有符合的內容。"}</p>}
        {matched.length > shown.length && (
          <p>前面還有 {matched.length - shown.length} 行，可以複製全部或打開資料夾查看。</p>
        )}
        {shown.map((line, index) => (
          <p key={index} data-level={lineLevel(line)}>
            {line}
          </p>
        ))}
      </div>
    </div>
  );
}
