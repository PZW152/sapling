const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;
const MONTH_MS = 30 * DAY_MS;
const YEAR_MS = 365 * DAY_MS;

/** 用口語說多久以前。 */
export function timeAgo(thenMs: number, nowMs: number) {
  const elapsed = Math.max(nowMs - thenMs, 0);
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
  if (elapsed < YEAR_MS) {
    return `${Math.floor(elapsed / MONTH_MS)} 個月前`;
  }
  return `${Math.floor(elapsed / YEAR_MS)} 年前`;
}

/** 累計遊玩時間。不到一分鐘的不值得顯示。 */
export function formatPlaytime(seconds: number) {
  const minutes = Math.floor(seconds / 60);
  if (minutes < 1) {
    return null;
  }
  return minutes < 60 ? `玩了 ${minutes} 分鐘` : `玩了 ${(minutes / 60).toFixed(1)} 小時`;
}
