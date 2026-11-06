import { useCallback, useEffect, useState } from "react";
import { api } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { ConfirmButton } from "../../shell/ConfirmButton";
import { ErrorText } from "../../shell/ErrorText";
import type { Feature } from "../../shell/feature";
import { ResourceBrowser } from "../../shell/resources/ResourceBrowser";
import { formatSize } from "../../shell/resources/resources";
import type { PackFile, PackKind } from "../../shell/types";

const SECTIONS: { kind: PackKind; title: string; empty: string }[] = [
  {
    kind: "resourcePack",
    title: "資源包",
    empty: "還沒有資源包。裝好後在遊戲的「選項、資源包」裡啟用。",
  },
  {
    kind: "shader",
    title: "光影",
    empty: "還沒有光影。光影需要 Iris 或 OptiFine 才能使用。",
  },
];

function PackSection({
  instanceId,
  kind,
  title,
  empty,
  onBrowse,
}: {
  instanceId: string;
  kind: PackKind;
  title: string;
  empty: string;
  onBrowse: () => void;
}) {
  const [packs, setPacks] = useState<PackFile[]>([]);
  const [error, setError] = useState("");

  const load = useCallback(() => {
    api.listPacks(instanceId, kind)
      .then(setPacks)
      .catch((reason) => setError(String(reason)));
  }, [instanceId, kind]);

  useEffect(load, [load]);

  function remove(name: string) {
    setError("");
    api.deletePack(instanceId, kind, name)
      .then(load)
      .catch((reason) => setError(String(reason)));
  }

  return (
    <section className="packs">
      <div className="mods-head packs-head">
        <h2>{title}</h2>
        <div className="mods-actions">
          <button className="text-button" aria-label={`下載${title}`} onClick={onBrowse}>
            下載
          </button>
          <button
            className="text-button"
            aria-label={`打開${title}資料夾`}
            onClick={() =>
              api.openPackFolder(instanceId, kind).catch((reason) => setError(String(reason)))
            }
          >
            打開資料夾
          </button>
        </div>
      </div>
      {packs.length === 0 && <p className="mods-empty">{empty}</p>}
      <ul>
        {packs.map(({ name, size }) => (
          <li key={name} className="pack">
            <span className="mod-name" title={name}>
              {name}
            </span>
            {size > 0 && <small>{formatSize(size)}</small>}
            <ConfirmButton
              className="text-button"
              label="刪除"
              warning="再按一次刪除"
              subject={name}
              onConfirm={() => remove(name)}
            />
          </li>
        ))}
      </ul>
      <ErrorText message={error} />
    </section>
  );
}

/** 實例頁的資源分頁：下載過的資源包和光影都看得到，也刪得掉。 */
function PackPanel({ instanceId }: { instanceId: string }) {
  const { selected } = useApp();
  const [browsing, setBrowsing] = useState<PackKind | null>(null);

  if (browsing && selected) {
    return (
      <ResourceBrowser
        kind={browsing}
        instance={selected}
        install={async (hit, versionId) => {
          await api.installProject(selected.id, browsing, hit.projectId, versionId);
        }}
        onClose={() => setBrowsing(null)}
      />
    );
  }

  return (
    <>
      {SECTIONS.map((section) => (
        <PackSection
          key={section.kind}
          instanceId={instanceId}
          {...section}
          onBrowse={() => setBrowsing(section.kind)}
        />
      ))}
    </>
  );
}

export const packs: Feature = {
  id: "packs",
  name: "資源包和光影",
  description: "在實例頁查看和刪除資源包、光影",
  optional: true,
  instancePanel: { label: "資源", order: 3, Panel: PackPanel },
};
