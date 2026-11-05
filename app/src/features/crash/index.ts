import type { Feature } from "../../shell/feature";

/** 分析在核心完成，這裡只提供開關。id 必須和 Tauri 層的 CRASH_FEATURE 相同。 */
export const crash: Feature = {
  id: "crash",
  name: "崩潰分析",
  description: "遊戲異常結束時，用白話說明可能原因",
  optional: true,
};
