import { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { api } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { ConfirmButton } from "../../shell/ConfirmButton";
import { ErrorText } from "../../shell/ErrorText";
import type { Feature } from "../../shell/feature";
import { useEscape, useFocusHandoff } from "../../shell/keyboard";
import { timeAgo } from "../../shell/time";
import type { Screenshot } from "../../shell/types";
import "./screenshots.css";

/** 大圖檢視：左右鍵換圖，Esc 關閉。 */
function Viewer({
  shots,
  index,
  onMove,
  onDelete,
  onClose,
}: {
  shots: Screenshot[];
  index: number;
  onMove: (index: number) => void;
  onDelete: (shot: Screenshot) => void;
  onClose: () => void;
}) {
  const root = useRef<HTMLDivElement>(null);
  const close = useFocusHandoff<HTMLButtonElement>();
  const shot = shots[index];

  useEscape(onClose, root);

  useEffect(() => {
    function handle(event: KeyboardEvent) {
      if (event.key === "ArrowLeft" && index > 0) {
        onMove(index - 1);
      } else if (event.key === "ArrowRight" && index < shots.length - 1) {
        onMove(index + 1);
      }
    }
    window.addEventListener("keydown", handle);
    return () => window.removeEventListener("keydown", handle);
  }, [index, shots.length, onMove]);

  return (
    <div className="shot-viewer" ref={root} role="dialog" aria-label="截圖">
      <img key={shot.path} src={convertFileSrc(shot.path)} alt={shot.name} />
      <footer>
        <small>
          {index + 1} / {shots.length}，{timeAgo(shot.takenMs, Date.now())}
        </small>
        <button className="text-button" disabled={index === 0} onClick={() => onMove(index - 1)}>
          上一張
        </button>
        <button
          className="text-button"
          disabled={index === shots.length - 1}
          onClick={() => onMove(index + 1)}
        >
          下一張
        </button>
        <ConfirmButton
          className="text-button"
          label="刪除"
          warning="再按一次刪除"
          onConfirm={() => onDelete(shot)}
        />
        <button className="text-button" ref={close} onClick={onClose}>
          關閉
        </button>
      </footer>
    </div>
  );
}

function ScreenshotPanel({ instanceId }: { instanceId: string }) {
  const { contentVersion } = useApp();
  const [shots, setShots] = useState<Screenshot[]>([]);
  const [viewing, setViewing] = useState<number | null>(null);
  const [error, setError] = useState("");

  const load = useCallback(() => {
    api.listScreenshots(instanceId)
      .then(setShots)
      .catch((reason) => setError(String(reason)));
  }, [instanceId]);

  useEffect(load, [load, contentVersion]);

  async function remove(shot: Screenshot) {
    try {
      await api.deleteScreenshot(instanceId, shot.name);
      const left = shots.filter(({ name }) => name !== shot.name);
      setShots(left);
      setViewing((index) =>
        left.length === 0 || index === null ? null : Math.min(index, left.length - 1),
      );
    } catch (reason) {
      setError(String(reason));
    }
  }

  return (
    <>
      <div className="mods-head">
        <h2 className="sr-only">截圖</h2>
        <div className="mods-actions">
          <button
            className="text-button"
            onClick={() =>
              api
                .openInstanceSubfolder(instanceId, "screenshots")
                .catch((reason) => setError(String(reason)))
            }
          >
            打開資料夾
          </button>
        </div>
      </div>
      <ErrorText message={error} />
      {shots.length === 0 && <p className="mods-empty">還沒有截圖。在遊戲裡按 F2 就能截圖。</p>}
      <ul className="shots">
        {shots.map((shot, index) => (
          <li key={shot.path}>
            <button aria-label={`查看 ${shot.name}`} onClick={() => setViewing(index)}>
              <img src={convertFileSrc(shot.path)} alt="" loading="lazy" decoding="async" />
            </button>
          </li>
        ))}
      </ul>
      {viewing !== null && shots[viewing] && (
        <Viewer
          shots={shots}
          index={viewing}
          onMove={setViewing}
          onDelete={remove}
          onClose={() => setViewing(null)}
        />
      )}
    </>
  );
}

export const screenshots: Feature = {
  id: "screenshots",
  name: "截圖",
  description: "在實例頁查看和刪除遊戲截圖",
  optional: true,
  instancePanel: { label: "截圖", order: 2.5, Panel: ScreenshotPanel },
};
