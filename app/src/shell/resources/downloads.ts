const TEN_THOUSAND = 10_000;
const HUNDRED_MILLION = 100_000_000;

/** 下載次數用中文習慣的單位顯示。 */
export function formatDownloads(count: number) {
  if (count >= HUNDRED_MILLION) {
    return `${(count / HUNDRED_MILLION).toFixed(1)} 億`;
  }
  if (count >= TEN_THOUSAND) {
    return `${Math.floor(count / TEN_THOUSAND)} 萬`;
  }
  return String(count);
}
