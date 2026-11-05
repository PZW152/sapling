import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import type { VersionList } from "./types";

export function VersionPicker({
  list,
  selected,
  disabled,
  onSelect,
}: {
  list: VersionList | null;
  selected: string;
  disabled: boolean;
  onSelect: (id: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const options = useRef<HTMLUListElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    options.current?.querySelector<HTMLElement>("[aria-current]")?.focus();

    function closeOnOutsideClick(event: PointerEvent) {
      if (!root.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    function closeOnEscape(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape") {
        setOpen(false);
        trigger.current?.focus();
      }
    }
    document.addEventListener("pointerdown", closeOnOutsideClick);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOnOutsideClick);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [open]);

  function moveFocus(event: KeyboardEvent<HTMLUListElement>) {
    const step = { ArrowDown: 1, ArrowUp: -1 }[event.key];
    if (!step) {
      return;
    }
    event.preventDefault();
    const buttons = [...(options.current?.querySelectorAll("button") ?? [])];
    const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
    buttons[current + step]?.focus();
  }

  return (
    <div className="picker" ref={root}>
      {open && list && (
        <ul className="picker-list" ref={options} aria-label="選擇遊戲版本" onKeyDown={moveFocus}>
          {list.versions.map((id) => (
            <li key={id}>
              <button
                className="picker-option"
                aria-current={id === selected ? "true" : undefined}
                onClick={() => {
                  onSelect(id);
                  setOpen(false);
                  trigger.current?.focus();
                }}
              >
                <span>{id}</span>
                {id === list.latest && <small>最新</small>}
              </button>
            </li>
          ))}
        </ul>
      )}
      <button
        ref={trigger}
        className="picker-trigger"
        aria-expanded={open}
        disabled={!list || disabled}
        onClick={() => setOpen(!open)}
      >
        <span>{list ? selected : "正在取得版本"}</span>
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path d="M2.5 7.5 6 4l3.5 3.5" />
        </svg>
      </button>
    </div>
  );
}
