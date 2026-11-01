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

/** Option label without repeating segments already present in the model name
 * ("Whisper Tiny · English" + "English" → "Whisper Tiny · English"). */
export function modelOptionLabel(name: string, languages: string): string {
  const parts: string[] = [];
  for (const part of `${name} · ${languages}`.split(" · ").map((p) => p.trim()).filter(Boolean)) {
    if (!parts.includes(part)) parts.push(part);
  }
  return parts.join(" · ");
}
