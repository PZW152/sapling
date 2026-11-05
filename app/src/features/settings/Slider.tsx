import type { CSSProperties } from "react";

export function Slider({
  id,
  min,
  max,
  step,
  value,
  disabled,
  onChange,
}: {
  id: string;
  min: number;
  max: number;
  step?: number;
  value: number;
  disabled?: boolean;
  onChange: (value: number) => void;
}) {
  return (
    <input
      id={id}
      type="range"
      min={min}
      max={max}
      step={step}
      value={value}
      disabled={disabled}
      style={{ "--fill": `${((value - min) / (max - min)) * 100}%` } as CSSProperties}
      onChange={(event) => onChange(Number(event.target.value))}
    />
  );
}
