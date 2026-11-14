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

  // Parse each entire registry entry instead of harvesting only recognized
  // paths. Otherwise a new unqualified handler, unknown syntax or module is
  // silently invisible, leaving the allowlist equality check falsely green.
  const splitEntries = (block: string): string[] =>
    block.trim().replace(/,\s*$/, "").split(",").map((entry) => entry.trim());
  const readNativeNames = (block: string): string[] =>
    splitEntries(block).map((entry) => {
      const match = entry.match(/^(?:[a-z_][a-z_0-9]*::)*([a-z_][a-z_0-9]*)$/);
      assert.ok(match, `unrecognized Tauri native handler entry: ${entry}`);
      return match[1]!;
    });
  const readBuildNames = (block: string): string[] =>
    splitEntries(block).map((entry) => {
      const match = entry.match(/^"([a-z_][a-z_0-9]*)"$/);
      assert.ok(match, `unrecognized Tauri build manifest entry: ${entry}`);
      return match[1]!;
    });
  assert.deepEqual(
    readNativeNames("cmd_core::status, future_module::not_granted, crate::nested::handler, standalone,"),
    ["status", "not_granted", "handler", "standalone"],
    "registry parser must include future modules, nested paths and bare handlers",
  );
  const invoked = readNativeNames(handlerBlock);
  const declared = readBuildNames(manifestBlock);
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

test("project-scoped React subtree remounts when selected project changes", () => {
  // A keyed fragment must wrap *all* project-local view state. Guard against
  // a previous project's transcript/draft remaining visible under a new one.
  const source = readFileSync(new URL("./components/project.tsx", import.meta.url), "utf8");
  assert.match(source, /\{current\s*&&\s*<Fragment\s+key=\{current\.id\}>\{children\(current\.id,\s*current\)\}<\/Fragment>\}/);
  assert.doesNotMatch(source, /\{current\s*&&\s*children\(current\.id,\s*current\)\}/);
});
