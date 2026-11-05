import type { Feature } from "../shell/feature";
import { about } from "./about";
import { appearance } from "./appearance";
import { arborDay } from "./arbor-day";
import { crash } from "./crash";
import { download } from "./download";
import { instances } from "./instances";
import { launch } from "./launch";
import { mods } from "./mods";
import { offlineAccount } from "./offline-account";
import { settings } from "./settings";
import { update } from "./update";

/** 所有內建功能模塊。分頁和設定區塊按這裡的順序排列。 */
export const FEATURES: Feature[] = [
  appearance,
  launch,
  instances,
  mods,
  offlineAccount,
  download,
  crash,
  update,
  settings,
  about,
  arborDay,
];
