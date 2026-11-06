import { useState } from "react";

/** 會刪除資料的按鈕：第一下變紅提醒，第二下才執行，移開焦點或按 Esc 就取消。 */
export function ConfirmButton({
  className,
  label,
  warning,
  subject,
  disabled,
  onConfirm,
}: {
  className: string;
  label: string;
  /** 按第一下之後顯示的文字。 */
  warning: string;
  /** 讀屏軟體要知道這個按鈕作用在什麼東西上。 */
  subject?: string;
  disabled?: boolean;
  onConfirm: () => void;
}) {
  const [confirming, setConfirming] = useState(false);
  const text = confirming ? warning : label;

  return (
    <button
      className={confirming ? `${className} danger` : className}
      disabled={disabled}
      aria-label={subject ? `${text} ${subject}` : undefined}
      onClick={() => {
        if (confirming) {
          onConfirm();
        }
        setConfirming(!confirming);
      }}
      onBlur={() => setConfirming(false)}
      onKeyDown={(event) => {
        if (event.key === "Escape" && confirming) {
          event.stopPropagation();
          setConfirming(false);
        }
      }}
    >
      {text}
    </button>
  );
}
