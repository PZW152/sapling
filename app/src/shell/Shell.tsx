import { useCallback, useMemo, useState } from "react";
import type { CSSProperties } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { FEATURES } from "../features";
import { AppProvider, useApp } from "./AppContext";
import { Scene } from "./Scene";

const HOME_TAB = "launch";

/** 基礎框架：窗口、分頁和功能模塊的掛載點。業務內容都在 features/。 */
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
  const { activeTab, goTo, isEnabled } = useApp();
  const features = useMemo(
    () => FEATURES.filter((feature) => !feature.optional || isEnabled(feature.id)),
    [isEnabled],
  );
  const tabs = features.filter((feature) => feature.tab);
  const index = Math.max(
    tabs.findIndex((feature) => feature.id === activeTab),
    0,
  );
  const current = tabs[index]?.id ?? HOME_TAB;

  return (
    <div className="app">
      <Scene />
      <header className="titlebar" data-tauri-drag-region>
        <div className="brand" data-tauri-drag-region>
          <img src="/logo.svg" alt="" width={24} height={24} data-tauri-drag-region />
          <span data-tauri-drag-region>Sapling</span>
        </div>
        <nav
          className="tabs"
          aria-label="主要分頁"
          style={{ "--tab-count": tabs.length } as CSSProperties}
        >
          {tabs.map((feature) => (
            <button
              key={feature.id}
              className="tab"
              aria-current={feature.id === current ? "page" : undefined}
              onClick={() => goTo(feature.id)}
            >
              {feature.tab?.label}
            </button>
          ))}
          <span
            className="tab-indicator"
            style={{ transform: `translateX(calc(${index} * var(--tab-width)))` }}
          />
        </nav>
        <WindowControls />
      </header>

      {tabs.map(({ id, tab }) => {
        if (!tab || (id !== current && !tab.keepAlive)) {
          return null;
        }
        return (
          <main key={id} className="page" hidden={id !== current}>
            <tab.Page />
          </main>
        );
      })}

      {features.map(({ id, Overlay }) => Overlay && <Overlay key={id} />)}
    </div>
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
