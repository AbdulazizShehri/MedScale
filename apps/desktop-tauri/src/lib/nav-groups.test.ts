import assert from "node:assert/strict";
import { test } from "node:test";
import { navGroupOpen, parseClosedGroups, serializeClosedGroups } from "./nav-groups.ts";

test("clinical and AI work start open, operations and governance start folded", () => {
  for (const area of ["Clinical", "Intelligence", "Research"]) assert.equal(navGroupOpen(area, null), true);
  for (const area of ["Operations", "Governance"]) assert.equal(navGroupOpen(area, null), false);
});

test("a stored choice round-trips", () => {
  const closed = parseClosedGroups(serializeClosedGroups(new Set(["Clinical"])));
  assert.equal(navGroupOpen("Clinical", closed), false);
  assert.equal(navGroupOpen("Governance", closed), true);
});

test("an unreadable preference falls back to the default and drops unknown names", () => {
  for (const raw of [null, "not json", "{}", "42"]) assert.equal(parseClosedGroups(raw), null);
  assert.deepEqual([...parseClosedGroups('["Clinical", "<script>", 3]')!], ["Clinical"]);
});
