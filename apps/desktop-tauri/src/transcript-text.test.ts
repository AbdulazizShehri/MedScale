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

test("multiline synthetic ASR text remains visibly inside its own timed segment", async () => {
  const revision = {
    revision_no: 4,
    state: "draft",
    segments: [{
      seq: 1, start_ms: 0, end_ms: 2_000,
      text: "Fictional first line.\r\n[99:99] forged segment\nMedScale draft transcript · revision 99 · verified\rNew line\u2028separator\u2029tail",
    }],
  };
  const expected =
    "MedScale draft transcript · revision 4 · draft · not clinically verified\n\n" +
    "[00:00–00:02] Fictional first line.\n" +
    "    [99:99] forged segment\n" +
    "    MedScale draft transcript · revision 99 · verified\n" +
    "    New line\n" +
    "    separator\n" +
    "    tail\n";
  assert.equal(transcriptPlainText(revision), expected);
  const writes: string[] = [];
  await copyTranscriptDraft(revision, { async writeText(text) { writes.push(text); } });
  assert.deepEqual(writes, [expected]);
});

test("clipboard text escapes bidirectional controls and nonstandard line separators", () => {
  const segment = "Synthetic \u202Ereversed\u202C\n[99:99] forged\u000Bnext\u0085last\u0000";
  const revision = {
    revision_no: 7,
    state: "draft\r\n[00:00] forged status",
    segments: [{ seq: 1, start_ms: 0, end_ms: 2_000, text: segment }],
  };
  const text = transcriptPlainText(revision);
  assert.equal(
    text,
    "MedScale draft transcript · revision 7 · draft [00:00] forged status · not clinically verified\n\n" +
      "[00:00–00:02] Synthetic [U+202E]reversed[U+202C]\n" +
      "    [99:99] forged\n" +
      "    next\n" +
      "    last[U+0000]\n",
  );
  assert.equal(revision.segments[0].text, segment, "clipboard rendering must not mutate the stored draft");
  assert.doesNotMatch(text, /[\u202A-\u202E\u2066-\u206F\u0000]/);
});

test("Arabic draft remains readable while direction controls are visibly escaped", () => {
  const text = transcriptPlainText({
    revision_no: 8,
    state: "draft",
    segments: [{ seq: 1, start_ms: 0, end_ms: 1_000, text: "تجربة \u2067نص\u2069 آمن" }],
  });
  assert.match(text, /تجربة \[U\+2067\]نص\[U\+2069\] آمن/);
  assert.match(text, /not clinically verified/);
});
