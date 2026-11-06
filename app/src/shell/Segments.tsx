/** 一排分頁按鈕。 */
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
  return (
    <nav className="segments" aria-label={label}>
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
