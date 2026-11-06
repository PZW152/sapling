import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, onPackEvent } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { trackDownload } from "../../shell/downloads";
import { ErrorText } from "../../shell/ErrorText";
import type { Feature } from "../../shell/feature";
import { ResourceBrowser } from "../../shell/resources/ResourceBrowser";

function ModpackPage() {
  const { goTo, loadInstances, selectInstance } = useApp();
  const [files, setFiles] = useState("");
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState("");

  async function importFile() {
    const path = await open({
      title: "選擇整合包",
      filters: [{ name: "Modrinth 整合包", extensions: ["mrpack"] }],
    });
    if (!path) {
      return;
    }
    setImporting(true);
    setError("");
    try {
      const instance = await trackDownload("匯入整合包", () => api.installLocalPack(path));
      await loadInstances();
      selectInstance(instance.id);
      goTo("launch");
    } catch (reason) {
      setError(String(reason));
    }
    setImporting(false);
  }

  useEffect(
    () =>
      onPackEvent((payload) => {
        if (payload.kind === "progress") {
          setFiles(`${payload.finishedFiles} / ${payload.totalFiles}`);
        }
      }),
    [],
  );

  return (
    <section className="panel mod-section">
      <div className="mods-head packs-head">
        <p className="hint">整合包是別人配好的一整套模組，安裝後會建立新實例。</p>
        <button className="text-button" disabled={importing} onClick={importFile}>
          {importing ? `正在匯入 ${files}` : "從檔案匯入"}
        </button>
      </div>
      <ErrorText message={error} />
      <ResourceBrowser
        kind="modpack"
        busyLabel={`正在安裝 ${files}`}
        install={async (hit, versionId) => {
          setFiles("");
          const instance = await api.installPack(hit.projectId, versionId, hit.title);
          await loadInstances();
          selectInstance(instance.id);
          goTo("launch");
        }}
      />
    </section>
  );
}

export const modpacks: Feature = {
  id: "modpacks",
  name: "整合包",
  description: "從 Modrinth 安裝別人配好的模組組合",
  optional: true,
  downloadSections: [{ id: "modpacks", label: "整合包", Panel: ModpackPage }],
};
