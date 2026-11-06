import { useState } from "react";
import type { CSSProperties } from "react";
import { FEATURES } from "..";
import { useApp } from "../../shell/AppContext";
import { FeatureBoundary } from "../../shell/FeatureBoundary";
import { LOADERS } from "../../shell/loaders";
import { DownloadQueue } from "./DownloadQueue";
import type { LoaderKind } from "../../shell/types";
import { groupBySeries } from "./series";
import { useInstall } from "./useInstall";
import type { InstallState } from "./useInstall";

const GAME_SECTION = "game";

/** 頂部分頁切換內容。全部保持掛載，切走時下載進度不會消失。 */
export function DownloadPage() {
  const { isEnabled } = useApp();
  const [current, setCurrent] = useState(GAME_SECTION);
  const sections = FEATURES.filter(
    (feature) => !feature.optional || isEnabled(feature.id),
  ).flatMap((feature) =>
    (feature.downloadSections ?? []).map((section) => ({ ...section, feature: feature.name })),
  );
  const active = sections.some(({ id }) => id === current) ? current : GAME_SECTION;

  function tab(id: string, label: string) {
    return (
      <button
        key={id}
        className="segment"
        aria-current={id === active ? "page" : undefined}
        onClick={() => setCurrent(id)}
      >
        {label}
      </button>
    );
  }

  return (
    <div className="download-page">
      <div className="download-bar">
        {sections.length > 0 && (
          <nav className="segments" aria-label="下載內容">
            {tab(GAME_SECTION, "遊戲")}
            {sections.map(({ id, label }) => tab(id, label))}
          </nav>
        )}
        <DownloadQueue />
      </div>
      <div className="download-section" hidden={active !== GAME_SECTION}>
        <GameDownloads />
      </div>
      {sections.map(({ id, feature, Panel }) => (
        <div key={id} className="download-section" hidden={active !== id}>
          <FeatureBoundary name={feature}>
            <Panel />
          </FeatureBoundary>
        </div>
      ))}
    </div>
  );
}

function GameDownloads() {
  const { list, listError, loadVersions } = useApp();
  const { install, installGame, playVersion } = useInstall();
  const [loader, setLoader] = useState<LoaderKind>("vanilla");
  const [snapshots, setSnapshots] = useState(false);

  if (!list && !listError) {
    return (
      <div className="download panel" aria-label="正在取得版本">
        <span className="skeleton name-skeleton" />
        <div className="skeleton-list">
          <span className="skeleton row-skeleton" />
          <span className="skeleton row-skeleton" />
          <span className="skeleton row-skeleton" />
          <span className="skeleton row-skeleton" />
        </div>
      </div>
    );
  }

  if (!list) {
    return (
      <div className="notice">
        <div className="notice-card">
          <h2>無法取得版本</h2>
          <p>{listError}</p>
          <button className="secondary" onClick={loadVersions}>
            重新取得版本
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="download panel">
      <div className="download-head">
        <h2>下載遊戲</h2>
        <p>下載好的版本會自動建立實例，需要的 Java 會一起下載。</p>
        <div className="chips" role="group" aria-label="載入器">
          {LOADERS.map(({ kind, name }) => (
            <button
              key={kind}
              className="chip"
              aria-pressed={kind === loader}
              onClick={() => setLoader(kind)}
            >
              {name}
            </button>
          ))}
          <button
            className="chip"
            aria-pressed={snapshots}
            onClick={() => setSnapshots(!snapshots)}
          >
            快照版
          </button>
        </div>
      </div>
      <ul className="vine">
        {(snapshots
          ? [{ series: "快照版（測試用，可能不穩定）", versions: list.snapshots }]
          : groupBySeries(list.versions)
        ).map(({ series, versions }, index) => (
          <li key={series} className="vine-series" style={{ "--i": index } as CSSProperties}>
            <p className="vine-title">{series}</p>
            {versions.map((id) => (
              <VersionRow
                key={id}
                id={id}
                isLatest={id === list.latest}
                isInstalled={list.installed.includes(id)}
                install={install}
                onInstall={(version) => installGame(version, loader)}
                onPlay={(version) => playVersion(version, loader)}
              />
            ))}
          </li>
        ))}
      </ul>
    </div>
  );
}

function VersionRow({
  id,
  isLatest,
  isInstalled,
  install,
  onInstall,
  onPlay,
}: {
  id: string;
  isLatest: boolean;
  isInstalled: boolean;
  install: InstallState | null;
  onInstall: (id: string) => void;
  onPlay: (id: string) => void;
}) {
  const active = install?.id === id;
  const failed = active && install.error;

  return (
    <div className={`vine-row${isInstalled ? " installed" : ""}${active ? " active" : ""}`}>
      <span className="vine-leaf" aria-hidden="true" />
      <span className="vine-id">
        {id}
        {isLatest && <small>最新</small>}
      </span>
      <div className="vine-actions">
        {isInstalled ? (
          <button className="secondary" onClick={() => onPlay(id)}>
            去啟動
          </button>
        ) : (
          <button
            className="secondary"
            disabled={install !== null && !install.error}
            onClick={() => onInstall(id)}
          >
            {active && !failed ? "下載中" : "下載"}
          </button>
        )}
      </div>
      {active && !failed && (
        <div className="vine-progress">
          <p className="status status-row">
            <span>{install.progress?.stage === "java" ? "正在下載 Java" : "正在下載遊戲檔案"}</span>
            <span>{Math.round(install.percent * 100)}%</span>
          </p>
          <div
            className="progress"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(install.percent * 100)}
          >
            <div style={{ transform: `scaleX(${install.percent})` }} />
          </div>
        </div>
      )}
      {failed && <p className="error vine-progress">{install.error}</p>}
    </div>
  );
}
