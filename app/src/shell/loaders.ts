import type { LoaderKind } from "./types";

/** 建立實例時可以選的載入器。 */
export const LOADERS: { kind: LoaderKind; name: string }[] = [
  { kind: "vanilla", name: "原版" },
  { kind: "fabric", name: "Fabric" },
  { kind: "neoForge", name: "NeoForge" },
  { kind: "forge", name: "Forge" },
];

export function loaderName(kind: LoaderKind) {
  return LOADERS.find((loader) => loader.kind === kind)?.name ?? kind;
}
