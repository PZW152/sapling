import { useCallback, useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, onLaunchEvent } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { overallProgress } from "../../shell/progress";
import { setTaskbarProgress } from "../../shell/taskbar";
import type { Progress } from "../../shell/types";
import type { OverlayOrigin } from "./LaunchOverlay";

/** 問題記錄最多保留的行數，避免長時間遊玩時佔用太多記憶體。 */
const MAX_LOG_LINES = 2000;
/** 遊戲開始後等多久才收起窗口，讓用戶看到轉場完成。 */
const MINIMIZE_DELAY_MS = 1500;

export type Phase = "idle" | "preparing" | "running" | "failed";

async function returnToLauncher() {
  const appWindow = getCurrentWindow();
  await appWindow.unminimize();
  await appWindow.setFocus();
}

/** 一次啟動的完整狀態：準備、運行、失敗，以及啟動轉場。 */
export function useLaunch() {
  const { config, loadInstances } = useApp();
  const minimize = config.minimizeOnLaunch;
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<Progress | null>(null);
  const [error, setError] = useState("");
  const [logs, setLogs] = useState<string[]>([]);
  const [origin, setOrigin] = useState<OverlayOrigin | null>(null);

  useEffect(
    () =>
      onLaunchEvent((payload) => {
        if (payload.kind === "progress") {
          setProgress(payload);
          setTaskbarProgress(overallProgress(payload));
        } else if (payload.kind === "started") {
          setPhase("running");
          setTaskbarProgress(null);
        } else {
          setLogs((lines) => [...lines, payload.line].slice(-MAX_LOG_LINES));
        }
      }),
    [],
  );

  useEffect(() => {
    if (phase !== "running") {
      return;
    }
    // 收起後移除轉場，用戶中途打開窗口時仍然可以使用啟動器。
    const timer = setTimeout(async () => {
      if (minimize) {
        await getCurrentWindow().minimize();
      }
      setOrigin(null);
    }, MINIMIZE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [phase, minimize]);

  async function start(
    instanceId: string,
    offlineName: string | null,
    world: string | null,
    server: string | null,
    from: OverlayOrigin | null,
  ) {
    setOrigin(from);
    setPhase("preparing");
    setProgress(null);
    setError("");
    setLogs([]);
    try {
      await api.playGame(instanceId, offlineName, world, server);
      setPhase("idle");
    } catch (reason) {
      setError(String(reason));
      setPhase("failed");
    }
    setTaskbarProgress(null);
    returnToLauncher();
    // 遊玩時間和排序更新了。
    loadInstances();
  }

  function reset() {
    setPhase("idle");
    setProgress(null);
  }

  const clearOrigin = useCallback(() => setOrigin(null), []);

  return { phase, progress, error, logs, origin, start, reset, clearOrigin };
}
