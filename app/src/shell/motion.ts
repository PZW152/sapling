import { useEffect, useLayoutEffect, useRef } from "react";

const CHANGE_FADE_MS = 300;

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
      element.style.setProperty("--mark-y", `${active?.offsetTop ?? 0}px`);
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

/** 內容換掉時淡入。切分頁不算換掉，所以不用 CSS 動畫，否則每次回到頁面都會重播。 */
export function useChangeFade<T extends HTMLElement>(value: unknown) {
  const element = useRef<T>(null);
  const previous = useRef(value);

  useEffect(() => {
    if (previous.current === value) {
      return;
    }
    previous.current = value;
    if (!window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      element.current?.animate([{ opacity: 0 }, { opacity: 1 }], {
        duration: CHANGE_FADE_MS,
        easing: "cubic-bezier(0.2, 0, 0, 1)",
      });
    }
  }, [value]);

  return element;
}
