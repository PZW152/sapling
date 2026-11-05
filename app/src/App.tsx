import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Sapling } from "./Sapling";
import "./App.css";

type Tab = "launch" | "download" | "settings" | "more";

const TABS: { id: Tab; label: string }[] = [
  { id: "launch", label: "啟動" },
  { id: "download", label: "下載" },
  { id: "settings", label: "設定" },
  { id: "more", label: "更多" },
];

const COMING_LATER: Record<Exclude<Tab, "launch">, string> = {
  download: "這裡之後用來管理遊戲版本和模組。",
  settings: "這裡之後用來調整記憶體、Java 和下載設定。",
  more: "這裡之後放關於 Sapling 的資訊。",
};

/** 遊戲檔案佔整體進度的比例，其餘是 Java。 */
const GAME_SHARE = 0.85;

type VersionList = { latest: string; versions: string[] };

type Progress = {
  stage: "game" | "java";
  finishedFiles: number;
  totalFiles: number;
  finishedBytes: number;
  totalBytes: number;
};

type Phase = "idle" | "preparing" | "ready" | "failed";

function App() {
  const [tab, setTab] = useState<Tab>("launch");

  return (
    <div className="app">
      <TitleBar tab={tab} onTabChange={setTab} />
      <main className="page" key={tab}>
        {tab === "launch" ? <LaunchPage /> : <ComingLater text={COMING_LATER[tab]} />}
      </main>
    </div>
  );
}

function TitleBar({ tab, onTabChange }: { tab: Tab; onTabChange: (tab: Tab) => void }) {
  const appWindow = getCurrentWindow();

  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="brand" data-tauri-drag-region>
        <img src="/logo.svg" alt="" width={26} height={26} data-tauri-drag-region />
        <span data-tauri-drag-region>Sapling</span>
      </div>
      <nav className="tabs" aria-label="主要分頁">
        {TABS.map(({ id, label }) => (
          <button
            key={id}
            className="tab"
            aria-current={tab === id ? "page" : undefined}
            onClick={() => onTabChange(id)}
          >
            {label}
          </button>
        ))}
      </nav>
      <div className="window-controls">
        <button aria-label="最小化" onClick={() => appWindow.minimize()}>
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path d="M2 6h8" />
          </svg>
        </button>
        <button aria-label="最大化" onClick={() => appWindow.toggleMaximize()}>
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <rect x="2.5" y="2.5" width="7" height="7" rx="1" />
          </svg>
        </button>
        <button className="close" aria-label="關閉" onClick={() => appWindow.close()}>
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" />
          </svg>
        </button>
      </div>
    </header>
  );
}

function LaunchPage() {
  const [list, setList] = useState<VersionList | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [selected, setSelected] = useState("");
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<Progress | null>(null);
  const [error, setError] = useState("");

  const loadVersions = useCallback(() => {
    setListError(null);
    invoke<VersionList>("list_versions")
      .then((result) => {
        setList(result);
        setSelected(result.latest);
      })
      .catch((reason) => setListError(String(reason)));
  }, []);

  useEffect(loadVersions, [loadVersions]);

  useEffect(() => {
    const stopListening = listen<Progress>("prepare-progress", (event) =>
      setProgress(event.payload),
    );
    return () => {
      stopListening.then((stop) => stop());
    };
  }, []);

  async function prepareGame() {
    setPhase("preparing");
    setProgress(null);
    setError("");
    try {
      await invoke("prepare", { id: selected });
      setPhase("ready");
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

  return (
    <div className="launch">
      <aside className="side">
        <section className="panel account">
          <svg className="avatar" viewBox="0 0 56 56" aria-hidden="true">
            <circle cx="28" cy="22" r="10" />
            <path d="M10 56c0-13 8-20 18-20s18 7 18 20z" />
          </svg>
          <div>
            <h2>尚未登入</h2>
            <p>需要正版微軟帳號才能進入遊戲</p>
          </div>
          <button className="secondary" disabled>
            用微軟帳號登入
          </button>
        </section>

        <section className="panel version">
          <label htmlFor="version">遊戲版本</label>
          {listError ? (
            <>
              <p className="error">{listError}</p>
              <button className="secondary" onClick={loadVersions}>
                重新取得版本
              </button>
            </>
          ) : (
            <select
              id="version"
              value={selected}
              disabled={!list || preparing}
              onChange={(event) => selectVersion(event.target.value)}
            >
              {!list && <option value="">正在取得版本</option>}
              {list?.versions.map((id) => (
                <option key={id} value={id}>
                  {id === list.latest ? `${id}（最新）` : id}
                </option>
              ))}
            </select>
          )}
        </section>
      </aside>

      <section className="stage">
        <div className="stage-text">
          <p className="game-name">Minecraft</p>
          <h1>{selected || "…"}</h1>
          <p className="edition">原版</p>
        </div>

        <Sapling growth={preparing ? overall : 1} />

        <div className="stage-action">
          <StatusLine phase={phase} progress={progress} error={error} />
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
          <button className="primary" disabled={!selected || preparing} onClick={prepareGame}>
            {preparing ? "準備中" : "啟動遊戲"}
          </button>
        </div>
      </section>
    </div>
  );
}

function StatusLine({
  phase,
  progress,
  error,
}: {
  phase: Phase;
  progress: Progress | null;
  error: string;
}) {
  if (phase === "failed") {
    return <p className="status error">{error}</p>;
  }
  if (phase === "ready") {
    return <p className="status">遊戲檔案和 Java 都準備好了，進入遊戲的功能還在製作中。</p>;
  }
  if (phase === "preparing") {
    if (!progress) {
      return <p className="status">正在確認需要哪些檔案</p>;
    }
    const what = progress.stage === "game" ? "遊戲檔案" : "Java";
    return (
      <p className="status">
        正在檢查和下載{what}：{progress.finishedFiles} / {progress.totalFiles}
      </p>
    );
  }
  return null;
}

function overallProgress(progress: Progress | null) {
  if (!progress) {
    return 0;
  }
  const part = progress.finishedBytes / Math.max(progress.totalBytes, 1);
  return progress.stage === "game" ? part * GAME_SHARE : GAME_SHARE + part * (1 - GAME_SHARE);
}

function ComingLater({ text }: { text: string }) {
  return (
    <div className="coming-later">
      <p>{text}</p>
    </div>
  );
}

export default App;
