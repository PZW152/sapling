import { useRef, useState } from "react";
import type { CSSProperties } from "react";
import { api } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { ConfirmButton } from "../../shell/ConfirmButton";
import { ErrorText } from "../../shell/ErrorText";
import { FeatureBoundary } from "../../shell/FeatureBoundary";
import { InstanceIcon } from "../../shell/InstanceIcon";
import { useEscape } from "../../shell/keyboard";
import { LOADERS, LOADER_HINT, loaderName } from "../../shell/loaders";
import { matchesInstance } from "../../shell/instances";
import { Segments } from "../../shell/Segments";
import { formatPlaytime, timeAgo } from "../../shell/time";
import type { Instance, LoaderKind } from "../../shell/types";

/** 實例超過這個數量才顯示搜尋框。 */
const FILTER_FROM = 8;

export function InstancePage() {
  const { instances, list, selected, selectInstance, loadInstances } = useApp();
  const [creating, setCreating] = useState(false);
  // 放在這裡，換實例時才會停在同一個分頁。
  const [panel, setPanel] = useState("");
  const [filter, setFilter] = useState("");
  const shown = instances.filter((instance) => matchesInstance(instance, filter));

  return (
    <div className="instances">
      <aside className="panel instance-list">
        {instances.length >= FILTER_FROM && (
          <input
            type="search"
            className="instance-filter"
            aria-label="搜尋實例"
            placeholder="搜尋實例"
            spellCheck={false}
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
          />
        )}
        <ul>
          {shown.map((instance, index) => (
            <li key={instance.id} style={{ "--i": index } as CSSProperties}>
              <button
                className="instance-item"
                aria-current={instance.id === selected?.id ? "true" : undefined}
                title={`${instance.name}，${instance.version}`}
                onClick={() => {
                  setCreating(false);
                  selectInstance(instance.id);
                }}
              >
                <InstanceIcon id={instance.id} name={instance.name} />
                <span className="instance-item-text">
                  <span>{instance.name}</span>
                  <small>{instance.version}</small>
                </span>
              </button>
            </li>
          ))}
        </ul>
        <button className="secondary" onClick={() => setCreating(true)}>
          新增實例
        </button>
      </aside>

      {creating ? (
        <CreateInstance
          versions={list?.installed ?? []}
          onCancel={() => setCreating(false)}
          onCreated={async (id) => {
            await loadInstances();
            selectInstance(id);
            setCreating(false);
          }}
        />
      ) : selected ? (
        <InstanceDetail
          key={selected.id}
          instance={selected}
          panel={panel}
          onPanel={setPanel}
          onChanged={loadInstances}
          onDuplicated={async (id) => {
            await loadInstances();
            selectInstance(id);
          }}
        />
      ) : (
        <div className="panel instance-detail empty-detail">
          <p>還沒有實例。到下載頁下載一個版本，就會自動建立。</p>
        </div>
      )}
    </div>
  );
}

function CreateInstance({
  versions,
  onCancel,
  onCreated,
}: {
  versions: string[];
  onCancel: () => void;
  onCreated: (id: string) => Promise<void>;
}) {
  const [name, setName] = useState("");
  const [version, setVersion] = useState(versions[0] ?? "");
  const [loader, setLoader] = useState<LoaderKind>("vanilla");
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState("");
  const form = useRef<HTMLElement>(null);

  useEscape(onCancel, form);

  async function create() {
    setError("");
    setCreating(true);
    try {
      const instance = await api.createInstance(name, version, loader);
      await onCreated(instance.id);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setCreating(false);
    }
  }

  if (versions.length === 0) {
    return (
      <div className="panel instance-detail empty-detail">
        <p>先到下載頁下載一個遊戲版本，才能建立實例。</p>
      </div>
    );
  }

  return (
    <section className="panel instance-detail" ref={form}>
      <h2>新增實例</h2>
      <label className="field">
        <span>名稱</span>
        <input
          autoFocus
          value={name}
          placeholder="例如：生存世界"
          onChange={(event) => setName(event.target.value)}
          onKeyDown={(event) => event.key === "Enter" && !creating && create()}
        />
      </label>
      <div className="field">
        <span>遊戲版本</span>
        <div className="chips" role="group" aria-label="遊戲版本">
          {versions.map((id) => (
            <button
              key={id}
              className="chip"
              aria-pressed={id === version}
              onClick={() => setVersion(id)}
            >
              {id}
            </button>
          ))}
        </div>
      </div>
      <div className="field">
        <span>{LOADER_HINT}</span>
        <div className="chips" role="group" aria-label="載入器">
          {LOADERS.map(({ kind, name: label }) => (
            <button
              key={kind}
              className="chip"
              aria-pressed={kind === loader}
              onClick={() => setLoader(kind)}
            >
              {label}
            </button>
          ))}
        </div>
      </div>
      <ErrorText message={error} />
      <div className="detail-actions">
        <button className="secondary strong" disabled={creating} onClick={create}>
          {creating ? "正在建立" : "建立"}
        </button>
        <button className="secondary" onClick={onCancel}>
          取消
        </button>
      </div>
    </section>
  );
}

