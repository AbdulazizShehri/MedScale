import assert from "node:assert/strict";
import test from "node:test";
import { mayStartModelDownload } from "./model-consent.ts";

test("download cannot start until the user confirms the exact checked model", () => {
  assert.equal(mayStartModelDownload(false, "whisper-tiny-en", "whisper-tiny-en", true, false), false);
  assert.equal(mayStartModelDownload(true, "whisper-tiny-en", null, true, false), false);
  assert.equal(mayStartModelDownload(true, "whisper-tiny-en", "whisper-base-en", true, false), false);
  assert.equal(mayStartModelDownload(true, "whisper-tiny-en", "whisper-tiny-en", false, false), false);
  assert.equal(mayStartModelDownload(true, "whisper-tiny-en", "whisper-tiny-en", true, true), false);
  assert.equal(mayStartModelDownload(true, "whisper-tiny-en", "whisper-tiny-en", true, false), true);
});
