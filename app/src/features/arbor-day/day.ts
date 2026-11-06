const MARCH = 2;
const ARBOR_DAY = 12;

/** 3 月 12 日：植樹節，也是國父孫中山先生的逝世紀念日。 */
export function isArborDay(date: Date) {
  return date.getMonth() === MARCH && date.getDate() === ARBOR_DAY;
}

/** 進度對應到每片花瓣開了多少：前一片開完，下一片才開始。 */
export function petalOpenings(share: number, petals: number) {
  const progress = Math.min(Math.max(share, 0), 1) * petals;
  return Array.from({ length: petals }, (_, index) => Math.min(Math.max(progress - index, 0), 1));
}

/** 中山主題只在植樹節當天、種下今年的樹之後出現，隔天自動恢復。 */
export function wearsZhongshan(plantedYears: number[], date: Date) {
  return isArborDay(date) && plantedYears.includes(date.getFullYear());
}
