/** 預設的綠色主題。顏色值在 App.css 的 `[data-theme]` 裡。 */
export const THEMES = [
  { id: "sapling", name: "樹苗", color: "#2f9e5b" },
  { id: "mint", name: "薄荷", color: "#2ba58a" },
  { id: "moss", name: "苔蘚", color: "#6a8f35" },
  { id: "bamboo", name: "竹林", color: "#4f9e3a" },
  { id: "forest", name: "深林", color: "#1f6f4a" },
];

const REVEAL_MS = 700;

/** 切換主題。傳入點擊位置時，新顏色會從那裡擴散開來。 */
export function applyTheme(theme: string, origin?: { x: number; y: number }) {
  const root = document.documentElement;
  const change = () => {
    root.dataset.theme = theme;
  };
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
