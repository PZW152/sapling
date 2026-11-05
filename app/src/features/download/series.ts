/** 把版本按系列分組，保持原來的順序。 */
export function groupBySeries(versions: string[]) {
  const groups: { series: string; versions: string[] }[] = [];
  for (const id of versions) {
    const series = seriesOf(id);
    const last = groups[groups.length - 1];
    if (last?.series === series) {
      last.versions.push(id);
    } else {
      groups.push({ series, versions: [id] });
    }
  }
  return groups;
}

/** `1.21.4` 屬於 1.21，`26.1.2` 屬於 26。 */
function seriesOf(id: string) {
  const parts = id.split(".");
  return parts[0] === "1" ? `${parts[0]}.${parts[1]}` : parts[0];
}
