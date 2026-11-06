import { useLayoutEffect, useRef } from "react";
import { applyLiquid } from "./liquid";

const STRETCH_MS = 420;

/** 標記這次往哪個方向移動。沒移動時回傳 null。 */
function stretchDirection(container: HTMLElement, active: HTMLElement | null | undefined) {
  if (!active) {
    return null;
  }
  const x = parseFloat(container.style.getPropertyValue("--mark-x"));
  const y = parseFloat(container.style.getPropertyValue("--mark-y"));
  if (Number.isNaN(x) || Number.isNaN(y)) {
    return null;
  }
  if (Math.abs(active.offsetTop - y) > 1) {
    return "vertical";
  }
  return Math.abs(active.offsetLeft - x) > 1 ? "horizontal" : null;
}

/** 標記滑動時像水滴一樣拉長，到了再彈回去。 */
function stretch(mark: HTMLElement, direction: "vertical" | "horizontal") {
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    return;
  }
  mark.animate(
    { scale: ["1 1", direction === "vertical" ? "0.93 1.14" : "1.1 0.9", "1 1"] },
    { duration: STRETCH_MS, easing: "cubic-bezier(0.3, 0.7, 0.3, 1)", composite: "replace" },
  );
}

/** 讓標記平滑滑到容器裡目前選中的項目。 */
export function useSlideMark<T extends HTMLElement>(dependency: unknown) {
  const container = useRef<T>(null);

  useLayoutEffect(() => {
    const element = container.current;
    if (!element) {
      return;
    }
    function measure() {
      const active = element?.querySelector<HTMLElement>(
        '[aria-current="page"], [aria-current="true"]',
      );
      // 頁面隱藏時量不到尺寸。回到頁面或窗口變大時，下面的觀察器會再量一次。
      if (!element || (active && active.offsetHeight === 0)) {
        return;
      }
      const moved = stretchDirection(element, active);
      element.style.setProperty("--mark-x", `${active?.offsetLeft ?? 0}px`);
      element.style.setProperty("--mark-y", `${active?.offsetTop ?? 0}px`);
      element.style.setProperty("--mark-w", `${active?.offsetWidth ?? 0}px`);
      element.style.setProperty("--mark-h", `${active?.offsetHeight ?? 0}px`);
      element.style.setProperty("--mark-opacity", active ? "1" : "0");
      const mark = element.querySelector<HTMLElement>(":scope > .slide-mark");
      if (active && mark) {
        applyLiquid(mark, mark.offsetWidth || active.offsetWidth, active.offsetHeight);
        if (moved) {
          stretch(mark, moved);
        }
      }
    }
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [dependency]);

  return container;
}
