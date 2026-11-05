import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AccountPanel } from "./AccountPanel";
import { overallProgress, toMegabytes } from "./progress";
import { LogPanel } from "./LogPanel";
import { Sapling } from "./Sapling";
import { VersionPicker } from "./VersionPicker";
import type { LaunchEvent, Profile, Progress, VersionList } from "./types";

/** 問題記錄最多保留的行數，避免長時間遊玩時佔用太多記憶體。 */
const MAX_LOG_LINES = 2000;

type Phase = "idle" | "preparing" | "running" | "failed";

export function LaunchPage({
  list,
  listError,
  selected,
  onSelect,
  onGoToDownload,
  onRetry,
}: {
  list: VersionList | null;
  listError: string | null;
  selected: string;
  onSelect: (id: string) => void;
  onGoToDownload: () => void;
  onRetry: () => void;
}) {
  const [profile, setProfile] = useState<Profile | null>(null);
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<Progress | null>(null);
  const [error, setError] = useState("");
  const [logs, setLogs] = useState<string[]>([]);
  const [logOpen, setLogOpen] = useState(false);
  const [downloading, setDownloading] = useState(false);

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
    setPhase("preparing");
    setProgress(null);
    setDownloading(false);
    setError("");
    setLogs([]);
    try {
      await invoke("play_game", { version: selected });
      setPhase("idle");
    } catch (reason) {
      setError(String(reason));
      setPhase("failed");
    }
  }

  function selectVersion(id: string) {
    onSelect(id);
    setPhase("idle");
    setProgress(null);
  }

  const overall = overallProgress(progress);
  const preparing = phase === "preparing";
  const busy = preparing || phase === "running";
  const noVersion = list !== null && list.installed.length === 0;

  return (
    <div className="launch">
      <aside className="side">
        <AccountPanel profile={profile} onProfileChange={setProfile} disabled={busy} />

        <section className="panel version">
          <h2 className="panel-title">遊戲版本</h2>
          {listError ? (
            <>
              <p className="error">{listError}</p>
              <button className="secondary" onClick={onRetry}>
                重新取得版本
              </button>
            </>
          ) : (
            <VersionPicker
              list={list}
              selected={selected}
              disabled={busy}
              onSelect={selectVersion}
            />
          )}
        </section>
      </aside>

      <section className="stage">
        <div className="stage-text">
          <p className="game-name">Minecraft</p>
          <h1 key={selected}>{selected || (noVersion ? "待播種" : "…")}</h1>
          <p className="edition">原版</p>
        </div>

        {/* 檔案都已存在時只是快速檢查，樹苗保持長大的樣子，不用重新長一次。 */}
        <Sapling
          growth={preparing && downloading ? overall : 1}
          leaves={list?.installed.length ?? 0}
          celebrate={phase === "running"}
        />

        <div className="stage-action">
          <StatusLine
            phase={phase}
            progress={progress}
            error={error}
            percent={overall}
            signedIn={profile !== null}
            noVersion={noVersion}
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
          {noVersion ? (
            <button className="primary" onClick={onGoToDownload}>
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
  noVersion,
}: {
  phase: Phase;
  progress: Progress | null;
  error: string;
  percent: number;
  signedIn: boolean;
  noVersion: boolean;
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
  if (noVersion) {
    return <p className="status">先下載一個遊戲版本。</p>;
  }
  return <p className="status">{signedIn ? "準備好就開始吧。" : "登入微軟帳號後就能啟動遊戲。"}</p>;
}
