import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AccountPanel } from "./AccountPanel";
import { LogPanel } from "./LogPanel";
import { Sapling } from "./Sapling";
import { VersionPicker } from "./VersionPicker";
import type { LaunchEvent, Profile, Progress, Settings, VersionList } from "./types";

/** 遊戲檔案佔整體進度的比例，其餘是 Java。 */
const GAME_SHARE = 0.85;
/** 日誌最多保留的行數，避免長時間遊玩時佔用太多記憶體。 */
const MAX_LOG_LINES = 2000;

type Phase = "idle" | "preparing" | "running" | "failed";

export function LaunchPage() {
  const [list, setList] = useState<VersionList | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [selected, setSelected] = useState("");
  const [profile, setProfile] = useState<Profile | null>(null);
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<Progress | null>(null);
  const [error, setError] = useState("");
  const [logs, setLogs] = useState<string[]>([]);
  const [logOpen, setLogOpen] = useState(false);
  const [downloading, setDownloading] = useState(false);

  const loadVersions = useCallback(() => {
    setListError(null);
    Promise.all([invoke<VersionList>("list_versions"), invoke<Settings>("load_settings")])
      .then(([versions, settings]) => {
        setList(versions);
        const saved = settings.instance.version;
        setSelected(versions.versions.includes(saved) ? saved : versions.latest);
      })
      .catch((reason) => setListError(String(reason)));
  }, []);

  useEffect(loadVersions, [loadVersions]);

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
    setSelected(id);
    setPhase("idle");
    setProgress(null);
  }

  const overall = overallProgress(progress);
  const preparing = phase === "preparing";
  const busy = preparing || phase === "running";

  return (
    <div className="launch">
      <aside className="side">
        <AccountPanel profile={profile} onProfileChange={setProfile} disabled={busy} />

        <section className="panel version">
          <h2 className="panel-title">遊戲版本</h2>
          {listError ? (
            <>
              <p className="error">{listError}</p>
              <button className="secondary" onClick={loadVersions}>
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
        {logs.length > 0 && (
          <button className="stage-log text-button" onClick={() => setLogOpen(true)}>
            查看遊戲日誌
          </button>
        )}

        <div className="stage-text">
          <p className="game-name">Minecraft</p>
          <h1 key={selected}>{selected || "…"}</h1>
          <p className="edition">原版</p>
        </div>

        {/* 檔案都已存在時只是快速檢查，樹苗保持長大的樣子，不用重新長一次。 */}
        <Sapling growth={preparing && downloading ? overall : 1} celebrate={phase === "running"} />

        <svg className="hills" viewBox="0 0 800 220" preserveAspectRatio="none" aria-hidden="true">
          <path className="hill-back" d="M0 170C190 130 430 190 800 40V220H0Z" />
          <path className="hill-front" d="M0 196C260 180 520 200 800 110V220H0Z" />
        </svg>

        <div className="stage-action">
          <StatusLine
            phase={phase}
            progress={progress}
            error={error}
            percent={overall}
            signedIn={profile !== null}
          />
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
          <button
            className={phase === "running" ? "primary running" : "primary"}
            disabled={!selected || !profile || busy}
            onClick={startGame}
          >
            {launchLabel(phase)}
          </button>
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
}: {
  phase: Phase;
  progress: Progress | null;
  error: string;
  percent: number;
  signedIn: boolean;
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
          正在檢查和下載{what}：{toMegabytes(progress.finishedBytes)} /{" "}
          {toMegabytes(progress.totalBytes)} MB
        </span>
        <span>{Math.round(percent * 100)}%</span>
      </p>
    );
  }
  return (
    <p className="status">
      {signedIn ? "第一次啟動會自動下載遊戲檔案和 Java。" : "登入微軟帳號後就能啟動遊戲。"}
    </p>
  );
}

function toMegabytes(bytes: number) {
  return Math.round(bytes / 1024 / 1024);
}

function overallProgress(progress: Progress | null) {
  if (!progress) {
    return 0;
  }
  const part = progress.finishedBytes / Math.max(progress.totalBytes, 1);
  return progress.stage === "game" ? part * GAME_SHARE : GAME_SHARE + part * (1 - GAME_SHARE);
}
