import type { Feature } from "../../shell/feature";
import { InstanceManage } from "./InstanceManage";
import { InstancePage } from "./InstancePage";
import "./instances.css";

export const instances: Feature = {
  id: "instances",
  name: "實例管理",
  description: "新增、改名和刪除實例",
  optional: true,
  tab: { label: "實例", Page: InstancePage },
  instancePanel: { label: "管理", order: 5, Panel: InstanceManage },
};
