import type { Feature } from "../../shell/feature";
import { LaunchAdvanced } from "./LaunchAdvanced";
import { LaunchPage } from "./LaunchPage";
import { LaunchSettings } from "./LaunchSettings";
import "./launch.css";

export const launch: Feature = {
  id: "launch",
  name: "啟動",
  description: "登入帳號和啟動遊戲",
  optional: false,
  tab: { label: "啟動", Page: LaunchPage, keepAlive: true },
  InstancePanel: LaunchSettings,
  AdvancedSettings: LaunchAdvanced,
};
