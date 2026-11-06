import type { Feature } from "../../shell/feature";
import { LaunchPage } from "./LaunchPage";
import { LaunchOptions } from "./LaunchOptions";
import { LaunchSettings } from "./LaunchSettings";
import "./launch.css";

export const launch: Feature = {
  id: "launch",
  name: "啟動",
  description: "登入帳號和啟動遊戲",
  optional: false,
  tab: { label: "啟動", Page: LaunchPage, keepAlive: true },
  instancePanel: { label: "設定", order: 4, Panel: LaunchSettings },
  Settings: LaunchOptions,
};
