import assert from "node:assert/strict";
import test from "node:test";
import { parseShellStatus } from "./ipc.ts";

const idle = { schemaVersion: 2, coreConnection: "idle", detail: "No workspace is open.", syntheticOnly: true };

test("shell response preserves the versioned native protocol", () => {
  assert.deepEqual(parseShellStatus(idle), idle);
  const connected = { ...idle, coreConnection: "connected", syntheticOnly: false };
  assert.deepEqual(parseShellStatus(connected), connected);
});

test("mismatched or malformed responses are rejected, never assumed", () => {
  for (const invalid of [null, [], "ready", {}, { ...idle, schemaVersion: 1 },
    { ...idle, coreConnection: "unavailable" }, { ...idle, syntheticOnly: "yes" },
    { ...idle, detail: "" }, { ...idle, detail: "x".repeat(257) }]) {
    assert.throws(() => parseShellStatus(invalid));
  }
});
