import { test } from "node:test";
import assert from "node:assert/strict";
import { AUTO_LOCK_MINUTES, shouldAutoLock } from "./auto-lock.ts";

test("an idle encrypted vault locks after the timeout, and not before", () => {
  const start = 1_000_000;
  const limit = AUTO_LOCK_MINUTES * 60_000;
  assert.equal(shouldAutoLock(start, start), false);
  assert.equal(shouldAutoLock(start, start + limit - 1), false);
  assert.equal(shouldAutoLock(start, start + limit), true);
  assert.equal(shouldAutoLock(start, start + 2 * limit), true);
});

test("bad input never locks by surprise", () => {
  assert.equal(shouldAutoLock(Number.NaN, 5), false);
  assert.equal(shouldAutoLock(0, Number.POSITIVE_INFINITY), false);
  assert.equal(shouldAutoLock(0, 10 ** 12, 0), false, "a zero timeout means off");
});
