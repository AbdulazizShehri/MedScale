/** Explicit approval is for the exact pinned model and the checked disk budget. */
export function mayStartModelDownload(
  approved: boolean,
  selectedId: string,
  checkedId: string | null,
  enoughSpace: boolean,
  busy: boolean,
  cacheEntryExists: boolean,
): boolean {
  return approved && !!selectedId && checkedId === selectedId && enoughSpace &&
    !busy && !cacheEntryExists;
}
