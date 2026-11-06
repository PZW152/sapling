import { useEffect, useLayoutEffect, useRef } from "react";

/** 讓標記平滑滑到容器裡目前選中的項目。 */
export function useSlideMark<T extends HTMLElement>(dependency: unknown) {
  const container = useRef<T>(null);

  useLayoutEffect(() => {
    const element = container.current;
    const active = element?.querySelector<HTMLElement>('[aria-current="page"], [aria-current="true"]');
    if (!element) {
      return;
    }
    element.style.setProperty("--mark-y", `${active?.offsetTop ?? 0}px`);
    element.style.setProperty("--mark-h", `${active?.offsetHeight ?? 0}px`);
    element.style.setProperty("--mark-opacity", active ? "1" : "0");
  }, [dependency]);

  return container;
}

/** 按下按鈕時，從點擊位置擴散一圈水波。 */
export function useRipples() {
  useEffect(() => {
    function spawn(event: PointerEvent) {
      const button = (event.target as Element).closest<HTMLElement>(
        ".primary, .secondary, .row, .instance-item, .theme, .chip, .world",
      );
      if (!button || button.matches(":disabled")) {
        return;
      }
      const bounds = button.getBoundingClientRect();
      const size = Math.max(bounds.width, bounds.height) * 2;
      const ripple = document.createElement("span");
      ripple.className = "ripple";
      ripple.style.width = ripple.style.height = `${size}px`;
      ripple.style.left = `${event.clientX - bounds.left - size / 2}px`;
      ripple.style.top = `${event.clientY - bounds.top - size / 2}px`;
      ripple.addEventListener("animationend", () => ripple.remove());
      button.append(ripple);
    }
    document.addEventListener("pointerdown", spawn);
    return () => document.removeEventListener("pointerdown", spawn);
  }, []);
}
