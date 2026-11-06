import assert from "node:assert/strict";
import { test } from "node:test";
import { DEFAULT_CLOSED, navGroupOpen } from "./nav-groups.ts";

test("clinical and AI work start open, operations and governance start folded", () => {
  for (const area of ["Clinical", "Intelligence", "Research"]) assert.equal(navGroupOpen(area, DEFAULT_CLOSED), true);
  for (const area of ["Operations", "Governance"]) assert.equal(navGroupOpen(area, DEFAULT_CLOSED), false);
});
