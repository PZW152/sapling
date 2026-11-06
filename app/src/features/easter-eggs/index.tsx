import type { ComponentType } from "react";
import type { Feature, HeroPanelProps } from "../../shell/feature";
import type { LauncherConfig } from "../../shell/types";
import { arborDay } from "./arbor-day";

/** 一個彩蛋。新彩蛋放在自己的資料夾，再加進下面的清單。 */
export type EasterEgg = {
  Overlay?: ComponentType;
  HeroPanel?: ComponentType<HeroPanelProps>;
  launchProgress?: (config: LauncherConfig) => ComponentType<{ share: number }> | null;
};

const EGGS: EasterEgg[] = [arborDay];

function Overlays() {
  return EGGS.map(({ Overlay }, index) => Overlay && <Overlay key={index} />);
}

function HeroPanels(props: HeroPanelProps) {
  return EGGS.map(({ HeroPanel }, index) => HeroPanel && <HeroPanel key={index} {...props} />);
}

export const easterEggs: Feature = {
  id: "easter-eggs",
  name: "彩蛋",
  description: "特定日子才會出現的小驚喜",
  optional: true,
  Overlay: Overlays,
  HeroPanel: HeroPanels,
  launchProgress: (config) =>
    EGGS.map((egg) => egg.launchProgress?.(config)).find(Boolean) ?? null,
};
