/** An open encrypted vault locks itself after this long without keyboard or
 * pointer activity, so an unattended or shared computer doesn't leave it open. */
export const AUTO_LOCK_MINUTES = 15;

/** Activity that keeps the vault open. */
export const ACTIVITY_EVENTS = ["pointerdown", "pointermove", "keydown", "wheel", "touchstart"] as const;

export function shouldAutoLock(lastActivity: number, now: number, minutes: number = AUTO_LOCK_MINUTES): boolean {
  if (!Number.isFinite(lastActivity) || !Number.isFinite(now) || minutes <= 0) return false;
  return now - lastActivity >= minutes * 60_000;
}
