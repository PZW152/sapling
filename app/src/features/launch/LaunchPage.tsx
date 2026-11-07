import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { api, onLaunchRequest } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { ErrorText } from "../../shell/ErrorText";
import { FeatureBoundary } from "../../shell/FeatureBoundary";
import { loaderName } from "../../shell/loaders";
import { useSlideMark } from "../../shell/motion";
import type { Instance, Profile } from "../../shell/types";
import { AccountPanel } from "./AccountPanel";
import { LaunchOverlay } from "./LaunchOverlay";
import { LogPanel } from "./LogPanel";
import { useLaunch } from "./useLaunch";
import type { Phase } from "./useLaunch";
import { versionUnits } from "./versionUnits";

export function LaunchPage() {
  const {
    loaded,
    config,
    instances,
    selected,
    listError,
    selectInstance,
    goTo,
    list,
    loadInstances,
    loadVersions,
    features: active,
    isEnabled,
  } = useApp();
  const [profile, setProfile] = useState<Profile | null>(null);
  const { phase, progress, error, logs, origin, start, reset, clearOrigin } = useLaunch();
  const [logOpen, setLogOpen] = useState(false);
  const [pending, setPending] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [quickError, setQuickError] = useState("");
  const hero = useRef<HTMLElement>(null);

  const offlineName =
    active.map((feature) => feature.offlineName?.(config)).find((name) => name) ?? null;
  const progressView = active.map((feature) => feature.launchProgress?.(config)).find(Boolean);
  const launchBlock =
    active.map((feature) => feature.launchBlock?.(config)).find((reason) => reason) ?? null;
  const canStart = launchBlock === null && (profile !== null || offlineName !== null);

  const busy = phase === "preparing" || phase === "running";

  // 從桌面捷徑打開，或第一次按「開始玩」時，等登入好就自動啟動。
  useEffect(() => {
    api.takePendingLaunch().then(setPending);
    return onLaunchRequest(setPending);
  }, []);

  useEffect(() => {
    if (!pending || !loaded) {
      return;
    }
    if (busy || !instances.some(({ id }) => id === pending)) {
      setPending(null);
    } else if (selected?.id !== pending) {
      selectInstance(pending);
    } else if (canStart) {
      setPending(null);
      goTo("launch");
      startGame();
    }
  }, [pending, loaded, busy, instances, selected?.id, canStart]);

  async function quickStart() {
    if (!list) {
      goTo("download");
      return;
    }
    setCreating(true);
    setQuickError("");
    try {
      const instance = await api.createInstance(list.latest, list.latest, "vanilla");
      await loadInstances();
      setPending(instance.id);
    } catch (reason) {
      setQuickError(String(reason));
    }
    setCreating(false);
  }

  function startGame(world: string | null = null, server: string | null = null) {
    if (!selected) {
      return;
    }
    const bounds = hero.current?.getBoundingClientRect();
    start(
      selected.id,
      offlineName,
      world,
      server,
      bounds
        ? {
            top: bounds.top,
            right: window.innerWidth - bounds.right,
            bottom: window.innerHeight - bounds.bottom,
            left: bounds.left,
          }
        : null,
    );
  }

  function choose(id: string) {
    selectInstance(id);
    reset();
  }

  const noInstance = instances.length === 0;

  return (
    <div className="launch">
      <section className={`hero ${phase}`} ref={hero}>
        {!loaded ? (
          <div className="hero-top" aria-hidden="true">
            <span className="skeleton on-dark name-skeleton" />
            <span className="skeleton on-dark number-skeleton" />
          </div>
        ) : noInstance ? (
          <div className="hero-top">
            <p className="hero-name">歡迎使用 Sapling</p>
            <p className="hero-welcome">兩步就能開始玩</p>
          </div>
        ) : (
          <div className="hero-top">
            <p className="hero-name">
              {selected?.name}，{selected && loaderName(selected.loader)}
            </p>
            <VersionNumber version={selected?.version ?? ""} />
          </div>
        )}

        <div className="hero-bottom">
          {selected &&
            active.map(
              ({ id, HeroPanel }) =>
                HeroPanel && (
                  <FeatureBoundary key={id}>
                    <HeroPanel
                      instance={selected}
                      busy={busy}
                      canStart={canStart}
                      onPlay={startGame}
                      onJoin={(address) => startGame(null, address)}
                    />
                  </FeatureBoundary>
                ),
            )}
          <ErrorText message={quickError} />
          <HeroStatus
            phase={phase}
            error={error}
            note={launchNote(launchBlock, offlineName, profile)}
            noInstance={noInstance}
            hasLogs={logs.length > 0}
            onShowLogs={() => setLogOpen(true)}
            onShowReports={() =>
              selected && api.openInstanceSubfolder(selected.id, "crash-reports").catch(() => {})
            }
          />
          {noInstance && loaded ? (
            <button className="primary" disabled={creating} onClick={quickStart}>
              <span className="primary-label">
                {creating ? "準備中" : list ? `開始玩 ${list.latest}` : "前往下載"}
              </span>
            </button>
          ) : (
            <button
              className={phase === "running" ? "primary running" : "primary"}
              disabled={!selected || !canStart || busy}
              onClick={() => startGame()}
            >
              <span key={phase} className="primary-label">
                {launchLabel(phase)}
              </span>
            </button>
          )}
        </div>

        {logOpen && <LogPanel lines={logs} onClose={() => setLogOpen(false)} />}
      </section>

      {origin && selected && (
        <LaunchOverlay
          origin={origin}
          closing={!busy}
          running={phase === "running"}
          name={selected.name}
          version={selected.version}
          progress={progress}
          ProgressView={progressView}
          onClosed={clearOrigin}
        />
      )}

      <aside className="side">
        <AccountPanel
          profile={profile}
          onProfileChange={setProfile}
          disabled={busy}
          offline={offlineName !== null}
        />
        {active.map(
          ({ id, AccountPanel: Extra }) =>
            Extra && (
              <FeatureBoundary key={id}>
                <Extra profile={profile} disabled={busy} />
              </FeatureBoundary>
            ),
        )}
        <InstanceList
          loaded={loaded}
          instances={instances}
          selected={selected}
          disabled={busy}
          error={listError}
          canManage={isEnabled("instances")}
          onManage={() => goTo("instances")}
          onDownload={() => goTo("download")}
          onRetry={loadVersions}
          onChoose={choose}
        />
      </aside>
    </div>
  );
}

