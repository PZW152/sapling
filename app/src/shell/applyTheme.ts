import { Effect, getCurrentWindow } from "@tauri-apps/api/window";
import type { ColorMode } from "./types";

const REVEAL_MS = 900;

type Origin = { x: number; y: number };

/** 切換主題。傳入點擊位置時，新顏色會從那裡擴散開來。 */
export function applyTheme(theme: string, origin?: Origin) {
  revealChange(() => {
    document.documentElement.dataset.theme = theme;
  }, origin);
}

/** 切換深淺色。跟隨系統時看系統目前的設定。 */
export function applyColorMode(mode: ColorMode) {
  const dark =
    mode === "dark" ||
    (mode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.mode = dark ? "dark" : "light";
  // 窗口後面的系統玻璃也要跟著變深變淺。
  getCurrentWindow()
    .setTheme(dark ? "dark" : "light")
    .catch(() => {});
}

/** 切換玻璃和純色。用戶在 Windows 關掉透明效果或開高對比時，一律用純色。 */
export function applyMaterial(wanted: boolean) {
  const blocked =
    window.matchMedia("(prefers-reduced-transparency: reduce)").matches ||
    window.matchMedia("(forced-colors: active)").matches;
  const glass = wanted && !blocked;
  const root = document.documentElement;
  if (glass) {
    root.dataset.glass = "";
  } else {
    delete root.dataset.glass;
  }
  const win = getCurrentWindow();
  (glass ? win.setEffects({ effects: [Effect.Acrylic] }) : win.clearEffects()).catch(() => {});
}

/** 執行一次畫面改動，並讓新畫面從點擊位置擴散開來。 */
export function revealChange(change: () => void, origin?: Origin) {
  const root = document.documentElement;
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (!origin || reducedMotion || !document.startViewTransition) {
    change();
    return;
  }
  const radius = Math.hypot(window.innerWidth, window.innerHeight);
  document.startViewTransition(change).ready.then(() => {
    // 玻璃是半透明的，新舊畫面疊在一起會變濃，舊畫面要同時淡出。
    if ("glass" in root.dataset) {
      root.animate(
        { opacity: [1, 0] },
        { duration: REVEAL_MS, pseudoElement: "::view-transition-old(root)" },
      );
    }
    root.animate(
      {
        clipPath: [
          `circle(0px at ${origin.x}px ${origin.y}px)`,
          `circle(${radius}px at ${origin.x}px ${origin.y}px)`,
        ],
      },
      {
        duration: REVEAL_MS,
        easing: "cubic-bezier(0.22, 0.8, 0.28, 1)",
        pseudoElement: "::view-transition-new(root)",
      },
    );
  });
}
