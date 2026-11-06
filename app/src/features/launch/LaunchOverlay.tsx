import { useEffect } from "react";
import type { ComponentType, CSSProperties } from "react";
import { api } from "../../shell/api";
import { overallProgress, toMegabytes } from "../../shell/progress";
import type { Progress } from "../../shell/types";
import { versionUnits } from "./versionUnits";

/** 比 theme.css 的 --overlay 稍長，收合動畫播完才移除。 */
const CLOSE_MS = 780;

/** 主區塊在窗口裡的位置，轉場從這裡展開、也收回這裡。 */
export type OverlayOrigin = { top: number; right: number; bottom: number; left: number };

/** 啟動轉場：主區塊展開到整個窗口，遊戲開始後窗口自動收起。 */
export function LaunchOverlay({
  origin,
  closing,
  running,
  name,
  version,
  progress,
  ProgressView,
  onClosed,
}: {
  origin: OverlayOrigin;
  closing: boolean;
  running: boolean;
  name: string;
  version: string;
  progress: Progress | null;
  /** 功能模塊可以換掉預設的進度條。 */
  ProgressView?: ComponentType<{ share: number }> | null;
  onClosed: () => void;
}) {
  const share = running ? 1 : overallProgress(progress);
  const what = progress?.stage === "java" ? "Java" : "遊戲檔案";

  // 用計時而不是等動畫事件：系統關閉動畫或窗口被遮住時事件不一定會來。
  useEffect(() => {
    if (!closing) {
      return;
    }
    const timer = setTimeout(onClosed, CLOSE_MS);
    return () => clearTimeout(timer);
  }, [closing, onClosed]);

  return (
    <div
      className={closing ? "launch-overlay closing" : "launch-overlay"}
      data-tauri-drag-region
      style={
        {
          "--from-top": `${origin.top}px`,
          "--from-right": `${origin.right}px`,
          "--from-bottom": `${origin.bottom}px`,
          "--from-left": `${origin.left}px`,
          "--units": versionUnits(version),
        } as CSSProperties
      }
    >
      <div className="overlay-content" data-tauri-drag-region>
        <p className="overlay-name">{name}</p>
        <p className="overlay-version">{version}</p>
        <div className="overlay-status">
          <div
            className={ProgressView ? undefined : "overlay-bar"}
            role="progressbar"
            aria-label="啟動進度"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(share * 100)}
          >
            {ProgressView ? (
              <ProgressView share={share} />
            ) : (
              <div style={{ transform: `scaleX(${share})` }} />
            )}
          </div>
          <p className="sr-only" role="status">
            {running ? "遊戲已啟動" : "正在準備遊戲"}
          </p>
          <p key={running ? "running" : "preparing"} className="overlay-note">
            {running && "遊戲已啟動"}
            {!running && !progress && "正在確認需要哪些檔案"}
            {!running &&
              progress &&
              `正在檢查${what}，${toMegabytes(progress.finishedBytes)} / ${toMegabytes(progress.totalBytes)} MB`}
          </p>
          {!running && !closing && (
            <button className="overlay-cancel" onClick={api.cancelLaunch}>
              取消
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
