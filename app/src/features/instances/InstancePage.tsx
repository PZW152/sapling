import { useState } from "react";
import type { CSSProperties } from "react";
import { FEATURES } from "..";
import { api } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { LOADERS, loaderName } from "../../shell/loaders";
import type { Instance, LoaderKind } from "../../shell/types";

export function InstancePage() {
  const { instances, list, selected, selectInstance, loadInstances } = useApp();
  const [creating, setCreating] = useState(false);

  return (
    <div className="instances">
      <aside className="panel instance-list">
        <ul>
          {instances.map((instance, index) => (
            <li key={instance.id} style={{ "--i": index } as CSSProperties}>
              <button
                className="instance-item"
                aria-current={instance.id === selected?.id ? "true" : undefined}
                onClick={() => {
                  setCreating(false);
                  selectInstance(instance.id);
                }}
              >
                <span>{instance.name}</span>
                <small>{instance.version}</small>
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
          onChanged={loadInstances}
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
    <section className="panel instance-detail">
      <h2>新增實例</h2>
      <label className="field">
        <span>名稱</span>
        <input
          autoFocus
          value={name}
          placeholder="例如：生存世界"
          onChange={(event) => setName(event.target.value)}
        />
      </label>
      <div className="field">
        <span>遊戲版本</span>
        <div className="chips">
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
        <span>載入器</span>
        <div className="chips">
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
      {error && <p className="error">{error}</p>}
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
  onChanged,
}: {
  instance: Instance;
  onChanged: () => Promise<void>;
}) {
  const { isEnabled } = useApp();
  const panels = FEATURES.filter((feature) => feature.InstancePanel && isEnabled(feature.id));
  const [name, setName] = useState(instance.name);
  const [confirming, setConfirming] = useState(false);
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

  function deleteInstance() {
    if (!confirming) {
      setConfirming(true);
      return;
    }
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
        Minecraft {instance.version}，{loaderName(instance.loader)}
      </p>

      {panels.length > 0 ? (
        panels.map(
          ({ id, InstancePanel }) =>
            InstancePanel && <InstancePanel key={id} instanceId={instance.id} />,
        )
      ) : (
        <div className="detail-spacer" />
      )}

      {error && <p className="error">{error}</p>}
      <div className="detail-actions">
        <button
          className="secondary"
          onClick={() => run(() => api.openInstanceFolder(instance.id))}
        >
          打開實例資料夾
        </button>
        <button
          className={confirming ? "secondary danger" : "secondary"}
          onClick={deleteInstance}
          onBlur={() => setConfirming(false)}
        >
          {confirming ? "再按一次刪除（存檔也會刪除）" : "刪除實例"}
        </button>
      </div>
    </section>
  );
}
