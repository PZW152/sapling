import type { Feature } from "../../shell/feature";

/** 模組清單顯示在實例頁裡，這裡只提供開關。 */
export const mods: Feature = {
  id: "mods",
  name: "模組管理",
  description: "在實例頁啟用、停用和刪除本地模組",
  optional: true,
};
