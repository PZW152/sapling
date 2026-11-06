import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { api } from "../api";
import { useApp } from "../AppContext";
import { trackDownload } from "../downloads";
import { ErrorText } from "../ErrorText";
import { useEscape, useFocusHandoff } from "../keyboard";
import { timeAgo } from "../time";
import type { Instance, ModHit, ProjectKind, ProjectVersion } from "../types";
import { formatDownloads } from "./downloads";
import "./resources.css";
import { displayTitle, searchTerm } from "./names";
import {
  CATEGORY_NAMES,
  SORT_OPTIONS,
  appendHits,
  compatibleVersions,
  formatSize,
  hitTags,
  versionSummary,
} from "./resources";

/** 停止輸入多久後才搜尋，避免每打一個字都發請求。 */
const SEARCH_DELAY_MS = 350;
const MODRINTH_PROJECT = "https://modrinth.com/project/";

const PLACEHOLDERS: Record<ProjectKind, string> = {
  mod: "搜尋模組名稱",
  modpack: "搜尋整合包名稱",
  resourcePack: "搜尋資源包名稱",
  shader: "搜尋光影名稱",
  dataPack: "搜尋資料包名稱",
};

const KIND_NAMES: Record<ProjectKind, string> = {
  mod: "模組",
  modpack: "整合包",
  resourcePack: "資源包",
  shader: "光影",
  dataPack: "資料包",
};

type Props = {
  kind: ProjectKind;
  /** 目標實例。有實例時只列出它裝得上的內容，否則可以自己選遊戲版本。 */
  instance?: Instance | null;
  /** `versionId` 是空的代表裝最適合的版本。 */
  install: (hit: ModHit, versionId: string | null) => Promise<void>;
  /** 安裝途中顯示在按鈕上的文字。 */
  busyLabel?: string;
  /** 在實例頁開啟時才有，下載頁的分頁不需要返回。 */
  onClose?: () => void;
};

