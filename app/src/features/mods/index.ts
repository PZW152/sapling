import type { Feature } from "../../shell/feature";
import { ModList } from "./ModList";

export const mods: Feature = {
  id: "mods",
  name: "模組管理",
  description: "在實例頁啟用、停用和刪除本地模組",
  optional: true,
  InstancePanel: ModList,
};
