import { setTaskbarProgress } from "../../shell/taskbar";
import { useEffect, useState } from "react";
import { api, onInstallEvent } from "../../shell/api";
import { trackDownload } from "../../shell/downloads";
import { useApp } from "../../shell/AppContext";
import { loaderName } from "../../shell/loaders";
import { overallProgress } from "../../shell/progress";
import type { LoaderKind, Progress } from "../../shell/types";

/** 目前的下載。失敗時保留 `error`，讓對應版本顯示原因。 */
/** 和後端取消時回傳的文字相同。 */
const CANCELLED = "已取消下載";

export type InstallState = {
  id: string;
  progress: Progress | null;
  percent: number;
  error: string;
};

/** 下載版本和跳到對應實例。狀態留在這裡，進度更新不會牽動其他頁面。 */
export function useInstall() {
  const { goTo, loadInstances, loadVersions, selectInstance } = useApp();
  const [install, setInstall] = useState<InstallState | null>(null);

  useEffect(
    () =>
      onInstallEvent((payload) => {
        if (payload.kind === "progress") {
          setInstall(
            (current) =>
              current && { ...current, progress: payload, percent: overallProgress(payload) },
          );
          setTaskbarProgress(overallProgress(payload));
        }
      }),
    [],
  );

  /** 找到這個版本和載入器的實例；沒有就建立一個。 */
  async function instanceFor(version: string, loader: LoaderKind) {
    const instances = await api.listInstances();
    const existing = instances.find(
      (instance) => instance.version === version && instance.loader === loader,
    );
    if (existing) {
      return existing;
    }
    const name = loader === "vanilla" ? version : `${version} ${loaderName(loader)}`;
    const created = await api.createInstance(name, version, loader);
    await loadInstances();
    return created;
  }

  async function installGame(version: string, loader: LoaderKind) {
    setInstall({ id: version, progress: null, percent: 0, error: "" });
    try {
      // 先建立實例：這個版本沒有對應的載入器時，不用等遊戲下載完才知道。
      const instance = await instanceFor(version, loader);
      await trackDownload(`Minecraft ${version}`, () => api.installGame(version));
      selectInstance(instance.id);
      setInstall(null);
      await loadVersions();
    } catch (reason) {
      const cancelled = String(reason) === CANCELLED;
      setInstall(cancelled ? null : { id: version, progress: null, percent: 0, error: String(reason) });
    }
    setTaskbarProgress(null);
  }

  async function playVersion(version: string, loader: LoaderKind) {
    try {
      selectInstance((await instanceFor(version, loader)).id);
      goTo("launch");
    } catch (reason) {
      setInstall({ id: version, progress: null, percent: 0, error: String(reason) });
    }
  }

  return { install, installGame, playVersion, cancelInstall: api.cancelInstall };
}
