import type { CSSProperties } from "react";
import type { InstallState } from "./App";
import type { VersionList } from "./types";

export function DownloadPage({
  list,
  listError,
  install,
  onRetry,
  onInstall,
  onPlay,
}: {
  list: VersionList | null;
  listError: string | null;
  install: InstallState | null;
  onRetry: () => void;
  onInstall: (id: string) => void;
  onPlay: (id: string) => void;
}) {
  if (!list) {
    return (
      <div className="notice">
        <div className="notice-card">
          <p className={listError ? "error" : undefined}>{listError ?? "正在取得版本"}</p>
          {listError && (
            <button className="secondary" onClick={onRetry}>
              重新取得版本
            </button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="download panel">
      <div className="download-head">
        <h2>下載遊戲</h2>
        <p>下載好的版本會自動建立實例。需要的 Java 會一起下載。</p>
      </div>
      <ul className="vine">
        {groupBySeries(list.versions).map(({ series, versions }, index) => (
          <li key={series} className="vine-series" style={{ "--i": index } as CSSProperties}>
            <p className="vine-title">{series}</p>
            {versions.map((id) => (
              <VersionRow
                key={id}
                id={id}
                isLatest={id === list.latest}
                isInstalled={list.installed.includes(id)}
                install={install}
                onInstall={onInstall}
                onPlay={onPlay}
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

function groupBySeries(versions: string[]) {
  const groups: { series: string; versions: string[] }[] = [];
  for (const id of versions) {
    const series = seriesOf(id);
    const last = groups[groups.length - 1];
    if (last?.series === series) {
      last.versions.push(id);
    } else {
      groups.push({ series, versions: [id] });
    }
  }
  return groups;
}

/** `1.21.4` 屬於 1.21，`26.1.2` 屬於 26。 */
function seriesOf(id: string) {
  const parts = id.split(".");
  return parts[0] === "1" ? `${parts[0]}.${parts[1]}` : parts[0];
}
