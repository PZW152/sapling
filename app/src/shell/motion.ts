import { useLayoutEffect, useRef } from "react";

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
      element.style.setProperty("--mark-x", `${active?.offsetLeft ?? 0}px`);
      element.style.setProperty("--mark-y", `${active?.offsetTop ?? 0}px`);
      element.style.setProperty("--mark-w", `${active?.offsetWidth ?? 0}px`);
      element.style.setProperty("--mark-h", `${active?.offsetHeight ?? 0}px`);
      element.style.setProperty("--mark-opacity", active ? "1" : "0");
    }
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [dependency]);

  return container;
}
