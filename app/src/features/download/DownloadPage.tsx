import { useState } from "react";
import type { CSSProperties } from "react";
import { useApp } from "../../shell/AppContext";
import { ErrorText } from "../../shell/ErrorText";
import { FeatureBoundary } from "../../shell/FeatureBoundary";
import { LOADERS, LOADER_HINT } from "../../shell/loaders";
import { formatSpeed, useSpeed } from "../../shell/progress";
import { Segments } from "../../shell/Segments";
import { DownloadQueue } from "./DownloadQueue";
import type { LoaderKind } from "../../shell/types";
import { groupBySeries } from "./series";
import { useInstall } from "./useInstall";
import type { InstallState } from "./useInstall";

const GAME_SECTION = "game";

/** 頂部分頁切換內容。全部保持掛載，切走時下載進度不會消失。 */
export function DownloadPage() {
  const { features } = useApp();
  const [current, setCurrent] = useState(GAME_SECTION);
  const sections = features.flatMap((feature) =>
    (feature.downloadSections ?? []).map((section) => ({ ...section, feature: feature.name })),
  );
  const active = sections.some(({ id }) => id === current) ? current : GAME_SECTION;

  return (
    <div className="download-page">
      <div className="download-bar">
        {sections.length > 0 && (
          <Segments
            label="下載內容"
            items={[{ id: GAME_SECTION, label: "遊戲" }, ...sections]}
            current={active}
            onChoose={setCurrent}
          />
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
        <div className="field">
          <span>{LOADER_HINT}</span>
          <div className="download-options">
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
            </div>
            <div className="toggle-row">
              <button
                className="switch"
                role="switch"
                aria-checked={snapshots}
                aria-label="顯示快照版"
                onClick={() => setSnapshots(!snapshots)}
              />
              <span>顯示快照版</span>
            </div>
          </div>
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
  const speed = useSpeed(active ? install.progress : null);

  return (
    <div className={`vine-row${isInstalled ? " installed" : ""}${active ? " active" : ""}`}>
      <span className="vine-leaf" aria-hidden="true" />
      <span className="vine-id">
        {id}
        {isLatest && <small>最新</small>}
      </span>
      <div className="vine-actions">
        {isInstalled ? (
          <button className="secondary" aria-label={`去啟動 ${id}`} onClick={() => onPlay(id)}>
            去啟動
          </button>
        ) : (
          <button
            className="secondary"
            disabled={install !== null && !install.error}
            aria-label={`下載 ${id}`}
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
            <span>
              {speed > 0 && `${formatSpeed(speed)}，`}
              {Math.round(install.percent * 100)}%
            </span>
          </p>
          <div
            className="progress"
            role="progressbar"
            aria-label={`下載 ${id} 的進度`}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(install.percent * 100)}
          >
            <div style={{ transform: `scaleX(${install.percent})` }} />
          </div>
        </div>
      )}
      {failed && <ErrorText className="vine-progress" message={install.error ?? ""} />}
    </div>
  );
}
