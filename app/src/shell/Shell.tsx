import { useCallback, useMemo, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { FEATURES } from "../features";
import { AppProvider, useApp } from "./AppContext";
import type { Feature } from "./feature";
import { ICONS } from "./icons";
import { useRipples, useSlideMark } from "./motion";

const HOME_TAB = "launch";

/** 基礎框架：窗口、導航和功能模塊的掛載點。業務內容都在 features/。 */
export function Shell() {
  const [activeTab, setActiveTab] = useState(HOME_TAB);
  const goTo = useCallback((featureId: string) => setActiveTab(featureId), []);

  return (
    <AppProvider activeTab={activeTab} goTo={goTo}>
      <Frame />
    </AppProvider>
  );
}

function Frame() {
  const { activeTab, isEnabled } = useApp();
  useRipples();
  const features = useMemo(
    () => FEATURES.filter((feature) => !feature.optional || isEnabled(feature.id)),
    [isEnabled],
  );
  const tabs = features.filter((feature) => feature.tab);
  const index = Math.max(
    tabs.findIndex((feature) => feature.id === activeTab),
    0,
  );
  const current = tabs[index];

  // 內容跟著導航方向滑入：往下一頁從下方進來，往回從上方進來。
  const lastIndex = useRef(index);
  const direction = index >= lastIndex.current ? 1 : -1;
  lastIndex.current = index;

  return (
    <div className="app" style={{ "--dir": direction } as CSSProperties}>
      <Rail tabs={tabs} current={current?.id ?? HOME_TAB} />
      <div className="body">
        <header className="top" data-tauri-drag-region>
          <h1 key={current?.id} data-tauri-drag-region>
            {current?.tab?.label}
          </h1>
          <WindowControls />
        </header>
        {tabs.map(({ id, tab }) => {
          if (!tab || (id !== current?.id && !tab.keepAlive)) {
            return null;
          }
          return (
            <main key={id} className="page" hidden={id !== current?.id}>
              <tab.Page />
            </main>
          );
        })}
      </div>
      {features.map(({ id, Overlay }) => Overlay && <Overlay key={id} />)}
    </div>
  );
}

function Rail({ tabs, current }: { tabs: Feature[]; current: string }) {
  const { goTo } = useApp();
  const rail = useSlideMark<HTMLElement>(`${current}-${tabs.length}`);

  function item({ id, tab }: Feature) {
    return (
      <button
        key={id}
        className="nav"
        aria-current={id === current ? "page" : undefined}
        onClick={() => goTo(id)}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          {ICONS[id]}
        </svg>
        {tab?.label}
      </button>
    );
  }

  return (
    <nav className="rail" ref={rail} aria-label="主要分頁" data-tauri-drag-region>
      <img className="mark" src="/logo.svg" alt="Sapling" width={32} height={32} />
      <span className="slide-mark rail-mark" />
      {tabs.filter(({ tab }) => !tab?.pinned).map(item)}
      <div className="push" data-tauri-drag-region />
      {tabs.filter(({ tab }) => tab?.pinned).map(item)}
    </nav>
  );
}

function WindowControls() {
  const appWindow = getCurrentWindow();

  return (
    <div className="window-controls">
      <button aria-label="最小化" onClick={() => appWindow.minimize()}>
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path d="M2 6h8" />
        </svg>
      </button>
      <button aria-label="最大化" onClick={() => appWindow.toggleMaximize()}>
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <rect x="2.5" y="2.5" width="7" height="7" rx="1" />
        </svg>
      </button>
      <button className="close" aria-label="關閉" onClick={() => appWindow.close()}>
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" />
        </svg>
      </button>
    </div>
  );
}
