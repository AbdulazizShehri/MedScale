import assert from "node:assert/strict";
import test from "node:test";
import { mayStartModelDownload } from "./model-consent.ts";

test("model download requires consent, matching disk check, and an empty model cache", () => {
  assert.equal(mayStartModelDownload(false, "whisper-tiny-en", "whisper-tiny-en", true, false, false), false);
  assert.equal(mayStartModelDownload(true, "whisper-tiny-en", null, true, false, false), false);
  assert.equal(mayStartModelDownload(true, "whisper-tiny-en", "whisper-base-en", true, false, false), false);
  assert.equal(mayStartModelDownload(true, "whisper-tiny-en", "whisper-tiny-en", false, false, false), false);
  assert.equal(mayStartModelDownload(true, "whisper-tiny-en", "whisper-tiny-en", true, true, false), false);
  assert.equal(mayStartModelDownload(true, "whisper-tiny-en", "whisper-tiny-en", true, false, true), false);
  assert.equal(mayStartModelDownload(true, "whisper-tiny-en", "whisper-tiny-en", true, false, false), true);
});
