import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useApp } from "../../shell/AppContext";
import { HIDDEN_THEME, isArborDay } from "../arbor-day";
import { overallProgress, toMegabytes } from "../../shell/progress";
import type { LaunchEvent, Profile, Progress } from "../../shell/types";
import { AccountPanel } from "./AccountPanel";
import { InstancePicker } from "./InstancePicker";
import { LogPanel } from "./LogPanel";
import { Sapling } from "./Sapling";

/** 問題記錄最多保留的行數，避免長時間遊玩時佔用太多記憶體。 */
const MAX_LOG_LINES = 2000;
/** 植樹節當天樹苗長滿所有葉子。 */
const ARBOR_DAY_LEAVES = 5;

type Phase = "idle" | "preparing" | "running" | "failed";

export function LaunchPage() {
  const { instances, selected, listError, config, selectInstance, goTo, loadVersions, updateConfig } =
    useApp();
  const [profile, setProfile] = useState<Profile | null>(null);
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<Progress | null>(null);
  const [error, setError] = useState("");
  const [logs, setLogs] = useState<string[]>([]);
  const [logOpen, setLogOpen] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [plantings, setPlantings] = useState(0);

  useEffect(() => {
    const stopListening = listen<LaunchEvent>("launch-event", ({ payload }) => {
      if (payload.kind === "progress") {
        setProgress(payload);
        if (payload.downloadedFiles > 0) {
          setDownloading(true);
        }
      } else if (payload.kind === "started") {
        setPhase("running");
      } else {
        setLogs((lines) => [...lines, payload.line].slice(-MAX_LOG_LINES));
      }
    });
    return () => {
      stopListening.then((stop) => stop());
    };
  }, []);

  async function startGame() {
    if (!selected) {
      return;
    }
    setPhase("preparing");
    setProgress(null);
    setDownloading(false);
    setError("");
    setLogs([]);
    try {
      await invoke("play_game", { instanceId: selected.id });
      setPhase("idle");
    } catch (reason) {
      setError(String(reason));
      setPhase("failed");
    }
  }

  const today = new Date();
  const arborDay = isArborDay(today);
  const plantedToday = config.plantedYears.includes(today.getFullYear());

  function plant() {
    updateConfig({ plantedYears: [...config.plantedYears, today.getFullYear()] });
    setPlantings(plantings + 1);
  }

  function choose(id: string) {
    selectInstance(id);
    setPhase("idle");
    setProgress(null);
  }

  const overall = overallProgress(progress);
  const preparing = phase === "preparing";
  const busy = preparing || phase === "running";
  const noInstance = instances.length === 0;

  return (
    <div className="launch">
      <aside className="side">
        <AccountPanel profile={profile} onProfileChange={setProfile} disabled={busy} />

        <section className="panel version">
          <h2 className="panel-title">實例</h2>
          {listError ? (
            <>
              <p className="error">{listError}</p>
              <button className="secondary" onClick={loadVersions}>
                重新取得版本
              </button>
            </>
          ) : (
            <InstancePicker
              instances={instances}
              selected={selected}
              disabled={busy}
              onSelect={choose}
            />
          )}
        </section>
      </aside>

      <section className="stage">
        <div className="stage-text">
          <p className="game-name">{selected?.name ?? "Minecraft"}</p>
          <h1 key={selected?.id ?? "empty"}>{selected?.version ?? "待播種"}</h1>
          <p className="edition">原版</p>
        </div>

        {/* 檔案都已存在時只是快速檢查，樹苗保持長大的樣子，不用重新長一次。 */}
        <Sapling
          key={plantings}
          growth={preparing && downloading ? overall : 1}
          leaves={arborDay ? ARBOR_DAY_LEAVES : instances.length}
          celebrate={phase === "running"}
          blossoms={config.theme === HIDDEN_THEME || (arborDay && plantedToday)}
          onPlant={arborDay && !plantedToday ? plant : undefined}
        />

        <div className="stage-action">
          <StatusLine
            phase={phase}
            progress={progress}
            error={error}
            percent={overall}
            signedIn={profile !== null}
            noInstance={noInstance}
          />
          {phase === "failed" && logs.length > 0 && (
            <button className="text-button problem-button" onClick={() => setLogOpen(true)}>
              查看問題記錄
            </button>
          )}
          {preparing && (
            <div
              className="progress"
              role="progressbar"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(overall * 100)}
            >
              <div style={{ transform: `scaleX(${overall})` }} />
            </div>
          )}
          {noInstance ? (
            <button className="primary" onClick={() => goTo("download")}>
              前往下載
            </button>
          ) : (
            <button
              className={phase === "running" ? "primary running" : "primary"}
              disabled={!selected || !profile || busy}
              onClick={startGame}
            >
              {launchLabel(phase)}
            </button>
          )}
        </div>

        {logOpen && <LogPanel lines={logs} onClose={() => setLogOpen(false)} />}
      </section>
    </div>
  );
}

function launchLabel(phase: Phase) {
  if (phase === "preparing") {
    return "準備中";
  }
  return phase === "running" ? "遊戲運行中" : "啟動遊戲";
}

function StatusLine({
  phase,
  progress,
  error,
  percent,
  signedIn,
  noInstance,
}: {
  phase: Phase;
  progress: Progress | null;
  error: string;
  percent: number;
  signedIn: boolean;
  noInstance: boolean;
}) {
  if (phase === "failed") {
    return <p className="status-error">{error}</p>;
  }
  if (phase === "running") {
    return <p className="status">遊戲已啟動，關閉遊戲後可以再次啟動。</p>;
  }
  if (phase === "preparing") {
    if (!progress) {
      return <p className="status">正在確認需要哪些檔案</p>;
    }
    const what = progress.stage === "game" ? "遊戲檔案" : "Java";
    return (
      <p className="status status-row">
        <span>
          正在檢查{what}：{toMegabytes(progress.finishedBytes)} /{" "}
          {toMegabytes(progress.totalBytes)} MB
        </span>
        <span>{Math.round(percent * 100)}%</span>
      </p>
    );
  }
  if (noInstance) {
    return <p className="status">先下載一個遊戲版本。</p>;
  }
  return <p className="status">{signedIn ? "準備好就開始吧。" : "登入微軟帳號後就能啟動遊戲。"}</p>;
}
