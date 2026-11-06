const REVEAL_MS = 900;

type Origin = { x: number; y: number };

/** 切換主題。傳入點擊位置時，新顏色會從那裡擴散開來。 */
export function applyTheme(theme: string, origin?: Origin) {
  revealChange(() => {
    document.documentElement.dataset.theme = theme;
  }, origin);
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
