import type { ModHit } from "../types";

/** 熱門模組的中文名稱，按 Modrinth 的 slug 對應。這份清單由本專案自行整理。 */
const CHINESE_NAMES: Record<string, string> = {
  sodium: "鈉",
  iris: "鳶尾花",
  lithium: "鋰",
  "ferrite-core": "鐵氧體核心",
  entityculling: "實體剔除",
  modmenu: "模組選單",
  jei: "物品管理器",
  emi: "物品查詢",
  journeymap: "旅行地圖",
  "xaeros-minimap": "Xaero 小地圖",
  "xaeros-world-map": "Xaero 世界地圖",
  appleskin: "蘋果皮",
  jade: "方塊資訊",
  create: "機械動力",
  "farmers-delight": "農夫樂事",
  terralith: "地形重塑",
  waystones: "傳送石碑",
  "sophisticated-backpacks": "精緻背包",
  ae2: "應用能源 2",
  "simple-voice-chat": "簡易語音聊天",
  continuity: "連接紋理",
  "cloth-config": "設定介面前置",
  "architectury-api": "跨平台前置",
  geckolib: "動畫前置",
  yacl: "設定選單前置",
  balm: "跨平台前置 Balm",
  "mouse-tweaks": "滑鼠手勢",
  "inventory-profiles-next": "物品欄整理",
  shulkerboxtooltip: "界伏盒預覽",
  lambdynamiclights: "動態光源",
  "sodium-extra": "鈉額外選項",
  "reeses-sodium-options": "鈉選單強化",
  krypton: "氪",
  modernfix: "現代修復",
  immediatelyfast: "渲染加速",
  "c2me-fabric": "區塊並行引擎",
  "dynamic-fps": "動態幀率",
  "no-chat-reports": "停用聊天檢舉",
  worldedit: "創世神",
  carpet: "地毯",
  litematica: "投影",
  minihud: "迷你資訊",
  tweakeroo: "遊戲調整",
  "alexs-mobs": "亞歷克斯的生物",
  "biomes-o-plenty": "生物群系大全",
  cobblemon: "寶可夢模組",
  "ice-and-fire-dragons": "冰與火",
  "yungs-better-dungeons": "更好的地牢",
  supplementaries: "補充包",
  quark: "夸克",
  botania: "植物魔法",
  mekanism: "通用機械",
  "tinkers-construct": "匠魂",
  "sound-physics-remastered": "聲音物理",
  "presence-footsteps": "腳步聲",
  "eating-animation": "進食動畫",
  "3dskinlayers": "3D 皮膚",
  zoomify: "望遠縮放",
  bobby: "遠距區塊快取",
  "better-clouds": "更好的雲",
};

/** 列表上顯示的名稱：有中文名就放在前面。 */
export function displayTitle(hit: ModHit) {
  const chinese = CHINESE_NAMES[hit.slug];
  return chinese ? `${chinese} ${hit.title}` : hit.title;
}

/** Modrinth 只懂英文，打中文名時換成它的 slug 再搜尋。 */
export function searchTerm(query: string) {
  const typed = query.trim();
  if (typed === "") {
    return query;
  }
  const match = Object.entries(CHINESE_NAMES).find(([, name]) => name.includes(typed));
  return match ? match[0].split("-").join(" ") : query;
}
