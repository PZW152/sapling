import { displayTitle } from "../../shell/resources/names";
import type { InstalledMod, ModFile } from "../../shell/types";

/** 清單上顯示的名稱。查不到的模組只有檔名。 */
export function modTitle(file: ModFile, mod: InstalledMod | undefined) {
  return mod ? displayTitle(mod) : file.name;
}

/** 名稱和檔名都能搜尋，大小寫不分。 */
export function matchesMod(file: ModFile, mod: InstalledMod | undefined, filter: string) {
  const wanted = filter.trim().toLowerCase();
  return `${modTitle(file, mod)} ${file.name}`.toLowerCase().includes(wanted);
}

/** 目前還在資料夾裡、又有新版的模組。 */
export function updatable(files: ModFile[], known: InstalledMod[]) {
  const names = new Set(files.map((file) => file.name));
  return known.filter((mod) => mod.update && names.has(mod.fileName));
}
