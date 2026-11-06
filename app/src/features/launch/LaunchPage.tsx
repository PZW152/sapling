import { useRef, useState } from "react";
import type { CSSProperties } from "react";
import { FEATURES } from "..";
import { useApp } from "../../shell/AppContext";
import { loaderName } from "../../shell/loaders";
import { useSlideMark } from "../../shell/motion";
import type { Instance, Profile } from "../../shell/types";
import { AccountPanel } from "./AccountPanel";
import { LaunchOverlay } from "./LaunchOverlay";
import { LogPanel } from "./LogPanel";
import { useLaunch } from "./useLaunch";
import type { Phase } from "./useLaunch";

export function LaunchPage() {
  const {
    loaded,
    config,
    instances,
    selected,
    listError,
    selectInstance,
    goTo,
    loadVersions,
    isEnabled,
  } = useApp();
  const [profile, setProfile] = useState<Profile | null>(null);
  const { phase, progress, error, logs, origin, start, reset, clearOrigin } = useLaunch();
  const [logOpen, setLogOpen] = useState(false);
  const hero = useRef<HTMLElement>(null);

  const active = FEATURES.filter((feature) => !feature.optional || isEnabled(feature.id));
  const offlineName =
    active.map((feature) => feature.offlineName?.(config)).find((name) => name) ?? null;
  const launchBlock =
    active.map((feature) => feature.launchBlock?.(config)).find((reason) => reason) ?? null;
  const canStart = launchBlock === null && (profile !== null || offlineName !== null);

  function startGame(world: string | null = null) {
    if (!selected) {
      return;
    }
    const bounds = hero.current?.getBoundingClientRect();
    start(
      selected.id,
      offlineName,
      world,
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

  const busy = phase === "preparing" || phase === "running";
  const noInstance = instances.length === 0;

  return (
    <div className="launch">
      <section className={`hero ${phase}`} ref={hero}>
        <p className="motto" aria-hidden="true">
          天下為公
        </p>
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
            <p className="hero-name" key={selected?.id}>
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
                  <HeroPanel
                    key={id}
                    instance={selected}
                    busy={busy}
                    canStart={canStart}
                    onPlay={startGame}
                  />
                ),
            )}
          <HeroStatus
            phase={phase}
            error={error}
            note={launchNote(launchBlock, offlineName, profile)}
            noInstance={noInstance}
            hasLogs={logs.length > 0}
            onShowLogs={() => setLogOpen(true)}
          />
          {noInstance && loaded ? (
            <button className="primary" onClick={() => goTo("download")}>
              前往下載
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
            Extra && <Extra key={id} profile={profile} disabled={busy} />,
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

/** 版本號的視覺字數：小數點很窄，只算半個字。 */
function versionUnits(version: string) {
  const dots = version.split(".").length - 1;
  return Math.max(version.length - dots / 2, 1);
}

/** 版本號換掉時，只有變動的字元彈入。 */
function VersionNumber({ version }: { version: string }) {
  return (
    <h2
      className="version-number"
      aria-label={version}
      style={{ "--units": versionUnits(version) } as CSSProperties}
    >
      {[...version].map((character, index) => (
        <span
          key={`${index}-${character}`}
          className="digit"
          aria-hidden="true"
          style={{ "--i": index } as CSSProperties}
        >
          {character}
        </span>
      ))}
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
          <p className="error">{error}</p>
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
        <span className="slide-mark row-mark" />
        {instances.map((instance) => (
          <li key={instance.id}>
            <button
              className="row"
              disabled={disabled}
              aria-current={instance.id === selected?.id ? "true" : undefined}
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
}: {
  phase: Phase;
  error: string;
  note: string | null;
  noInstance: boolean;
  hasLogs: boolean;
  onShowLogs: () => void;
}) {
  if (phase === "failed") {
    return (
      <div className="hero-error">
        <p>{error}</p>
        {hasLogs && (
          <button className="text-button" onClick={onShowLogs}>
            查看問題記錄
          </button>
        )}
      </div>
    );
  }
  if (phase === "preparing") {
    return null;
  }
  if (phase === "running") {
    return <p className="hero-note">遊戲已啟動，關閉遊戲後可以再次啟動。</p>;
  }
  if (noInstance) {
    return <p className="hero-note">下載一個遊戲版本，再登入微軟帳號。</p>;
  }
  return note && <p className="hero-note">{note}</p>;
}
