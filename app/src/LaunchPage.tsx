import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Sapling } from "./Sapling";
import type { LaunchEvent, LoginCode, Profile, Progress, Settings, VersionList } from "./types";

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
            className="primary"
            disabled={!selected || !profile || busy}
            onClick={startGame}
          >
            {launchLabel(phase)}
          </button>
          {logs.length > 0 && (
            <button className="text-button" onClick={() => setLogOpen(true)}>
              查看遊戲日誌
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

function AccountPanel({
  profile,
  onProfileChange,
  disabled,
}: {
  profile: Profile | null;
  onProfileChange: (profile: Profile | null) => void;
  disabled: boolean;
}) {
  const [restoring, setRestoring] = useState(true);
  const [code, setCode] = useState<LoginCode | null>(null);
  const [starting, setStarting] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    invoke<Profile | null>("restore_account")
      .then(onProfileChange)
      .catch((reason) => setError(String(reason)))
      .finally(() => setRestoring(false));
  }, [onProfileChange]);

  async function signIn() {
    setError("");
    setStarting(true);
    try {
      const loginCode = await invoke<LoginCode>("login_start");
      setCode(loginCode);
      setCopied(await copyText(loginCode.userCode));
      setStarting(false);
      onProfileChange(await invoke<Profile>("login_finish"));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setStarting(false);
      setCode(null);
    }
  }

  async function signOut() {
    setError("");
    try {
      await invoke("sign_out");
      onProfileChange(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  if (profile) {
    return (
      <section className="panel account">
        <Avatar skinUrl={profile.skinUrl} />
        <div>
          <h2>{profile.name}</h2>
          <p>正版帳號</p>
        </div>
        <button className="secondary" disabled={disabled} onClick={signOut}>
          登出
        </button>
        {error && <p className="error">{error}</p>}
      </section>
    );
  }

  if (code) {
    return (
      <section className="panel account">
        <h2>在瀏覽器輸入代碼</h2>
        <button className="login-code" onClick={() => copyText(code.userCode).then(setCopied)}>
          {code.userCode}
        </button>
        <p>
          {copied ? "代碼已複製。" : "按上面的代碼可以複製。"}
          登入頁面已在瀏覽器打開，沒看到的話請前往 {code.verificationUri}
        </p>
        <p className="status">正在等待你完成登入</p>
      </section>
    );
  }

  return (
    <section className="panel account">
      <Avatar skinUrl={null} />
      <div>
        <h2>{restoring ? "正在恢復登入" : "尚未登入"}</h2>
        <p>需要正版微軟帳號才能進入遊戲</p>
      </div>
      <button className="secondary" disabled={restoring || starting} onClick={signIn}>
        {starting ? "正在取得代碼" : "用微軟帳號登入"}
      </button>
      {error && <p className="error">{error}</p>}
    </section>
  );
}

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

function Avatar({ skinUrl }: { skinUrl: string | null }) {
  if (skinUrl) {
    return (
      <div className="skin-head" aria-hidden="true">
        <span className="skin-face" style={{ backgroundImage: `url(${skinUrl})` }} />
        <span className="skin-hat" style={{ backgroundImage: `url(${skinUrl})` }} />
      </div>
    );
  }
  return (
    <svg className="avatar" viewBox="0 0 56 56" aria-hidden="true">
      <circle cx="28" cy="22" r="10" />
      <path d="M10 56c0-13 8-20 18-20s18 7 18 20z" />
    </svg>
  );
}

function VersionPicker({
  list,
  selected,
  disabled,
  onSelect,
}: {
  list: VersionList | null;
  selected: string;
  disabled: boolean;
  onSelect: (id: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    function closeOnOutsideClick(event: PointerEvent) {
      if (!root.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setOpen(false);
        trigger.current?.focus();
      }
    }
    document.addEventListener("pointerdown", closeOnOutsideClick);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOnOutsideClick);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [open]);

  return (
    <div className="picker" ref={root}>
      {open && list && (
        <ul className="picker-list" aria-label="選擇遊戲版本">
          {list.versions.map((id) => (
            <li key={id}>
              <button
                className="picker-option"
                aria-current={id === selected ? "true" : undefined}
                onClick={() => {
                  onSelect(id);
                  setOpen(false);
                  trigger.current?.focus();
                }}
              >
                <span>{id}</span>
                {id === list.latest && <small>最新</small>}
              </button>
            </li>
          ))}
        </ul>
      )}
      <button
        ref={trigger}
        className="picker-trigger"
        aria-expanded={open}
        disabled={!list || disabled}
        onClick={() => setOpen(!open)}
      >
        <span>{list ? selected : "正在取得版本"}</span>
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path d="M2.5 7.5 6 4l3.5 3.5" />
        </svg>
      </button>
    </div>
  );
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
    return <p className="status error">{error}</p>;
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

function LogPanel({ lines, onClose }: { lines: string[]; onClose: () => void }) {
  const end = useRef<HTMLDivElement>(null);

  useEffect(() => {
    end.current?.scrollIntoView();
  }, [lines]);

  return (
    <div className="log-panel" role="dialog" aria-label="遊戲日誌">
      <header>
        <h2>遊戲日誌</h2>
        <button className="text-button" onClick={onClose}>
          關閉
        </button>
      </header>
      <div className="log-lines">
        {lines.map((line, index) => (
          <p key={index}>{line}</p>
        ))}
        <div ref={end} />
      </div>
    </div>
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
