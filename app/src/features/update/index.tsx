import { useEffect, useState } from "react";
import { api } from "../../shell/api";
import type { Feature } from "../../shell/feature";
import type { UpdateInfo } from "../../shell/types";

type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "latest" }
  | { kind: "available"; update: UpdateInfo }
  | { kind: "installing" }
  | { kind: "failed"; message: string };

function useUpdate() {
  const [state, setState] = useState<UpdateState>({ kind: "idle" });

  async function check() {
    setState({ kind: "checking" });
    try {
      const found = await api.checkUpdate();
      setState(found ? { kind: "available", update: found } : { kind: "latest" });
    } catch (reason) {
      setState({ kind: "failed", message: String(reason) });
    }
  }

  async function install(update: UpdateInfo) {
    setState({ kind: "installing" });
    try {
      await api.applyUpdate(update);
    } catch (reason) {
      setState({ kind: "failed", message: String(reason) });
    }
  }

  return { state, check, install };
}

function UpdateSettings() {
  const { state, check, install } = useUpdate();
  const busy = state.kind === "checking" || state.kind === "installing";

  return (
    <section className="panel setting setting-wide update-setting">
      <div className="setting-head">
        <span>啟動器更新</span>
      </div>
      {state.kind === "available" ? (
        <button className="secondary strong" onClick={() => install(state.update)}>
          更新到 {state.update.version}
        </button>
      ) : (
        <button className="secondary" disabled={busy} onClick={check}>
          {busy ? (state.kind === "checking" ? "正在檢查" : "正在更新") : "檢查更新"}
        </button>
      )}
      {state.kind === "latest" && <p>已經是最新版本。</p>}
      {state.kind === "failed" && <p className="error">{state.message}</p>}
    </section>
  );
}

/** 打開啟動器時檢查一次，有新版本才顯示。檢查失敗不打擾用戶。 */
function UpdateNotice() {
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [error, setError] = useState("");
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    api.checkUpdate()
      .then(setUpdate)
      .catch(() => {});
  }, []);

  if (!update) {
    return null;
  }

  return (
    <aside className="toast">
      <p>{error || `新版本 ${update.version} 可以更新`}</p>
      <button
        className="text-button"
        disabled={installing}
        onClick={() => {
          setInstalling(true);
          api.applyUpdate(update).catch((reason) => {
            setError(String(reason));
            setInstalling(false);
          });
        }}
      >
        {installing ? "正在更新" : "立即更新"}
      </button>
      <button className="text-button" onClick={() => setUpdate(null)}>
        稍後
      </button>
    </aside>
  );
}

export const update: Feature = {
  id: "update",
  name: "自動更新",
  description: "打開時檢查新版本",
  optional: true,
  Settings: UpdateSettings,
  Overlay: UpdateNotice,
};
