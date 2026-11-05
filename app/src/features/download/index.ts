import type { Feature } from "../../shell/feature";
import { DownloadPage } from "./DownloadPage";
import { DownloadSettings } from "./DownloadSettings";

export const download: Feature = {
  id: "download",
  name: "下載",
  description: "下載遊戲版本和 Java",
  optional: false,
  tab: { label: "下載", Page: DownloadPage },
  Settings: DownloadSettings,
};
