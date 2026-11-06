/** Which sidebar groups are folded. Only group names are stored, nothing about the
 * workspace, and an unreadable preference falls back to the default. */
const DEFAULT_CLOSED: ReadonlySet<string> = new Set(["Operations", "Governance"]);
const KNOWN: ReadonlySet<string> = new Set(["Clinical", "Intelligence", "Research", "Operations", "Governance"]);

export function parseClosedGroups(raw: string | null): ReadonlySet<string> | null {
  if (raw === null) return null;
  try {
    const value: unknown = JSON.parse(raw);
    if (!Array.isArray(value)) return null;
    return new Set(value.filter((v): v is string => typeof v === "string" && KNOWN.has(v)));
  } catch {
    return null;
  }
}

export function serializeClosedGroups(closed: ReadonlySet<string>): string {
  return JSON.stringify([...closed]);
}

export function navGroupOpen(area: string, closed: ReadonlySet<string> | null): boolean {
  return !(closed ?? DEFAULT_CLOSED).has(area);
}