/** 搜尋、篩選和安裝 Modrinth 的資源。模組、資源包、光影和整合包都用它。 */
export function ResourceBrowser({ kind, instance, install, busyLabel, onClose }: Props) {
  const { list, config, updateConfig } = useApp();
  const [query, setQuery] = useState("");
  const [gameVersion, setGameVersion] = useState("");
  const [category, setCategory] = useState("");
  const [sort, setSort] = useState("");
  const [hits, setHits] = useState<ModHit[]>([]);
  const [total, setTotal] = useState(0);
  /** Modrinth 已經回傳幾個。重複的不會顯示，所以不能用 `hits.length` 當翻頁位置。 */
  const [fetched, setFetched] = useState(0);
  const [searching, setSearching] = useState(true);
  const [installing, setInstalling] = useState<string[]>([]);
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  const [installed, setInstalled] = useState<string[]>([]);
  const [error, setError] = useState("");
  const [detail, setDetail] = useState<ModHit | null>(null);
  const browser = useRef<HTMLDivElement>(null);
  const listScroll = useRef(0);

  useEscape(onClose, browser);

  const scroller = () => browser.current?.closest<HTMLElement>(".mod-section, .detail-panel");

  function openDetail(hit: ModHit) {
    listScroll.current = scroller()?.scrollTop ?? 0;
    setDetail(hit);
  }

  // 詳情頁從最上面開始，回到清單時停在原本的位置。
  useLayoutEffect(() => {
    const element = scroller();
    if (element) {
      element.scrollTop = detail ? 0 : listScroll.current;
    }
  }, [detail]);

  const targetVersion = instance?.version ?? gameVersion;
  const favorites = config.favorites.filter((favorite) => favorite.kind === kind);
  const isFavorite = (hit: ModHit) =>
    favorites.some((favorite) => favorite.projectId === hit.projectId);

  function toggleFavorite(hit: ModHit) {
    updateConfig({
      favorites: isFavorite(hit)
        ? config.favorites.filter((favorite) => favorite.projectId !== hit.projectId)
        : [...config.favorites, { ...hit, kind }],
    });
  }

  const request = {
    query: searchTerm(query),
    kind,
    gameVersion: targetVersion || null,
    loader: kind === "mod" ? (instance?.loader ?? null) : null,
    category: category || null,
    sort: sort || null,
  };
  const requestKey = JSON.stringify(request);
  const latestRequest = useRef(requestKey);
  latestRequest.current = requestKey;

  useEffect(() => {
    let current = true;
    setSearching(true);
    const timer = setTimeout(() => {
      api
        .searchProjects({ ...JSON.parse(requestKey), offset: 0 })
        .then((page) => {
          if (current) {
            setHits(page.hits);
            setFetched(page.hits.length);
            setTotal(page.total);
            setError("");
          }
        })
        .catch((reason) => current && setError(String(reason)))
        .finally(() => current && setSearching(false));
    }, SEARCH_DELAY_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [requestKey]);

  async function loadMore() {
    // 等待途中換了篩選條件，這一頁就是舊的結果，不能接到新清單後面。
    const isCurrent = () => latestRequest.current === requestKey;
    setSearching(true);
    try {
      const page = await api.searchProjects({ ...request, offset: fetched });
      if (isCurrent()) {
        setHits((shown) => appendHits(shown, page.hits));
        setFetched(fetched + page.hits.length);
        setTotal(page.hits.length === 0 ? fetched : page.total);
      }
    } catch (reason) {
      if (isCurrent()) {
        setError(String(reason));
      }
    }
    if (isCurrent()) {
      setSearching(false);
    }
  }

  async function run(hit: ModHit, versionId: string | null) {
    const key = versionId ?? hit.projectId;
    setInstalling((running) => [...running, key]);
    setError("");
    try {
      await trackDownload(displayTitle(hit), () => install(hit, versionId));
      setInstalled((done) => [...done, hit.projectId, ...(versionId ? [versionId] : [])]);
    } catch (reason) {
      setError(String(reason));
    }
    setInstalling((running) => running.filter((item) => item !== key));
  }

  const shownHits = favoritesOnly ? favorites : hits;

  // 詳情頁打開時清單留著，回來時才會停在原本的位置。
  return (
    <>
      {detail && (
        <ProjectDetail
          hit={detail}
          kind={kind}
          instance={instance}
          installing={installing}
          installed={installed}
          busyLabel={busyLabel}
          error={error}
          favorite={isFavorite(detail)}
          onToggleFavorite={() => toggleFavorite(detail)}
          onInstall={(versionId) => run(detail, versionId)}
          onBack={() => setDetail(null)}
        />
      )}
    <div className="mod-browser" ref={browser} hidden={detail !== null}>
      {onClose && (
        <div className="mods-head">
          <h2>下載{KIND_NAMES[kind]}</h2>
          <button className="text-button" onClick={onClose}>
            返回
          </button>
        </div>
      )}
      <div className="resource-filters">
        <input
          type="search"
          className="resource-search"
          aria-label={PLACEHOLDERS[kind]}
          value={query}
          placeholder={PLACEHOLDERS[kind]}
          spellCheck={false}
          onChange={(event) => setQuery(event.target.value)}
        />
        {!instance && (
          <select
            aria-label="遊戲版本"
            value={gameVersion}
            onChange={(event) => setGameVersion(event.target.value)}
          >
            <option value="">全部版本</option>
            {list?.versions.map((id) => (
              <option key={id} value={id}>
                {id}
              </option>
            ))}
          </select>
        )}
        {kind === "mod" && (
          <select
            aria-label="分類"
            value={category}
            onChange={(event) => setCategory(event.target.value)}
          >
            <option value="">全部分類</option>
            {Object.entries(CATEGORY_NAMES).map(([id, name]) => (
              <option key={id} value={id}>
                {name}
              </option>
            ))}
          </select>
        )}
        <select aria-label="排序" value={sort} onChange={(event) => setSort(event.target.value)}>
          {SORT_OPTIONS.map(({ id, name }) => (
            <option key={id} value={id}>
              {name}
            </option>
          ))}
        </select>
        <button
          className="chip"
          aria-pressed={favoritesOnly}
          onClick={() => setFavoritesOnly(!favoritesOnly)}
        >
          收藏 {favorites.length}
        </button>
      </div>
      <ErrorText message={error} />
      {!error && searching && !favoritesOnly && hits.length === 0 && (
        <p className="hint" role="status">
          正在搜尋
        </p>
      )}
      {!error && (favoritesOnly || !searching) && shownHits.length === 0 && (
        <p className="mods-empty">
          {favoritesOnly
            ? "還沒有收藏。打開項目後按「收藏」就會出現在這裡。"
            : "找不到符合的內容，換個關鍵字或篩選條件試試。"}
        </p>
      )}
      <ul className="mod-hits">
        {shownHits.map((hit) => {
          const done = installed.includes(hit.projectId);
          return (
            <li key={hit.projectId}>
              <button className="hit-main" onClick={() => openDetail(hit)}>
                <HitIcon hit={hit} />
                <HitText hit={hit} />
              </button>
              <button
                className="secondary"
                disabled={done || installing.includes(hit.projectId)}
                aria-label={`${done ? "已安裝" : "安裝"} ${displayTitle(hit)}`}
                onClick={() => run(hit, null)}
              >
                {done
                  ? "已安裝"
                  : installing.includes(hit.projectId)
                    ? (busyLabel ?? "正在安裝")
                    : "安裝"}
              </button>
            </li>
          );
        })}
      </ul>
      {!favoritesOnly && hits.length > 0 && fetched < total && (
        <button className="secondary load-more" disabled={searching} onClick={loadMore}>
          {searching ? "載入中" : `載入更多（還有 ${total - fetched} 個）`}
        </button>
      )}
    </div>
    </>
  );
}

function HitIcon({ hit }: { hit: ModHit }) {
  return hit.iconUrl ? (
    <img src={hit.iconUrl} alt="" loading="lazy" />
  ) : (
    <span className="mod-hit-icon" aria-hidden="true" />
  );
}

function HitText({ hit }: { hit: ModHit }) {
  const updated = Date.parse(hit.dateModified);

  return (
    <span className="hit-text">
      <strong>{displayTitle(hit)}</strong>
      <span className="hit-description">{hit.description}</span>
      <span className="hit-meta">
        {hitTags(hit).map((tag) => (
          <span key={tag} className="hit-tag">
            {tag}
          </span>
        ))}
        <small>{formatDownloads(hit.downloads)} 次下載</small>
        {Number.isFinite(updated) && <small>{timeAgo(updated, Date.now())}更新</small>}
      </span>
    </span>
  );
}

function ProjectDetail({
  hit,
  kind,
  instance,
  installing,
  installed,
  busyLabel,
  error,
  favorite,
  onToggleFavorite,
  onInstall,
  onBack,
}: {
  hit: ModHit;
  kind: ProjectKind;
  instance?: Instance | null;
  installing: string[];
  installed: string[];
  busyLabel?: string;
  error: string;
  favorite: boolean;
  onToggleFavorite: () => void;
  onInstall: (versionId: string) => void;
  onBack: () => void;
}) {
  const [versions, setVersions] = useState<ProjectVersion[] | null>(null);
  const [loadError, setLoadError] = useState("");
  const root = useRef<HTMLDivElement>(null);
  const heading = useFocusHandoff<HTMLHeadingElement>();

  useEscape(onBack, root);

  useEffect(() => {
    api.projectVersions(hit.projectId).then(setVersions).catch((reason) => setLoadError(String(reason)));
  }, [hit.projectId]);

  const shown = versions && compatibleVersions(versions, instance, kind);

  return (
    <div className="mod-browser resource-detail" ref={root}>
      <button className="text-button detail-back" onClick={onBack}>
        返回列表
      </button>
      <header className="detail-head">
        <HitIcon hit={hit} />
        <div>
          <h2 ref={heading} tabIndex={-1}>
            {displayTitle(hit)}
          </h2>
          {hit.author && <small>作者 {hit.author}</small>}
          <p>{hit.description}</p>
          <div className="detail-links">
            <button className="text-button" aria-pressed={favorite} onClick={onToggleFavorite}>
              {favorite ? "已收藏" : "收藏"}
            </button>
            <button
              className="text-button"
              onClick={() => api.openLink(`${MODRINTH_PROJECT}${hit.projectId}`)}
            >
              在 Modrinth 查看
            </button>
          </div>
        </div>
      </header>
      <ErrorText message={error || loadError} />
      {!shown && !loadError && <p className="hint">正在取得版本</p>}
      {shown?.length === 0 && (
        <p className="mods-empty">
          沒有適合 {instance?.name} 的版本。可以換一個實例，或到 Modrinth 查看。
        </p>
      )}
      {shown && shown.length > 0 && (
        <>
          <h3>{instance ? `適合「${instance.name}」的版本` : "全部版本"}</h3>
          <ul className="version-list">
            {shown.map((version) => {
              const done = installed.includes(version.id);
              return (
                <li key={version.id}>
                  <div>
                    <strong>
                      {version.name}
                      {version.versionType !== "release" && (
                        <span className="hit-tag">測試版</span>
                      )}
                    </strong>
                    <small>{versionSummary(version)}</small>
                    <small>
                      {timeAgo(Date.parse(version.datePublished), Date.now())}，
                      {formatSize(version.size)}
                    </small>
                  </div>
                  <button
                    className="secondary"
                    disabled={done || installing.includes(version.id)}
                    aria-label={`${done ? "已安裝" : "安裝"} ${version.name}`}
                    onClick={() => onInstall(version.id)}
                  >
                    {done
                      ? "已安裝"
                      : installing.includes(version.id)
                        ? (busyLabel ?? "正在安裝")
                        : "安裝"}
                  </button>
                </li>
              );
            })}
          </ul>
        </>
      )}
    </div>
  );
}
