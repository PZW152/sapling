import type { LauncherConfig } from "../../shell/types";

/** 遊戲接受的玩家名稱：3 到 16 個英文字母、數字或底線。規則要和 core 的 auth 一致。 */
export function isValidPlayerName(name: string) {
  return /^[A-Za-z0-9_]{3,16}$/.test(name);
}

/** 開了離線模式卻沒有可用名稱時，不能悄悄改用正版帳號啟動。 */
export function offlineLaunchBlock({ offlineEnabled, offlineName }: LauncherConfig) {
  return offlineEnabled && !isValidPlayerName(offlineName) ? "先填好離線玩家名稱。" : null;
}
