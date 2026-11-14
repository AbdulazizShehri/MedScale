/**
 * Per-view action gate. React's pending state updates on a later render,
 * so two event handlers may otherwise begin mutations before buttons disable.
 * The synchronous gate enforces the same single-flight contract immediately.
 */
export function createSingleFlightGate() {
  let occupied = false;
  return {
    tryEnter(): boolean {
      if (occupied) return false;
      occupied = true;
      return true;
    },
    leave(): void { occupied = false; },
    isOccupied(): boolean { return occupied; },
  };
}
