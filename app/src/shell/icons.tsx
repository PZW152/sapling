import type { ReactNode } from "react";

/** 側欄圖示，按功能模塊的 id 對應。 */
export const ICONS: Record<string, ReactNode> = {
  launch: <path d="M7 4.5v15l12-7.5z" />,
  instances: (
    <>
      <rect x="4" y="4" width="16" height="6" rx="2" />
      <rect x="4" y="14" width="16" height="6" rx="2" />
    </>
  ),
  download: <path d="M12 4v11M7 11l5 5 5-5M5 20h14" />,
  settings: (
    <>
      <path d="M4 7h10M18 7h2M4 17h2M10 17h10" />
      <circle cx="16" cy="7" r="2" />
      <circle cx="8" cy="17" r="2" />
    </>
  ),
  about: (
    <>
      <circle cx="6" cy="12" r="1.2" />
      <circle cx="12" cy="12" r="1.2" />
      <circle cx="18" cy="12" r="1.2" />
    </>
  ),
};
