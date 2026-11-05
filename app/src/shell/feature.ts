import type { ComponentType } from "react";

/** 一個內建功能模塊。新功能只需要實作這個結構並到 features/index.ts 註冊。 */
export type Feature = {
  id: string;
  /** 顯示在設定頁模塊清單的名稱和說明。 */
  name: string;
  description: string;
  /** 可選模塊可以在設定裡關閉。 */
  optional: boolean;
  /** 頂部分頁。 */
  tab?: {
    label: string;
    Page: ComponentType;
    /** 切換分頁時保留狀態，不卸載頁面。 */
    keepAlive?: boolean;
    /** 固定在側欄底部。 */
    pinned?: boolean;
  };
  /** 放進設定頁的區塊。 */
  Settings?: ComponentType;
  /** 放進設定頁「進階設定」的區塊，平時收起。 */
  AdvancedSettings?: ComponentType;
  /** 放進實例頁的區塊。 */
  InstancePanel?: ComponentType<{ instanceId: string }>;
  /** 浮在所有頁面上的內容，例如更新提示。 */
  Overlay?: ComponentType;
};
