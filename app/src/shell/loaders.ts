import type { LoaderKind } from "./types";

/** 建立實例時可以選的載入器。 */
export const LOADERS: { kind: LoaderKind; name: string }[] = [
  { kind: "vanilla", name: "原版" },
  { kind: "fabric", name: "Fabric" },
  { kind: "neoForge", name: "NeoForge" },
  { kind: "forge", name: "Forge" },
];

/** 載入器欄位的標題。多數人不知道載入器是什麼，只要知道什麼時候需要。 */
export const LOADER_HINT = "載入器（要裝模組才需要，不確定就用原版）";

export function loaderName(kind: LoaderKind) {
  return LOADERS.find((loader) => loader.kind === kind)?.name ?? kind;
}
