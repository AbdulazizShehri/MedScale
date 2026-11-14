/** Which sidebar groups start folded. A choice lasts until the workspace locks or
 * the app closes. It isn't saved, so a restart returns to these defaults. */
export const DEFAULT_CLOSED: ReadonlySet<string> = new Set(["Operations", "Governance"]);

export function navGroupOpen(area: string, closed: ReadonlySet<string>): boolean {
  return !closed.has(area);
}
