import assert from "node:assert/strict";
import test from "node:test";
import { parseShellStatus } from "./ipc.ts";
import { readFileSync } from "node:fs";

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


test("confirmed audio removal is registered and explicitly permissioned end to end", () => {
  // Build-time Tauri ACL is separate from Rust's generate_handler registry:
  // all three must agree or IPC is rejected before it reaches Core.
  const read = (relative: string) => readFileSync(new URL(relative, import.meta.url), "utf8");
  const commands = read("../src-tauri/build.rs");
  const handler = read("../src-tauri/src/main.rs");
  const permissions = JSON.parse(read("../src-tauri/capabilities/default.json")).permissions;
  assert.match(commands, /"audio_source_remove_confirmed"/);
  assert.match(handler, /cmd_ops::audio_source_remove_confirmed/);
  assert.ok(permissions.includes("allow-audio-source-remove-confirmed"));
});
