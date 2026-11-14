import assert from "node:assert/strict";
import test from "node:test";
import { selectedPathOrPrevious } from "./audio-picker.ts";

test("OS picker cancellation preserves a previously entered WAV or Pack path", () => {
  assert.equal(selectedPathOrPrevious("C:\\Audio\\dictation.wav", null), "C:\\Audio\\dictation.wav");
  assert.equal(selectedPathOrPrevious("/home/user/verified-pack", null), "/home/user/verified-pack");
});

test("OS picker accepts a selected absolute path and rejects malformed results", () => {
  assert.equal(selectedPathOrPrevious("old", "/tmp/speech.wav"), "/tmp/speech.wav");
  assert.equal(selectedPathOrPrevious("old", "C:\\Audio\\pack"), "C:\\Audio\\pack");
  assert.equal(selectedPathOrPrevious("old", []), "old");
  assert.equal(selectedPathOrPrevious("old", " "), "old");
});
