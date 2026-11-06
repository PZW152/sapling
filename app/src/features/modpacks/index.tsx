import { useEffect, useState } from "react";
import { api, onInstallEvent } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import type { Feature } from "../../shell/feature";
import { ResourceBrowser } from "../mods/ResourceBrowser";

function ModpackPage() {
  const { goTo, loadInstances, selectInstance } = useApp();
  const [files, setFiles] = useState("");

  useEffect(
    () =>
      onInstallEvent((payload) => {
        if (payload.kind === "progress") {
          setFiles(`${payload.finishedFiles} / ${payload.totalFiles}`);
        }
      }),
    [],
  );

  return (
    <section className="panel mod-section">
      <p className="hint">整合包是別人配好的一整套模組，安裝後會建立新實例。</p>
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