function InstanceDetail({
  instance,
  panel,
  onPanel,
  onChanged,
  onDuplicated,
}: {
  instance: Instance;
  panel: string;
  onPanel: (id: string) => void;
  onChanged: () => Promise<void>;
  onDuplicated: (id: string) => Promise<void>;
}) {
  const { features, goTo } = useApp();
  const panels = features
    .flatMap(({ id, name, instancePanel }) =>
      instancePanel && (instancePanel.available?.(instance) ?? true)
        ? [{ ...instancePanel, id, feature: name }]
        : [],
    )
    .sort((first, second) => first.order - second.order);
  const active = panels.find(({ id }) => id === panel) ?? panels[0];
  const [name, setName] = useState(instance.name);
  const [copying, setCopying] = useState(false);
  const [error, setError] = useState("");

  async function run(action: () => Promise<unknown>) {
    setError("");
    try {
      await action();
    } catch (reason) {
      setError(String(reason));
    }
  }

  function rename() {
    if (name.trim() !== instance.name) {
      run(async () => {
        await api.saveInstance({ ...instance, name });
        await onChanged();
      });
    }
  }

  async function duplicate() {
    setCopying(true);
    await run(async () => {
      const copy = await api.duplicateInstance(instance.id);
      await onDuplicated(copy.id);
    });
    setCopying(false);
  }

  function deleteInstance() {
    run(async () => {
      await api.deleteInstance(instance.id);
      await onChanged();
    });
  }

  return (
    <section className="panel instance-detail">
      <label className="field">
        <span>名稱</span>
        <input
          value={name}
          onChange={(event) => setName(event.target.value)}
          onBlur={rename}
          onKeyDown={(event) => event.key === "Enter" && event.currentTarget.blur()}
        />
      </label>
      <p className="detail-meta">
        {[
          `Minecraft ${instance.version}`,
          loaderName(instance.loader),
          formatPlaytime(instance.playedSeconds),
          instance.lastPlayedMs > 0 && `${timeAgo(instance.lastPlayedMs, Date.now())}玩過`,
        ]
          .filter(Boolean)
          .join("，")}
      </p>

      {panels.length > 1 && (
        <Segments
          label="實例內容"
          items={panels}
          current={active.id}
          onChoose={onPanel}
        />
      )}
      <div className="detail-panel">
        {active && (
          <FeatureBoundary key={active.id} name={active.feature}>
            <active.Panel key={instance.id} instanceId={instance.id} />
          </FeatureBoundary>
        )}
      </div>

      <ErrorText message={error} />
      <div className="detail-actions">
        <div className="detail-group">
          <button className="secondary strong" onClick={() => goTo("launch")}>
            去啟動
          </button>
          <button
            className="secondary"
            onClick={() => run(() => api.openInstanceFolder(instance.id))}
          >
            打開資料夾
          </button>
          <button className="secondary" disabled={copying} onClick={duplicate}>
            {copying ? "正在複製" : "複製實例"}
          </button>
        </div>
        <ConfirmButton
          className="secondary"
          label="刪除實例"
          warning="再按一次刪除（存檔也會刪除）"
          onConfirm={deleteInstance}
        />
      </div>
    </section>
  );
}
