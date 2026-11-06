import { ProgressBarStatus, getCurrentWindow } from "@tauri-apps/api/window";

/** 在工作列圖示上顯示進度，切到別的窗口也看得到。`null` 代表清除。 */
export function setTaskbarProgress(share: number | null) {
  const state =
    share === null
      ? { status: ProgressBarStatus.None }
      : { status: ProgressBarStatus.Normal, progress: Math.round(share * 100) };
  getCurrentWindow()
    .setProgressBar(state)
    .catch(() => {});
}
