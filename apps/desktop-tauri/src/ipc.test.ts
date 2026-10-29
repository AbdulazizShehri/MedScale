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

test("every Tauri native handler has an exactly matching build manifest and main-window ACL", () => {
  // Regression guard for the complete registry, not only a single Audio command.
  // Static source checks complement (but never replace) native IPC/OS tests.
  const read = (relative: string) => readFileSync(new URL(relative, import.meta.url), "utf8");
  const handler = read("../src-tauri/src/main.rs");
  const build = read("../src-tauri/build.rs");
  const capability = JSON.parse(read("../src-tauri/capabilities/default.json"));
  const handlerBlock = handler.match(/tauri::generate_handler!\[([\s\S]*?)\]/)?.[1];
  const manifestBlock = build.match(/AppManifest::new\(\)\.commands\(&\[([\s\S]*?)\]\)/)?.[1];
  assert.ok(handlerBlock, "missing the production Tauri invoke handler");
  assert.ok(manifestBlock, "missing the constrained Tauri build manifest");

  // Match every qualified native handler, including commands added under future
  // Rust modules. A fixed list of current modules silently misses new ones.
  const qualifiedCommand = /\b(?:[a-z_][a-z_0-9]*::)+([a-z_][a-z_0-9]*)\b/g;
  assert.deepEqual(
    [..."cmd_core::status, future_module::not_granted, crate::nested::handler".matchAll(qualifiedCommand)]
      .map((match) => match[1]!),
    ["status", "not_granted", "handler"],
    "registry reader must not ignore future modules or qualified paths",
  );
  const invoked = [...handlerBlock.matchAll(qualifiedCommand)]
    .map((match) => match[1]!);
  const declared = [...manifestBlock.matchAll(/"([a-z_0-9]+)"/g)]
    .map((match) => match[1]!);
  const permissions: string[] = capability.permissions;
  const allowed = permissions.filter((name) => name.startsWith("allow-"))
    .map((name) => name.slice("allow-".length).replaceAll("-", "_"));
  const sorted = (names: string[]) => [...names].sort();

  // Duplicate entries are themselves a review-worthy source/manifest drift.
  for (const [label, names] of [["handler", invoked], ["manifest", declared], ["capability", allowed]] as const) {
    assert.ok(names.length > 0, `empty ${label} registry`);
    assert.equal(new Set(names).size, names.length, `duplicate ${label} entries`);
  }
  assert.deepEqual(sorted(invoked), sorted(declared), "handler and build manifest commands differ");
  assert.deepEqual(sorted(invoked), sorted(allowed), "handler and main-window capabilities differ");
  assert.deepEqual(capability.windows, ["main"], "native IPC permission must be scoped to the main window");
  assert.deepEqual(
    sorted(permissions.filter((name) => !name.startsWith("allow-"))),
    ["dialog:allow-open"],
    "native window may not silently acquire unrelated plugin privileges",
  );
});
