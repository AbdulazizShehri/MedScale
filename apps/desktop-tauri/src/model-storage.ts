/** Storage display uses binary units and does not misstate estimates as exact. */
export function formatStorage(bytes: number): string {
  if (!Number.isSafeInteger(bytes) || bytes < 0) return "Unknown";
  return `${(bytes / (1024 * 1024)).toFixed(0)} MiB`;
}

/** Catalog sizes are decimal megabytes (bytes / 1_000_000 in medscale-pack). */
export function formatDecimalMegabytes(megabytes: number | null | undefined): string {
  if (megabytes == null || !Number.isFinite(megabytes) || megabytes < 0) return "—";
  return megabytes >= 1000 ? `${(megabytes / 1000).toFixed(1)} GB` : `${Math.round(megabytes)} MB`;
}
