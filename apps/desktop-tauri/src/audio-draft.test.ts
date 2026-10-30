import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { visibleRevisionDraft } from "./audio-draft.ts";

test("only a note for the exact transcript revision can be displayed", () => {
  const note = { note: "proposal for revision A" };
  const snapshot = { revisionId: "revision-A", value: note };
  assert.equal(visibleRevisionDraft("revision-A", snapshot), note);
  assert.equal(visibleRevisionDraft("revision-B", snapshot), null);
  assert.equal(visibleRevisionDraft(null, snapshot), null);
});

test("late response for an older revision remains hidden after correction", () => {
  const oldRequestResult = { revisionId: "revision-A", value: { note: "late old note" } };
  assert.equal(visibleRevisionDraft("revision-B", oldRequestResult), null);
});

test("no active revision or draft fails closed", () => {
  assert.equal(visibleRevisionDraft(null, null), null);
  assert.equal(visibleRevisionDraft("revision-A", null), null);
  assert.equal(visibleRevisionDraft("", { revisionId: "", value: { note: "empty" } }), null);
});

test("AudioFlow renders only the revision-gated draft instead of raw state", () => {
  const source = readFileSync(new URL("./pages/Operations.tsx", import.meta.url), "utf8");
  assert.match(source, /const visibleDraft = visibleRevisionDraft\(result\?\.transcript\?\.revision_id \?\? null, draft\)/);
  assert.match(source, /\{visibleDraft && <div/);
  assert.match(source, /setDraft\(\{ revisionId, value: r\.data \}\)/);
  assert.doesNotMatch(source, /\{draft && <div/);
});
