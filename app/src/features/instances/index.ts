import type { Feature } from "../../shell/feature";
import { InstancePage } from "./InstancePage";

export const instances: Feature = {
  id: "instances",
  name: "實例管理",
  description: "新增、改名和刪除實例",
  optional: true,
  tab: { label: "實例", Page: InstancePage },
};
