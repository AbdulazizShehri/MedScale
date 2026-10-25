/** A cancelled OS picker must not discard a manually entered path. */
export function selectedPathOrPrevious(previous: string, selected: string | string[] | null): string {
  return typeof selected === "string" && selected.trim() !== "" ? selected : previous;
}