/** 主視覺：換版本時一個字一個字跳上來。 */
function VersionNumber({ version }: { version: string }) {
  return (
    <h2
      className="version-number"
      aria-label={version}
      style={{ "--units": versionUnits(version) } as CSSProperties}
    >
      <span key={version} aria-hidden="true">
        {[...version].map((character, index) => (
          <span key={index} className="digit" style={{ "--i": index } as CSSProperties}>
            {character}
          </span>
        ))}
      </span>
    </h2>
  );
}

function InstanceList({
  loaded,
  instances,
  selected,
  disabled,
  error,
  canManage,
  onManage,
  onDownload,
  onRetry,
  onChoose,
}: {
  loaded: boolean;
  instances: Instance[];
  selected: Instance | null;
  disabled: boolean;
  error: string | null;
  canManage: boolean;
  onManage: () => void;
  onDownload: () => void;
  onRetry: () => void;
  onChoose: (id: string) => void;
}) {
  const list = useSlideMark<HTMLUListElement>(`${selected?.id}-${instances.length}`);

  return (
    <section className="panel instance-tile">
      <div className="head">
        <h2 className="label">實例</h2>
        {canManage && instances.length > 0 && (
          <button className="text-button" onClick={onManage}>
            管理
          </button>
        )}
      </div>
      {error && (
        <>
          <ErrorText message={error} />
          <button className="text-button" onClick={onRetry}>
            重試
          </button>
        </>
      )}
      {!loaded && <span className="skeleton row-skeleton" aria-hidden="true" />}
      {loaded && instances.length === 0 && !error && (
        <p className="hint">下載好的版本會出現在這裡。</p>
      )}
      <ul className="row-list" ref={list}>
        <li className="slide-mark row-mark" aria-hidden="true" />
        {instances.map((instance) => (
          <li key={instance.id}>
            <button
              className="row"
              disabled={disabled}
              aria-current={instance.id === selected?.id ? "true" : undefined}
              title={`${instance.name}，${instance.version}`}
              onClick={() => onChoose(instance.id)}
            >
              <span>{instance.name}</span>
              <small>{instance.version}</small>
            </button>
          </li>
        ))}
      </ul>
      {instances.length > 0 && (
        <button className="text-button more-versions" onClick={onDownload}>
          下載其他版本
        </button>
      )}
    </section>
  );
}

function launchLabel(phase: Phase) {
  if (phase === "preparing") {
    return "準備中";
  }
  if (phase === "running") {
    return "遊戲運行中";
  }
  return phase === "failed" ? "再試一次" : "啟動遊戲";
}

/** 按下啟動前要讓用戶知道的事，例如這次用哪個身分進遊戲。 */
function launchNote(block: string | null, offlineName: string | null, profile: Profile | null) {
  if (block) {
    return block;
  }
  if (offlineName) {
    return `這次用離線名稱 ${offlineName} 啟動。`;
  }
  return profile ? null : "登入微軟帳號後就能啟動遊戲。";
}

function HeroStatus({
  phase,
  error,
  note,
  noInstance,
  hasLogs,
  onShowLogs,
  onShowReports,
}: {
  phase: Phase;
  error: string;
  note: string | null;
  noInstance: boolean;
  hasLogs: boolean;
  onShowLogs: () => void;
  onShowReports: () => void;
}) {
  if (phase === "failed") {
    return (
      <div className="hero-error" role="alert">
        <p>{error}</p>
        <div className="mods-actions">
          {hasLogs && (
            <button className="text-button" onClick={onShowLogs}>
              查看問題記錄
            </button>
          )}
          <button className="text-button" onClick={onShowReports}>
            打開崩潰報告
          </button>
        </div>
      </div>
    );
  }
  if (phase === "preparing") {
    return null;
  }
  if (phase === "running") {
    return (
      <p className="hero-note" role="status">
        遊戲已啟動，關閉遊戲後可以再次啟動。
      </p>
    );
  }
  if (noInstance) {
    return (
      <p className="hero-note">
        {note ? "登入微軟帳號，再按下面的按鈕，遊戲會自動下載。" : "按下面的按鈕，遊戲會自動下載。"}
      </p>
    );
  }
  return note && <p className="hero-note">{note}</p>;
}
