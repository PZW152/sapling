import { useEffect, useState } from "react";
import { api, onInstallEvent } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { overallProgress } from "../../shell/progress";
import type { Progress } from "../../shell/types";

/** 目前的下載。失敗時保留 `error`，讓對應版本顯示原因。 */
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
        }
      }),
    [],
  );

  /** 找到這個版本的實例；沒有就建立一個，以版本號為名稱。 */
  async function instanceForVersion(version: string) {
    const instances = await api.listInstances();
    const existing = instances.find((instance) => instance.version === version);
    if (existing) {
      return existing;
    }
    const created = await api.createInstance(version, version, "vanilla");
    await loadInstances();
    return created;
  }

  async function installGame(version: string) {
    setInstall({ id: version, progress: null, percent: 0, error: "" });
    try {
      await api.installGame(version);
      selectInstance((await instanceForVersion(version)).id);
      setInstall(null);
      await loadVersions();
    } catch (reason) {
      setInstall({ id: version, progress: null, percent: 0, error: String(reason) });
    }
  }

  async function playVersion(version: string) {
    selectInstance((await instanceForVersion(version)).id);
    goTo("launch");
  }

  return { install, installGame, playVersion };
}
