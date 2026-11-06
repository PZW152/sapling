import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { ConfirmButton } from "../../shell/ConfirmButton";
import { ErrorText } from "../../shell/ErrorText";
import { LOADERS } from "../../shell/loaders";
import type { LoaderKind } from "../../shell/types";

const FOLDERS = [
  { id: "saves", name: "存檔" },
  { id: "screenshots", name: "截圖" },
  { id: "logs", name: "日誌" },
  { id: "crash-reports", name: "崩潰報告" },
];

/** 實例頁的管理分頁：不常用但需要時一定要有的操作。 */
export function InstanceManage() {
  const { list, selected, loadInstances, loadVersions, touchContent } = useApp();
  const [version, setVersion] = useState(selected?.version ?? "");
  const [loader, setLoader] = useState<LoaderKind>(selected?.loader ?? "vanilla");
  const [working, setWorking] = useState("");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  if (!selected) {
    return null;
  }
  const instance = selected;
  const versions = list?.versions.includes(instance.version)
    ? list.versions
    : [instance.version, ...(list?.versions ?? [])];
  const changed = version !== instance.version || loader !== instance.loader;

  async function run(label: string, action: () => Promise<string>) {
    setWorking(label);
    setMessage("");
    setError("");
    try {
      setMessage(await action());
    } catch (reason) {
      setError(String(reason));
    }
    setWorking("");
  }

  function changeVersion() {
    run("version", async () => {
      await api.changeInstanceVersion(instance.id, version, loader);
      await loadInstances();
      await loadVersions();
      return `已換成 ${version}。需要的檔案會在下次啟動時下載。`;
    });
  }

  function repair() {
    run("repair", async () => {
      await api.repairInstance(instance.id);
      return "下次啟動時會檢查全部遊戲檔案，並重新下載有問題的檔案。";
    });
  }

  async function chooseIcon() {
    const path = await open({
      title: "選擇圖示",
      filters: [{ name: "圖片", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
    });
    if (path) {
      run("icon", async () => {
        await api.setInstanceIcon(instance.id, path);
        touchContent();
        return "圖示已更換。";
      });
    }
  }

  function removeIcon() {
    run("icon", async () => {
      await api.setInstanceIcon(instance.id, null);
      touchContent();
      return "圖示已移除。";
    });
  }

  function shortcut() {
    run("shortcut", async () => {
      await api.createShortcut(instance.id);
      return "桌面捷徑已建立。";
    });
  }

  async function exportPack() {
    const path = await save({
      title: "匯出整合包",
      defaultPath: `${instance.name}.mrpack`,
      filters: [{ name: "Modrinth 整合包", extensions: ["mrpack"] }],
    });
    if (path) {
      run("export", async () => {
        const { linkedMods, bundledMods } = await api.exportPack(instance.id, path);
        return `已匯出。${linkedMods} 個模組由 Modrinth 下載，${bundledMods} 個放在檔案裡。`;
      });
    }
  }

  return (
    <div className="instance-settings">
      <section className="setting">
        <div className="setting-head">
          <label htmlFor="instance-version">遊戲版本</label>
        </div>
        <div className="manage-row">
          <select
            id="instance-version"
            value={version}
            onChange={(event) => setVersion(event.target.value)}
          >
            {versions.map((id) => (
              <option key={id} value={id}>
                {id}
              </option>
            ))}
          </select>
          <select
            aria-label="載入器"
            value={loader}
            onChange={(event) => setLoader(event.target.value as LoaderKind)}
          >
            {LOADERS.map(({ kind, name }) => (
              <option key={kind} value={kind}>
                {name}
              </option>
            ))}
          </select>
          <ConfirmButton
            className="secondary"
            label={working === "version" ? "正在更換" : "更換"}
            warning="再按一次更換"
            disabled={!changed || working !== ""}
            onConfirm={changeVersion}
          />
        </div>
        <p>模組可能不相容。換成比較舊的版本可能弄壞世界，建議先在世界分頁備份。</p>
      </section>

      <section className="setting">
        <div className="setting-head">
          <span>資料夾</span>
        </div>
        <div className="folder-links">
          {FOLDERS.map(({ id, name }) => (
            <button
              key={id}
              className="text-button"
              onClick={() =>
                api.openInstanceSubfolder(instance.id, id).catch((reason) => setError(String(reason)))
              }
            >
              {name}
            </button>
          ))}
        </div>
      </section>

      <section className="setting">
        <div className="setting-head">
          <span>圖示</span>
          <div className="mods-actions">
            <button className="text-button" disabled={working !== ""} onClick={chooseIcon}>
              更換
            </button>
            <button className="text-button" disabled={working !== ""} onClick={removeIcon}>
              移除
            </button>
          </div>
        </div>
      </section>

      <section className="setting">
        <div className="setting-head">
          <span>桌面捷徑</span>
          <button className="text-button" disabled={working !== ""} onClick={shortcut}>
            建立
          </button>
        </div>
        <p>按兩下捷徑就直接啟動這個實例。</p>
      </section>

      <section className="setting">
        <div className="setting-head">
          <span>修復遊戲檔案</span>
          <button className="text-button" disabled={working !== ""} onClick={repair}>
            檢查並修復
          </button>
        </div>
        <p>遊戲打不開或一直崩潰時可以試試，不會動到存檔和模組。</p>
      </section>

      <section className="setting">
        <div className="setting-head">
          <span>匯出整合包</span>
          <button className="text-button" disabled={working !== ""} onClick={exportPack}>
            {working === "export" ? "正在匯出" : "匯出"}
          </button>
        </div>
        <p>把模組、設定、資源包和光影打包成 .mrpack，可以分享給朋友。存檔不會包含在內。</p>
      </section>

      {message && (
        <p className="hint" role="status">
          {message}
        </p>
      )}
      <ErrorText message={error} />
    </div>
  );
}
