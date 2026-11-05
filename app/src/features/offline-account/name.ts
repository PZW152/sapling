/** 遊戲接受的玩家名稱：3 到 16 個英文字母、數字或底線。規則要和 core 的 auth 一致。 */
export function isValidPlayerName(name: string) {
  return /^[A-Za-z0-9_]{3,16}$/.test(name);
}
