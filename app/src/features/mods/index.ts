import type { Feature } from "../../shell/feature";
import { ModList } from "./ModList";
import { ModDownloads, ResourcePackDownloads, ShaderDownloads } from "./ResourceDownloads";

export const mods: Feature = {
  id: "mods",
  name: "模組管理",
  description: "下載模組、資源包和光影，並在實例頁管理模組",
  optional: true,
  InstancePanel: ModList,
  downloadSections: [
    { id: "mods", label: "模組", Panel: ModDownloads },
    { id: "resource-packs", label: "資源包", Panel: ResourcePackDownloads },
    { id: "shaders", label: "光影", Panel: ShaderDownloads },
  ],
};
