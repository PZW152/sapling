export { timeAgo as playedAgo } from "../../shell/time";

/** 第一個支援直接進入世界的 `1.x` 版本。 */
const FIRST_QUICK_PLAY_MINOR = 20;

/** 直接進入世界從 1.20 開始才有。 */
export function canEnterWorld(version: string) {
  const minor = /^1\.(\d+)/.exec(version)?.[1];
  return minor === undefined || Number(minor) >= FIRST_QUICK_PLAY_MINOR;
}

/** 後端傳來的 `icon.png` 內容轉成圖片網址。 */
export function iconUrl(bytes: number[]) {
  const text = bytes.reduce((result, byte) => result + String.fromCharCode(byte), "");
  return `data:image/png;base64,${btoa(text)}`;
}
