import { useEffect, useRef, useState } from "react";
import type { CSSProperties, KeyboardEvent } from "react";
import type { Instance } from "../../shell/types";

export function InstancePicker({
  instances,
  selected,
  disabled,
  onSelect,
}: {
  instances: Instance[];
  selected: Instance | null;
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
      {open && (
        <ul className="picker-list" ref={options} aria-label="選擇實例" onKeyDown={moveFocus}>
          {instances.map((instance, index) => (
            <li key={instance.id} style={{ "--i": index } as CSSProperties}>
              <button
                className="picker-option"
                aria-current={instance.id === selected?.id ? "true" : undefined}
                onClick={() => {
                  onSelect(instance.id);
                  setOpen(false);
                  trigger.current?.focus();
                }}
              >
                <span>{instance.name}</span>
                <small>{instance.version}</small>
              </button>
            </li>
          ))}
        </ul>
      )}
      <button
        ref={trigger}
        className="picker-trigger"
        aria-expanded={open}
        disabled={instances.length === 0 || disabled}
        onClick={() => setOpen(!open)}
      >
        <span>{selected?.name ?? "還沒有實例"}</span>
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path d="M2.5 7.5 6 4l3.5 3.5" />
        </svg>
      </button>
    </div>
  );
}
