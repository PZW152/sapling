import { useState } from "react";
import { api } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { loaderName } from "../../shell/loaders";
import type { ProjectKind } from "../../shell/types";
import { ResourceBrowser } from "./ResourceBrowser";

/** 下載頁的分頁：先選實例，只列出和它相容的內容，不會裝錯版本。 */
function ResourceDownloads({ kind, hint }: { kind: ProjectKind; hint?: string }) {
  const { instances, selected, goTo } = useApp();
  const usable = instances.filter((instance) => kind !== "mod" || instance.loader !== "vanilla");
  const [chosenId, setChosenId] = useState("");
  const chosen =
    usable.find((instance) => instance.id === chosenId) ??
    usable.find((instance) => instance.id === selected?.id) ??
    usable[0];

  if (!chosen) {
    return (
      <section className="panel mod-section empty-detail">
        <p>
          {kind === "mod"
            ? "模組要搭配載入器。先新增一個使用 Fabric、NeoForge 或 Forge 的實例。"
            : "先下載一個遊戲版本，才有地方安裝。"}
        </p>
        <button className="secondary" onClick={() => goTo("instances")}>
          前往實例
        </button>
      </section>
    );
  }

  return (
    <section className="panel mod-section">
      <label className="field">
        <span>安裝到</span>
        <select value={chosen.id} onChange={(event) => setChosenId(event.target.value)}>
          {usable.map((instance) => (
            <option key={instance.id} value={instance.id}>
              {instance.name}（{instance.version}，{loaderName(instance.loader)}）
            </option>
          ))}
        </select>
      </label>
      {hint && <p className="hint">{hint}</p>}
      <ResourceBrowser
        key={chosen.id}
        kind={kind}
        instance={chosen}
        install={async (hit, versionId) => {
          await api.installProject(chosen.id, kind, hit.projectId, versionId);
        }}
      />
    </section>
  );
}

export function ModDownloads() {
  return <ResourceDownloads kind="mod" />;
}

export function ResourcePackDownloads() {
  return <ResourceDownloads kind="resourcePack" hint="裝好後在遊戲的「選項、資源包」裡啟用。" />;
}

export function ShaderDownloads() {
  return (
    <ResourceDownloads kind="shader" hint="光影需要 Iris 或 OptiFine，請先在模組分頁安裝 Iris。" />
  );
}
