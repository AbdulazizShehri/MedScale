/** Which sidebar groups start folded. A choice lasts until the app closes: the
 * production WebView runs InPrivate and keeps no browser storage across restarts. */
export const DEFAULT_CLOSED: ReadonlySet<string> = new Set(["Operations", "Governance"]);

export function navGroupOpen(area: string, closed: ReadonlySet<string>): boolean {
  return !closed.has(area);
}
