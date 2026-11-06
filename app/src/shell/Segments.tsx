import { useSlideMark } from "./motion";

/** 一排分頁按鈕。選中的底色會滑過去。 */
export function Segments({
  label,
  items,
  current,
  onChoose,
}: {
  label: string;
  items: { id: string; label: string }[];
  current: string;
  onChoose: (id: string) => void;
}) {
  const bar = useSlideMark<HTMLElement>(`${current}-${items.length}`);

  return (
    <nav className="segments" ref={bar} aria-label={label}>
      <span className="slide-mark segment-mark" />
      {items.map((item) => (
        <button
          key={item.id}
          className="segment"
          aria-current={item.id === current ? "page" : undefined}
          onClick={() => onChoose(item.id)}
        >
          {item.label}
        </button>
      ))}
    </nav>
  );
}
