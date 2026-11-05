import { useCallback, useEffect, useState } from "react";
import type { CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Instance, ModFile, VersionList } from "./types";

const BYTES_PER_MB = 1024 * 1024;

export function InstancePage({
  instances,
  list,
  selectedId,
  onSelect,
  onChanged,
}: {
  instances: Instance[];
  list: VersionList | null;
  selectedId: string;
  onSelect: (id: string) => void;
  onChanged: () => Promise<void>;
}) {
  const [creating, setCreating] = useState(false);
  const current = instances.find((instance) => instance.id === selectedId) ?? null;

  return (
    <div className="instances">
      <aside className="panel instance-list">
        <ul>
          {instances.map((instance, index) => (
            <li key={instance.id} style={{ "--i": index } as CSSProperties}>
              <button
                className="instance-item"
                aria-current={instance.id === selectedId ? "true" : undefined}
                onClick={() => {
                  setCreating(false);
                  onSelect(instance.id);
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
            await onChanged();
            onSelect(id);
            setCreating(false);
          }}
        />
      ) : current ? (
        <InstanceDetail key={current.id} instance={current} onChanged={onChanged} />
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
  const [error, setError] = useState("");

  async function create() {
    setError("");
    try {
      const instance = await invoke<Instance>("create_instance", { name, version });
      await onCreated(instance.id);
    } catch (reason) {
      setError(String(reason));
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
      {error && <p className="error">{error}</p>}
      <div className="detail-actions">
        <button className="secondary strong" onClick={create}>
          建立
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
  const [name, setName] = useState(instance.name);
  const [mods, setMods] = useState<ModFile[]>([]);
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState("");

  const loadMods = useCallback(() => {
    invoke<ModFile[]>("list_mods", { id: instance.id })
      .then(setMods)
      .catch((reason) => setError(String(reason)));
  }, [instance.id]);

  useEffect(loadMods, [loadMods]);

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
        await invoke("save_instance", { instance: { ...instance, name } });
        await onChanged();
      });
    }
  }

  function modAction(command: string, file: ModFile, enabled = file.enabled) {
    run(async () => {
      await invoke(command, { action: { instanceId: instance.id, name: file.name, enabled } });
      loadMods();
    });
  }

  function deleteInstance() {
    if (!confirming) {
      setConfirming(true);
      return;
    }
    run(async () => {
      await invoke("delete_instance", { id: instance.id });
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
      <p className="detail-meta">Minecraft {instance.version}，原版</p>

      <div className="mods-head">
        <h3>模組</h3>
        <button
          className="text-button"
          onClick={() => run(() => invoke("open_mods_folder", { id: instance.id }))}
        >
          打開模組資料夾
        </button>
      </div>
      <ul className="mods">
        {mods.length === 0 && (
          <li className="mods-empty">
            沒有模組。把 .jar 放進模組資料夾就會出現在這裡。原版不會載入模組，要等之後支援載入器。
          </li>
        )}
        {mods.map((file) => (
          <li key={file.name} className={file.enabled ? "mod" : "mod off"}>
            <button
              className="switch"
              role="switch"
              aria-checked={file.enabled}
              aria-label={`${file.enabled ? "停用" : "啟用"} ${file.name}`}
              onClick={() => modAction("set_mod_enabled", file, !file.enabled)}
            />
            <span className="mod-name">{file.name}</span>
            <small>{(file.size / BYTES_PER_MB).toFixed(1)} MB</small>
            <button className="text-button" onClick={() => modAction("delete_mod", file)}>
              刪除
            </button>
          </li>
        ))}
      </ul>

      {error && <p className="error">{error}</p>}
      <div className="detail-actions">
        <button
          className="secondary"
          onClick={() => run(() => invoke("open_instance_folder", { id: instance.id }))}
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
