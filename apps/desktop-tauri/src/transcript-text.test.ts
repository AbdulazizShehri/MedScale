import { test } from "node:test";
import assert from "node:assert/strict";
import { copyTranscriptDraft, transcriptPlainText } from "./transcript-text.ts";

test("orders segments, labels draft status and keeps untranscribed gaps", () => {
  const text = transcriptPlainText({
    revision_no: 2,
    state: "draft",
    segments: [
      { seq: 2, start_ms: 61_500, end_ms: 65_000, text: null },
      { seq: 1, start_ms: 0, end_ms: 4_200, text: "  Synthetic opening line. ", speaker: { kind: "anonymous", index: 1 } },
    ],
  });
  assert.equal(
    text,
    "MedScale draft transcript · revision 2 · draft · not clinically verified\n\n" +
      "[00:00–00:04 speaker 1] Synthetic opening line.\n" +
      "[01:01–01:05] (untranscribed)\n",
  );
});

test("empty revision still states provenance", () => {
  assert.match(transcriptPlainText({ revision_no: 1, state: "draft", segments: [] }), /^MedScale draft transcript · revision 1/);
});

test("explicit clipboard copy writes exactly one provenance-bearing draft, never reads", async () => {
  const revision = {
    revision_no: 3,
    state: "draft",
    segments: [
      { seq: 1, start_ms: 0, end_ms: 2000, text: "Fictional words." },
      { seq: 2, start_ms: 2000, end_ms: 4000, text: null },
    ],
  };
  const writes: string[] = [];
  await copyTranscriptDraft(revision, {
    async writeText(value) { writes.push(value); },
  });
  assert.deepEqual(writes, [transcriptPlainText(revision)]);
  assert.match(writes[0], /not clinically verified/);
  assert.match(writes[0], /\(untranscribed\)/);
});

test("unavailable or denied clipboard fails closed without a retry", async () => {
  const revision = { revision_no: 1, state: "draft", segments: [] };
  await assert.rejects(copyTranscriptDraft(revision, undefined), /Clipboard write unavailable/);
  let attempts = 0;
  await assert.rejects(copyTranscriptDraft(revision, {
    async writeText() {
      attempts++;
      throw new Error("synthetic permission denied");
    },
  }), /synthetic permission denied/);
  assert.equal(attempts, 1);
});
