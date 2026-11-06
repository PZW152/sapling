import type { Instance } from "./types";

/** 最近玩過的排前面，沒玩過的維持建立順序。 */
export function recentFirst(instances: Instance[]) {
  return [...instances].sort((first, second) => second.lastPlayedMs - first.lastPlayedMs);
}

/** 名稱和版本都能搜尋，大小寫不分。 */
export function matchesInstance(instance: Instance, filter: string) {
  const wanted = filter.trim().toLowerCase();
  return `${instance.name} ${instance.version}`.toLowerCase().includes(wanted);
}
