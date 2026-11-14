/** Prevent a note generated for one transcript revision from appearing as a
 * note for a different revision, including after an asynchronous response. */
export type RevisionBoundDraft<T> = { revisionId: string; value: T };

export function visibleRevisionDraft<T>(
  currentRevisionId: string | null,
  snapshot: RevisionBoundDraft<T> | null,
): T | null {
  return currentRevisionId && snapshot?.revisionId === currentRevisionId ? snapshot.value : null;
}
