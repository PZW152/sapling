import { useEffect, useLayoutEffect, useRef } from "react";

/**
 * 按 Esc 時關閉。`scope` 看不見時不處理，分頁切走後頁面還留著，不能被背景的頁面攔走。
 * 輸入框裡還有字時，Esc 只留給輸入框，避免打到一半被關掉。
 */
export function useEscape(
  onEscape: (() => void) | undefined,
  scope: { current: HTMLElement | null },
) {
  useEffect(() => {
    if (!onEscape) {
      return;
    }
    function handle(event: KeyboardEvent) {
      const field = event.target instanceof HTMLInputElement && event.target.value !== "";
      const visible = (scope.current?.getClientRects().length ?? 0) > 0;
      if (event.key === "Escape" && visible && !field) {
        onEscape?.();
      }
    }
    window.addEventListener("keydown", handle);
    return () => window.removeEventListener("keydown", handle);
  }, [onEscape, scope]);
}

/**
 * 畫面換掉時把焦點移到新內容，離開時還給原來的按鈕，
 * 否則用鍵盤或讀屏軟體的人會被丟回頁面最上面。
 */
export function useFocusHandoff<T extends HTMLElement>() {
  const target = useRef<T>(null);
  const previous = useRef<Element | null>(null);

  useLayoutEffect(() => {
    previous.current ??= document.activeElement;
    target.current?.focus();
    return () => {
      // 原來的內容這時可能還沒顯示回來，等下一幀才能聚焦。
      requestAnimationFrame(() => {
        if (!target.current?.isConnected && previous.current instanceof HTMLElement) {
          previous.current.focus();
        }
      });
    };
  }, []);

  return target;
}
