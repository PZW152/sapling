/** 字母用的是比數字寬的備用字型。 */
const LETTER_UNITS = 1.2;
const DOT_UNITS = 0.5;

/** 版本號的視覺字數，用來算出放得下的字體大小：小數點很窄，字母比數字寬。 */
export function versionUnits(version: string) {
  let units = 0;
  for (const character of version) {
    units += character === "." ? DOT_UNITS : /\d/.test(character) ? 1 : LETTER_UNITS;
  }
  return Math.max(units, 1);
}
