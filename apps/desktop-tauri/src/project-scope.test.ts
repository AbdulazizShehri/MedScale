import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

// Structural regression guard: React unmounts children when a keyed Fragment's
// key changes. Without this boundary, Audio's transcript, draft and form state
// can remain visible after selecting another project, independently of IPC.
const source = readFileSync(new URL("./components/project.tsx", import.meta.url), "utf8");

test("project-scoped children are keyed by the selected project's exact ID", () => {
  assert.match(source, /\{current\s*&&\s*<Fragment\s+key=\{current\.id\}>\{children\(current\.id,\s*current\)\}<\/Fragment>\}/);
  assert.doesNotMatch(source, /\{current\s*&&\s*children\(current\.id,\s*current\)\}/);
});

test("ProjectScope keys the subtree rather than only its page header", () => {
  assert.match(source, /<Fragment\s+key=\{current\.id\}>\{children\(current\.id,\s*current\)\}<\/Fragment>/);
});
