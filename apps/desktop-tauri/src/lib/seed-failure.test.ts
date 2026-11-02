import { test } from "node:test";
import assert from "node:assert/strict";
import { SEED_REASONS, parseSeedReason, seedFailureGuidance } from "./seed-failure.ts";

test("only allowlisted reason strings are accepted", () => {
  assert.equal(parseSeedReason("workspace_busy"), "workspace_busy");
  assert.equal(parseSeedReason("C:\\Users\\someone\\vault"), undefined);
  assert.equal(parseSeedReason(42), undefined);
  assert.equal(parseSeedReason(undefined), undefined);
});

test("busy, competing writer and read failures are never described as damage", () => {
  for (const reason of ["workspace_busy", "writer_active", "workspace_io_failure"] as const) {
    const g = seedFailureGuidance(reason);
    assert.equal(g?.category, "retry", reason);
    assert.doesNotMatch(`${g?.title} ${g?.detail}`, /corrupt|damaged|not finished/i, reason);
  }
});

test("every incomplete or corrupt reason refuses replay without claiming deletion", () => {
  for (const reason of SEED_REASONS.filter((r) => !["workspace_busy", "writer_active", "workspace_io_failure"].includes(r))) {
    const g = seedFailureGuidance(reason);
    assert.equal(g?.category, "incomplete", reason);
    assert.match(g!.detail, /nothing has been changed or deleted/);
  }
});

test("unknown or absent reasons fall back to the raw message only", () => {
  assert.equal(seedFailureGuidance(undefined), null);
});

test("reason codes match the Rust seed_reason constants exactly", async () => {
  const fs = await import("node:fs");
  const host = fs.readFileSync(new URL("../../src-tauri/src/host.rs", import.meta.url), "utf8");
  const rust = [...host.matchAll(/pub const [A-Z_]+: &str = "([a-z_]+)";/g)].map((m) => m[1]).sort();
  assert.deepEqual(rust, [...SEED_REASONS].sort());
});
