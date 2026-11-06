import type { Feature } from "../../shell/feature";
import { ModList } from "./ModList";
import {
  DataPackDownloads,
  ModDownloads,
  ResourcePackDownloads,
  ShaderDownloads,
} from "./ResourceDownloads";

export const mods: Feature = {
  id: "mods",
  name: "模組管理",
  description: "下載模組、資源包、光影和資料包，並在實例頁管理模組",
  optional: true,
  instancePanel: {
    label: "模組",
    order: 1,
    Panel: ModList,
    available: (instance) => instance.loader !== "vanilla",
  },
  downloadSections: [
    { id: "mods", label: "模組", Panel: ModDownloads },
    { id: "resource-packs", label: "資源包", Panel: ResourcePackDownloads },
    { id: "shaders", label: "光影", Panel: ShaderDownloads },
    { id: "data-packs", label: "資料包", Panel: DataPackDownloads },
  ],
};
