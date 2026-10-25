/** Storage display uses binary units and does not misstate estimates as exact. */
export function formatStorage(bytes: number): string {
  if (!Number.isSafeInteger(bytes) || bytes < 0) return "Unknown";
  return `${(bytes / (1024 * 1024)).toFixed(0)} MiB`;
}
