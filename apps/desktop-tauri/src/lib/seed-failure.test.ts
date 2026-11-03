import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { RECOVERED_SEED_NOTICE, SEED_REASONS, parseSeedReason, recoveredInterruptedSeed, seedFailureGuidance } from "./seed-failure.ts";

const RETRY = ["workspace_busy", "writer_active", "workspace_io_failure"] as const;

test("only allowlisted reason strings are accepted", () => {
  assert.equal(parseSeedReason("workspace_busy"), "workspace_busy");
  assert.equal(parseSeedReason("C:\\Users\\someone\\vault"), undefined);
  assert.equal(parseSeedReason(42), undefined);
  assert.equal(parseSeedReason(undefined), undefined);
});

test("busy, competing writer and read failures are never described as damage", () => {
  for (const reason of RETRY) {
    const g = seedFailureGuidance(reason);
    assert.equal(g?.category, "retry", reason);
    assert.doesNotMatch(`${g?.title} ${g?.detail}`, /corrupt|damag|decod|not finished|integrity/i, reason);
  }
});

test("a missing or incomplete setup record is not described as undecodable", () => {
  for (const reason of ["seed_manifest_missing", "seed_manifest_incomplete", "seed_vault_missing", "core_ledger_incomplete"] as const) {
    const g = seedFailureGuidance(reason)!;
    assert.equal(g.category, "incomplete", reason);
    assert.doesNotMatch(`${g.title} ${g.detail}`, /decod|corrupt/i, reason);
    assert.match(g.detail, /another MedScale window may still be preparing it/);
  }
});

test("core integrity errors make no claim about earlier writes", () => {
  const g = seedFailureGuidance("core_integrity_error")!;
  assert.equal(g.category, "stopped");
  assert.doesNotMatch(g.detail, /nothing has been changed|deleted/i);
});

test("every reason has guidance; unknown or absent reasons fall back to the raw message", () => {
  for (const reason of SEED_REASONS) assert.ok(seedFailureGuidance(reason), reason);
  assert.equal(seedFailureGuidance(undefined), null);
});

test("reason codes match the Rust seed_reason constants exactly", () => {
  const host = fs.readFileSync(new URL("../../src-tauri/src/host.rs", import.meta.url), "utf8");
  const block = host.slice(host.indexOf("pub mod seed_reason {"), host.indexOf("\n}\n", host.indexOf("pub mod seed_reason {")));
  const rust = [...block.matchAll(/pub const [A-Z_]+: &str = "([a-z_]+)";/g)].map((m) => m[1]).sort();
  assert.deepEqual(rust, [...SEED_REASONS].sort());
});

test("the rebuilt notice appears only for an explicit true flag", () => {
  assert.equal(recoveredInterruptedSeed({ recovered_interrupted_seed: true, kind: "synthetic" }), true);
  for (const data of [{ recovered_interrupted_seed: false }, { recovered_interrupted_seed: "true" }, {}, null, undefined, "x"]) {
    assert.equal(recoveredInterruptedSeed(data), false, JSON.stringify(data));
  }
  assert.doesNotMatch(`${RECOVERED_SEED_NOTICE.title} ${RECOVERED_SEED_NOTICE.detail}`, /corrupt|deleted|lost/i);
});
