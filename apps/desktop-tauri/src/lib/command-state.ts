/** A command response belongs only to the exact command and arguments that produced it. */
export type CommandLoadable<T, E> =
  | { status: "loading" }
  | { status: "ready"; data: T }
  | { status: "error"; error: E };

export type KeyedLoadable<T, E> = {
  key: string | null;
  value: CommandLoadable<T, E>;
};

/** Never show a previous project's data when the query changes. Same-key
 * reloads may keep their last successful value until a new response arrives. */
export function visibleCommandState<T, E>(
  currentKey: string | null,
  snapshot: KeyedLoadable<T, E>,
): CommandLoadable<T, E> {
  return currentKey !== null && snapshot.key === currentKey
    ? snapshot.value
    : { status: "loading" };
}
