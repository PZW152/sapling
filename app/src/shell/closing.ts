import { getCurrentWindow } from "@tauri-apps/api/window";

/** 回傳 true 代表這次不要關閉。 */
type CloseGuard = () => boolean | Promise<boolean>;

const guards = new Set<CloseGuard>();

/** 關閉窗口前先問過這裡。回傳移除的函數。 */
export function addCloseGuard(guard: CloseGuard) {
  guards.add(guard);
  return () => {
    guards.delete(guard);
  };
}

/** 窗口的關閉按鈕和 Alt+F4 都走這裡。 */
export async function requestClose(flush: () => Promise<void>) {
  for (const guard of guards) {
    if (await guard()) {
      return;
    }
  }
  await flush();
  await getCurrentWindow().destroy();
}
