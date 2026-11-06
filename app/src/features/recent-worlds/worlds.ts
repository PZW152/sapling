const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;
const MONTH_MS = 30 * DAY_MS;

/** 直接進入世界從 1.20 開始才有，啟動器支援的版本裡只有 1.19 不行。 */
export function canEnterWorld(version: string) {
  return version !== "1.19" && !version.startsWith("1.19.");
}

/** 用口語說多久以前玩過。 */
export function playedAgo(playedMs: number, nowMs: number) {
  const elapsed = Math.max(nowMs - playedMs, 0);
  if (elapsed < MINUTE_MS) {
    return "剛剛";
  }
  if (elapsed < HOUR_MS) {
    return `${Math.floor(elapsed / MINUTE_MS)} 分鐘前`;
  }
  if (elapsed < DAY_MS) {
    return `${Math.floor(elapsed / HOUR_MS)} 小時前`;
  }
  if (elapsed < MONTH_MS) {
    return `${Math.floor(elapsed / DAY_MS)} 天前`;
  }
  return `${Math.floor(elapsed / MONTH_MS)} 個月前`;
}

/** 後端傳來的 `icon.png` 內容轉成圖片網址。 */
export function iconUrl(bytes: number[]) {
  const text = bytes.reduce((result, byte) => result + String.fromCharCode(byte), "");
  return `data:image/png;base64,${btoa(text)}`;
}
