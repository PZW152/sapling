import { getCurrentWebview } from "@tauri-apps/api/webview";

/** 放大整個介面。用窗口的縮放，版面會照放大後的尺寸重新排列。 */
export function applyScale(percent: number) {
  getCurrentWebview()
    .setZoom(percent / 100)
    .catch(() => {});
}
