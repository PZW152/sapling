import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api } from "../../shell/api";
import { useApp } from "../../shell/AppContext";
import { trackDownload } from "../../shell/downloads";
import type { Feature } from "../../shell/feature";
import type { AddedKind } from "../../shell/types";
import "./drop-install.css";

const MESSAGE_MS = 6000;
const PACK_SUFFIX = ".mrpack";

const KIND_NAMES: Record<AddedKind, string> = {
  mod: "模組",
  resourcePack: "資源包",
  shader: "光影",
};

function fileName(path: string) {
  return path.split(/[\\/]/).pop() ?? path;
}

/** 把模組、資源包、光影或整合包拖進窗口就能安裝。 */
function DropOverlay() {
  const { selected, loadInstances, selectInstance, touchContent } = useApp();
  const [dragging, setDragging] = useState(false);
  const [messages, setMessages] = useState<{ text: string; failed: boolean }[]>([]);
  // 事件只訂閱一次，要用最新的實例。
  const target = useRef(selected);
  target.current = selected;

  async function install(path: string) {
    const name = fileName(path);
    if (name.toLowerCase().endsWith(PACK_SUFFIX)) {
      const instance = await trackDownload(name, () => api.installLocalPack(path));
      await loadInstances();
      selectInstance(instance.id);
      return `已建立實例「${instance.name}」`;
    }
    const instance = target.current;
    if (!instance) {
      throw new Error("先建立一個實例，才能放模組和資源。");
    }
    const kind = await api.addInstanceFile(instance.id, path);
    return `${name} 已加入「${instance.name}」的${KIND_NAMES[kind]}`;
  }

  async function installAll(paths: string[]) {
    const results = await Promise.all(
      paths.map((path) =>
        install(path).then(
          (text) => ({ text, failed: false }),
          (reason) => ({ text: String(reason).replace(/^Error: /, ""), failed: true }),
        ),
      ),
    );
    touchContent();
    setMessages(results);
  }

  const handler = useRef(installAll);
  handler.current = installAll;

  useEffect(() => {
    const stop = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "enter" || payload.type === "over") {
        setDragging(true);
        return;
      }
      setDragging(false);
      if (payload.type === "drop") {
        handler.current(payload.paths);
      }
    });
    return () => {
      stop.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    if (messages.length === 0) {
      return;
    }
    const timer = setTimeout(() => setMessages([]), MESSAGE_MS);
    return () => clearTimeout(timer);
  }, [messages]);

  return (
    <>
      {dragging && (
        <div className="drop-overlay">
          <p>放開就能安裝</p>
          <small>
            模組、資源包和光影會裝到「{selected?.name ?? "目前的實例"}」，整合包會建立新實例
          </small>
        </div>
      )}
      <div className="drop-results" role="status">
        {messages.map(({ text, failed }) => (
          <p key={text} className={failed ? "failed" : undefined}>
            {text}
          </p>
        ))}
      </div>
    </>
  );
}

export const dropInstall: Feature = {
  id: "drop-install",
  name: "拖放安裝",
  description: "把模組、資源包、光影或整合包拖進窗口就能安裝",
  optional: true,
  Overlay: DropOverlay,
};
