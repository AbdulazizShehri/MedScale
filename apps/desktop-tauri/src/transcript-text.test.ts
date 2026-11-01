import { test } from "node:test";
import assert from "node:assert/strict";
import { transcriptPlainText } from "./transcript-text.ts";

test("orders segments, labels draft status and keeps untranscribed gaps", () => {
  const text = transcriptPlainText({
    revision_no: 2,
    state: "partial",
    segments: [
      { seq: 2, start_ms: 61_500, end_ms: 65_000, text: null },
      { seq: 1, start_ms: 0, end_ms: 4_200, text: "  Synthetic opening line. ", speaker: { kind: "anonymous", index: 1 } },
    ],
  });
  assert.equal(
    text,
    "MedScale draft transcript · revision 2 · partial · not clinically verified\n\n" +
      "[00:00–00:04 speaker 1] Synthetic opening line.\n" +
      "[01:01–01:05] (untranscribed)\n",
  );
});

test("empty revision still states provenance", () => {
  assert.match(transcriptPlainText({ revision_no: 1, state: "partial", segments: [] }), /^MedScale draft transcript · revision 1/);
});
